use crate::{config::atomic_write, process};
use anyhow::{Context, Result, bail};
use cairo::{Context as Painter, FontSlant, FontWeight, Format, ImageSurface};
use image::{DynamicImage, GenericImageView};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io::Cursor, path::Path};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Block {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub source: String,
    pub translated: String,
    pub confidence: f64,
    pub overflow: bool,
    /// The source was corrected in the fragment editor.
    #[serde(default)]
    pub corrected: bool,
    /// Text lines in the original, for sizing the translation like the source.
    #[serde(default)]
    pub lines: u32,
    /// Estimated font size of the original text in pixels (0 when unknown).
    #[serde(default)]
    pub em: f64,
}

/// Estimates the font size (em) of a text line from the height of its box.
///
/// A word box spans only the parts of the glyphs that are present: "use" is an
/// x-height tall, "Overview" reaches cap height, "Projects" adds a descender. Using
/// the raw height as the font size made neighbouring menu items come out in
/// different sizes. Ratios are for typical sans-serif UI fonts.
pub fn em_from_height(text: &str, height: f64) -> f64 {
    let wide = |c: char| {
        matches!(c as u32, 0x1100..=0x11FF | 0x2E80..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF)
    };
    if text.chars().any(wide) {
        return height / 0.88;
    }
    let tall = text.chars().any(|c| {
        c.is_uppercase() || c.is_ascii_digit() || "bdfhklt\u{0431}\u{0439}\u{0451}ßÄÖÜäöüàáâèéêìíîòóôùúûçñ".contains(c) || "([{/|".contains(c)
    });
    let deep = text.chars().any(|c| "gjpqyQ\u{0440}\u{0443}\u{0444}\u{0434}\u{0446}\u{0449}([{|,;".contains(c));
    let ratio = match (tall, deep) {
        (true, true) => 0.95,
        (true, false) | (false, true) => 0.74,
        (false, false) => 0.53,
    };
    height / ratio
}

pub fn read(path: &Path) -> Result<DynamicImage> {
    let reader = image::ImageReader::open(path)
        .context("invalidImage")?
        .with_guessed_format()
        .context("invalidImage")?;
    let (w, h) = reader.into_dimensions().context("invalidImage")?;
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) > 32_000_000 {
        bail!("imageTooLarge");
    }
    image::ImageReader::open(path)?
        .with_guessed_format()?
        .decode()
        .context("invalidImage")
}
pub fn save(image: &DynamicImage, path: &Path) -> Result<()> {
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png)?;
    atomic_write(path, &bytes.into_inner())
}
/// Prepares a screenshot for Tesseract and returns it with the factor it was scaled by.
///
/// Grey, light text on dark is inverted, the background is flattened by dividing
/// by a blurred copy — coloured cells and gradients turn white, so faint captions
/// on them survive Tesseract's global threshold — and small images are enlarged,
/// since Tesseract reads best at 25–35 px glyphs.
pub fn prepare(image: &DynamicImage) -> (image::GrayImage, f64) {
    use image::imageops::{self, FilterType};
    let mut gray = image.to_luma8();
    let (w, h) = gray.dimensions();
    let mean = gray.pixels().map(|p| u64::from(p[0])).sum::<u64>() / u64::from((w * h).max(1));
    // Dark themes read better as dark text on light. Local background flattening was
    // tried and dropped: it erased light text on dark buttons and split bold words,
    // while Tesseract's own binarisation already copes with coloured cells.
    if mean < 110 {
        imageops::invert(&mut gray);
    }
    let scale = if w.max(h) < 1600 && u64::from(w) * u64::from(h) * 4 <= 32_000_000 { 2.0 } else { 1.0 };
    if scale > 1.0 {
        gray = imageops::resize(&gray, w * 2, h * 2, FilterType::Triangle);
    }
    (gray, scale)
}

pub async fn recognize(
    image: &DynamicImage,
    work: &Path,
    languages: &str,
    cancel: &CancellationToken,
) -> Result<Vec<Block>> {
    let available = process::run("tesseract", &["--list-langs"], &[], 5, cancel)
        .await
        .context("ocrUnavailable")?;
    let available = String::from_utf8_lossy(&available);
    // Skip languages whose Tesseract data isn't installed instead of failing.
    let installed: Vec<&str> = languages
        .split('+')
        .filter(|lang| available.lines().any(|s| s.trim() == *lang))
        .collect();
    if installed.is_empty() {
        bail!("ocrUnavailable");
    }
    let languages = &installed.join("+");
    let source = image.clone();
    let (prepared, scale) = tokio::task::spawn_blocking(move || prepare(&source)).await?;
    let path = work.join("ocr-input.png");
    let mut bytes = Cursor::new(Vec::new());
    prepared.write_to(&mut bytes, image::ImageFormat::Png)?;
    atomic_write(&path, &bytes.into_inner())?;
    let input = path.to_str().context("invalidImage")?;
    // Sparse text (psm 11) suits screenshots: menus, labels and headings are found
    // one by one. Automatic layout (psm 3) still finds some text sparse mode misses,
    // such as light labels on dark buttons. Both run at once and are merged.
    let sparse_args = [input, "stdout", "-l", languages, "--psm", "11", "tsv"];
    let layout_args = [input, "stdout", "-l", languages, "--psm", "3", "tsv"];
    let (sparse, layout) = tokio::join!(
        process::run("tesseract", &sparse_args, &[], 30, cancel),
        process::run("tesseract", &layout_args, &[], 30, cancel)
    );
    let sparse = sparse.context("ocrFailed")?;
    let layout = layout.unwrap_or_default();
    let mut blocks = parse_tsv_merged(&String::from_utf8_lossy(&sparse), &String::from_utf8_lossy(&layout))?;
    if scale > 1.0 {
        let back = |v: i32| (f64::from(v) / scale).round() as i32;
        for b in &mut blocks {
            let (right, bottom) = (back(b.x + b.width), back(b.y + b.height));
            b.x = back(b.x);
            b.y = back(b.y);
            b.width = (right - b.x).max(1);
            b.height = (bottom - b.y).max(1);
            b.em /= scale;
        }
    }
    Ok(blocks)
}
/// (pass, page, block, paragraph, line) of a word: one text line of one Tesseract pass.
type LineKey = (u8, i32, i32, i32, i32);
/// x, y, width, height, text, confidence of one recognised word.
type Word = (i32, i32, i32, i32, String, f64);
/// A block being built: the block, its word count, its last line's em.
type Pending = (Block, usize, f64);

/// A run of words on one text line, without a column gap inside it.
#[derive(Clone, Debug)]
struct Segment {
    x: i32,
    y: i32,
    right: i32,
    bottom: i32,
    words: Vec<String>,
    confidence: f64,
}
impl Segment {
    fn height(&self) -> i32 {
        self.bottom - self.y
    }
    fn em(&self) -> f64 {
        em_from_height(&self.words.join(" "), f64::from(self.height().max(1)))
    }
}

/// Builds translatable blocks from Tesseract's word TSV.
///
/// Tesseract "paragraphs" happily span table rows and columns, and painting their
/// union box erases whole tables. So words are grouped per line, lines are cut at
/// column-sized gaps, and only cuts that are stacked closely and aligned (one
/// table cell, one wrapped paragraph) are joined again. Runs without letters —
/// numbers, percentages, punctuation — stay untouched in the image.
#[cfg(test)]
pub fn parse_tsv(tsv: &str) -> Result<Vec<Block>> {
    group(words(tsv, 0)?)
}

/// Builds blocks from a primary (sparse) and a secondary (layout) Tesseract pass.
///
/// Secondary words fill places where the primary pass found nothing. Where one
/// word of each pass covers the same spot, the more confident reading wins; when
/// one pass split a word the other read whole, the primary reading stays.
pub fn parse_tsv_merged(primary: &str, secondary: &str) -> Result<Vec<Block>> {
    let mut merged = words(primary, 0)?;
    let extra = words(secondary, 1).unwrap_or_default();
    let intersects = |a: &Word, b: &Word| {
        let w = (a.0 + a.2).min(b.0 + b.2) - a.0.max(b.0);
        let h = (a.1 + a.3).min(b.1 + b.3) - a.1.max(b.1);
        w > 0 && h > 0 && f64::from(w * h) >= 0.3 * f64::from((a.2 * a.3).min(b.2 * b.3))
    };
    let mut additions = Vec::new();
    for (key, word) in &extra {
        let hits: Vec<usize> = merged.iter().enumerate().filter(|(_, (_, m))| intersects(m, word)).map(|(i, _)| i).collect();
        match hits.as_slice() {
            [] => additions.push((*key, word.clone())),
            [only] => {
                let rivals = extra.iter().filter(|(_, e)| intersects(&merged[*only].1, e)).count();
                if rivals == 1 && word.5 > merged[*only].1.5 + 5.0 {
                    merged[*only].1 = word.clone();
                }
            }
            _ => {}
        }
    }
    merged.extend(additions);
    group(merged)
}

/// The recognised words of one Tesseract TSV, keyed by their text line.
fn words(tsv: &str, pass: u8) -> Result<Vec<(LineKey, Word)>> {
    let mut out = Vec::new();
    for line in tsv.lines().skip(1) {
        let v: Vec<_> = line.splitn(12, '\t').collect();
        if v.len() != 12 || v[0] != "5" || v[11].trim().is_empty() {
            continue;
        }
        let number = |i: usize| v[i].parse::<i32>().context("ocrFailed");
        let (x, y, w, h) = (number(6)?, number(7)?, number(8)?, number(9)?);
        if w <= 0 || h <= 0 || x < 0 || y < 0 {
            continue;
        }
        let confidence = v[10].parse::<f64>().unwrap_or(0.0);
        let key = (pass, number(1)?, number(2)?, number(3)?, number(4)?);
        out.push((key, (x, y, w, h, v[11].trim().to_owned(), confidence)));
    }
    Ok(out)
}

fn group(words: Vec<(LineKey, Word)>) -> Result<Vec<Block>> {
    let mut lines: BTreeMap<LineKey, Vec<Word>> = BTreeMap::new();
    for (key, word) in words {
        lines.entry(key).or_default().push(word);
    }

    let mut segments = Vec::new();
    for (_, mut words) in lines {
        words.sort_by_key(|w| w.0);
        let line_height = words.iter().map(|w| w.3).max().unwrap_or(1);
        let mut current: Option<Segment> = None;
        for (x, y, w, h, text, confidence) in words {
            // A small, isolated bullet between navigation links is often read
            // as a star, dash or guillemet. Keep its pixels between the labels.
            if h * 2 < line_height && matches!(text.as_str(), "•" | "·" | "*" | "-" | "»")
                && current.as_ref().is_some_and(|s| x - s.right >= line_height / 4)
            {
                segments.extend(current.take());
                continue;
            }
            let gap = current.as_ref().map(|s| x - s.right);
            if let (Some(s), Some(gap)) = (current.as_mut(), gap)
                && gap <= (line_height * 3 / 2).max(12)
            {
                s.right = s.right.max(x + w);
                s.y = s.y.min(y);
                s.bottom = s.bottom.max(y + h);
                s.words.push(text);
                s.confidence += confidence;
                continue;
            }
            segments.extend(current.take());
            current = Some(Segment {
                x,
                y,
                right: x + w,
                bottom: y + h,
                words: vec![text],
                confidence,
            });
        }
        segments.extend(current);
    }
    segments.retain(|s| s.words.iter().any(|w| w.chars().any(char::is_alphabetic)));
    segments.sort_by_key(|s| (s.y, s.x));

    let mut blocks: Vec<Pending> = Vec::new();
    'segments: for s in segments {
        let h = s.height().max(1);
        let em = s.em();
        for (b, count, last) in blocks.iter_mut().rev() {
            let gap = s.y - (b.y + b.height);
            let overlap = (b.x + b.width).min(s.right) - b.x.max(s.x);
            let aligned = (b.x - s.x).abs() <= h || overlap * 2 >= (s.right - s.x).min(b.width);
            // Lines of one paragraph share a font size; a heading over its description does not.
            let similar = em >= *last / 1.13 && em <= *last * 1.13;
            if gap >= -h / 2 && gap <= h + h / 10 && aligned && similar {
                let right = (b.x + b.width).max(s.right);
                let bottom = (b.y + b.height).max(s.bottom);
                b.x = b.x.min(s.x);
                b.y = b.y.min(s.y);
                b.width = right - b.x;
                b.height = bottom - b.y;
                b.source.push(' ');
                b.source.push_str(&s.words.join(" "));
                b.confidence += s.confidence;
                b.lines += 1;
                *count += s.words.len();
                *last = em;
                continue 'segments;
            }
        }
        let count = s.words.len();
        blocks.push((
            Block {
                x: s.x,
                y: s.y,
                width: s.right - s.x,
                height: h,
                source: s.words.join(" "),
                confidence: s.confidence,
                lines: 1,
                em,
                ..Default::default()
            },
            count,
            em,
        ));
        if blocks.len() > 250 {
            bail!("tooManyBlocks");
        }
    }
    let result: Vec<_> = blocks
        .into_iter()
        .map(|(mut b, n, _)| {
            b.confidence /= n as f64;
            b
        })
        .collect();
    if result.is_empty() {
        bail!("noText");
    }
    Ok(result)
}
fn background(image: &DynamicImage, b: &Block) -> [u8; 3] {
    let mut channels = [Vec::new(), Vec::new(), Vec::new()];
    let mut sample = |x: i32, y: i32| {
        let p = image.get_pixel(
            x.clamp(0, image.width() as i32 - 1) as u32,
            y.clamp(0, image.height() as i32 - 1) as u32,
        );
        for i in 0..3 {
            channels[i].push(p[i]);
        }
    };
    for x in (b.x..b.x + b.width).step_by((b.width / 25).max(1) as usize) {
        sample(x, b.y - 3);
        sample(x, b.y + b.height + 3);
    }
    for y in (b.y..b.y + b.height).step_by((b.height / 25).max(1) as usize) {
        sample(b.x - 3, y);
        sample(b.x + b.width + 3, y);
    }
    channels.map(|mut c| {
        c.sort_unstable();
        c.get(c.len() / 2).copied().unwrap_or(255)
    })
}
fn distance(pixel: image::Rgba<u8>, bg: [u8; 3]) -> usize {
    (0..3).map(|i| pixel[i].abs_diff(bg[i])).max().unwrap() as usize
}

/// Space up to the nearest visible content on either side of a line.
fn margin(image: &DynamicImage, b: &Block, bg: [u8; 3], right: bool) -> i32 {
    let (start, step) = if right {
        (b.x + b.width + 2, 1)
    } else {
        (b.x - 3, -1)
    };
    let mut x = start;
    while (0..image.width() as i32).contains(&x)
        && (b.y..b.y + b.height)
            .step_by((b.height / 24).max(1) as usize)
            .all(|y| distance(image.get_pixel(x as u32, y as u32), bg) < 8)
    {
        x += step;
    }
    (x - start).abs()
}

struct Lettering {
    color: [f64; 3],
    bold: bool,
    glyph_height: f64,
    plain_background: bool,
}

/// Sample the strokes inside the OCR box. A high contrast percentile finds
/// their colour without picking antialiased edges or a single stray bright pixel.
fn lettering(image: &DynamicImage, b: &Block, bg: [u8; 3], em: f64) -> Lettering {
    let mut histogram = [0usize; 256];
    let mut colors = [[0usize; 3]; 256];
    let dx = (b.width / 384).max(1) as usize;
    let dy = (b.height / 120).max(1) as usize;
    let mut rows = Vec::new();
    for y in (b.y..b.y + b.height).step_by(dy) {
        let mut row = [0usize; 256];
        let mut count = 0;
        for x in (b.x..b.x + b.width).step_by(dx) {
            let p = image.get_pixel(x as u32, y as u32);
            let d = distance(p, bg);
            histogram[d] += 1;
            for (i, sum) in colors[d].iter_mut().enumerate() {
                *sum += usize::from(p[i]);
            }
            row[d] += 1;
            count += 1;
        }
        let mut visible = 0;
        let contrast = (0..256)
            .rev()
            .find(|&d| {
                visible += row[d];
                visible * 24 >= count
            })
            .unwrap_or(0);
        rows.push(contrast);
    }
    let samples: usize = histogram.iter().sum();
    let mut tail = 0;
    let peak = (0..256)
        .rev()
        .find(|&d| {
            tail += histogram[d];
            tail * 50 >= samples
        })
        .unwrap_or(0);
    let color = colors[peak].map(|sum| sum as f64 / histogram[peak].max(1) as f64 / 255.0);
    let lines = f64::from(b.lines.max(1));
    let ink: usize = histogram[(peak / 2).max(1)..].iter().sum();
    let density = ink as f64 / samples.max(1) as f64 * f64::from(b.height) / (lines * em);
    Lettering {
        color,
        bold: density >= 0.265,
        glyph_height: (rows.iter().filter(|r| **r >= (peak / 3).max(12)).count() * dy) as f64
            / lines,
        plain_background: histogram[..12].iter().sum::<usize>() * 4 >= samples,
    }
}

/// OCR can include the underline in a link's bounds or miss it by a few pixels.
fn underline(image: &DynamicImage, b: &Block, bg: [u8; 3]) -> Option<i32> {
    if b.lines > 1 || b.width < 8 {
        return None;
    }
    let bottom = b.y + b.height;
    (bottom - 2..=(bottom + 4).min(image.height() as i32 - 1))
        .filter(|&y| y >= 0)
        .find(|&y| {
            let ink = |x: i32| {
                (0..image.width() as i32).contains(&x)
                    && distance(image.get_pixel(x as u32, y as u32), bg) > 32
            };
            !ink(b.x - 3)
                && !ink(b.x + b.width + 3)
                && (b.x..b.x + b.width).filter(|&x| ink(x)).count() * 10 >= b.width as usize * 9
        })
}

/// A row of endonyms is a language picker. Some of its scripts may not be
/// installed in Tesseract, so preserve the whole row, including unreadable names.
fn language_menu(blocks: &[Block]) -> Vec<bool> {
    let short = |b: &Block| b.lines == 1 && b.source.split_whitespace().count() <= 3;
    let is_name = |b: &Block| crate::languages::LANGUAGES.iter().any(|l| l.3 == b.source);
    let mut order: Vec<_> = (0..blocks.len()).filter(|&i| short(&blocks[i])).collect();
    order.sort_by_key(|&i| blocks[i].x);
    let mut menu = vec![false; blocks.len()];
    for (i, anchor) in blocks.iter().enumerate() {
        if !is_name(anchor) || menu[i] {
            continue;
        }
        let row: Vec<_> = order
            .iter()
            .copied()
            .filter(|&j| (blocks[j].y - anchor.y).abs() <= anchor.height / 2)
            .collect();
        // Allow for a short name in a script omitted entirely by OCR.
        for labels in row
            .chunk_by(|&a, &b| blocks[b].x - (blocks[a].x + blocks[a].width) <= 6 * anchor.height)
        {
            if labels.contains(&i) && labels.iter().filter(|&&j| is_name(&blocks[j])).count() >= 2 {
                for &j in labels {
                    menu[j] = true;
                }
            }
        }
    }
    menu
}

/// Use at most two additional lines of empty background below a paragraph.
fn paragraph_room(image: &DynamicImage, b: &Block, bg: [u8; 3], em: f64) -> f64 {
    if b.lines <= 1 {
        return 0.0;
    }
    let bottom = b.y + b.height;
    let end = image.height().min((bottom + (3.0 * em) as i32) as u32) as i32;
    let rows = (bottom + 2..end)
        .take_while(|&y| {
            (b.x..b.x + b.width)
                .all(|x| distance(image.get_pixel(x as u32, y as u32), bg) < 8)
        })
        .count();
    (rows as f64 - em * 0.35).max(0.0)
}

fn wrap(ctx: &Painter, text: &str, width: f64) -> Result<Option<Vec<String>>> {
    let mut lines = vec![String::new()];
    for word in text.split_whitespace() {
        if ctx.text_extents(word)?.x_advance() > width {
            return Ok(None);
        }
        let last = lines.last_mut().unwrap();
        let candidate = if last.is_empty() {
            word.to_owned()
        } else {
            format!("{last} {word}")
        };
        if ctx.text_extents(&candidate)?.x_advance() > width && !last.is_empty() {
            lines.push(word.to_owned());
        } else {
            *last = candidate;
        }
    }
    Ok(Some(lines))
}
pub fn render(original: &DynamicImage, blocks: &mut [Block], minimum: u32) -> Result<DynamicImage> {
    let (w, h) = original.dimensions();
    let stride = Format::ARgb32.stride_for_width(w)?;
    let mut pixels = vec![0u8; stride as usize * h as usize];
    for (x, y, p) in original.to_rgba8().enumerate_pixels() {
        let i = y as usize * stride as usize + x as usize * 4;
        let a = u32::from(p[3]);
        for channel in 0..3 {
            pixels[i + channel] = ((u32::from(p[2 - channel]) * a + 255 * (255 - a)) / 255) as u8;
        }
        pixels[i + 3] = 255;
    }
    let mut surface =
        ImageSurface::create_for_data(pixels, Format::ARgb32, w as i32, h as i32, stride)?;
    {
        let ctx = Painter::new(&surface)?;
        // Tesseract sometimes boxes a word far taller than its letters; the median
        // font size of the image keeps one such block from getting a huge font.
        let em_of = |b: &Block| {
            if b.em > 0.0 {
                b.em
            } else {
                f64::from(b.height) / f64::from(b.lines.max(1))
            }
        };
        let mut sizes: Vec<f64> = blocks.iter().map(em_of).collect();
        sizes.sort_by(f64::total_cmp);
        let typical = sizes.get(sizes.len() / 2).copied().unwrap_or(16.0);
        // Repeated left edges in a table or list take precedence over apparently
        // equal margins around an individual label.
        let left_aligned: Vec<_> = blocks.iter().map(|b| blocks.iter().any(|other| {
            let dy = (b.y - other.y).abs();
            (b.x - other.x).abs() <= 3 && (b.width - other.width).abs() > 8
                && dy >= b.height && f64::from(dy) <= em_of(b) * 8.0
        })).collect();
        let menus = language_menu(blocks);
        for ((b, menu), left_aligned) in blocks.iter_mut().zip(menus).zip(left_aligned) {
            b.overflow = false;
            if b.translated.is_empty()
                || (!b.corrected && (b.translated == b.source || menu || b.confidence < 80.0))
            {
                continue;
            }
            if b.width <= 0
                || b.height <= 0
                || b.x < 0
                || b.y < 0
                || b.x + b.width > w as i32
                || b.y + b.height > h as i32
            {
                continue;
            }
            let bg = background(original, b);
            let [r, g, blue] = bg.map(|v| f64::from(v) / 255.0);
            let em = em_of(b).max(1.0);
            let style = lettering(original, b, bg, em);
            // Tiny lettering, decorated badges and photos cannot be faithfully
            // replaced with a rectangle. Their translations stay in the sidebar.
            if !style.plain_background || style.glyph_height < 8.0 {
                continue;
            }
            let rule = underline(original, b, bg);
            let left = margin(original, b, bg, false);
            let right = margin(original, b, bg, true);
            let centered = !left_aligned && b.lines <= 1
                && left.min(right) as f64 > em * 2.0
                && (left - right).abs() as f64 <= em;
            let extra = if b.lines > 1 {
                0
            } else if centered {
                2 * (left.min(right) - 8).max(0)
            } else {
                (right - 8).max(0)
            };
            let x = f64::from(b.x);
            let y = f64::from(b.y);
            let width = f64::from(b.width);
            let height = f64::from(b.height);
            let room = width + f64::from(extra);
            let below = paragraph_room(original, b, bg, em);

            ctx.select_font_face(
                "sans-serif",
                FontSlant::Normal,
                if style.bold {
                    FontWeight::Bold
                } else {
                    FontWeight::Normal
                },
            );
            let largest = em.min(typical * 2.5).clamp(7.0, 72.0).round() as u32;
            let smallest = minimum.min(em as u32).max(7).min(largest);
            // Preserve the source baseline spacing in wrapped paragraphs.
            let pitch = if b.lines > 1 {
                (height - style.glyph_height) / f64::from(b.lines - 1)
            } else {
                0.0
            };
            let mut fitted = None;
            for size in (smallest..=largest).rev() {
                ctx.set_font_size(f64::from(size));
                if let Some(lines) = wrap(&ctx, &b.translated, room)? {
                    if b.lines <= 1 && lines.len() > 1 {
                        continue;
                    }
                    let extent = ctx.font_extents()?;
                    let advance = extent.height().max(pitch * f64::from(size) / em);
                    let baseline = ctx.text_extents(&lines[0])?;
                    let last = ctx.text_extents(lines.last().unwrap())?;
                    let needed = -baseline.y_bearing()
                        + (lines.len() - 1) as f64 * advance
                        + last.y_bearing()
                        + last.height();
                    if needed <= height + below + 2.0 {
                        fitted = Some((lines, advance, baseline.y_bearing(), needed));
                        break;
                    }
                }
            }
            b.overflow = fitted.is_none();
            if let Some((lines, advance, bearing, needed)) = fitted {
                ctx.set_source_rgb(r, g, blue);
                // Erase just the source label. Extra horizontal space used by
                // the translation was already checked for neighbouring content.
                ctx.rectangle(x - 1.0, y - 1.0, width + 2.0, height + 2.0);
                ctx.fill()?;
                if let Some(row) = rule {
                    ctx.rectangle(x - 1.0, f64::from(row), width + 2.0, 1.0);
                    ctx.fill()?;
                }
                let [red, green, blue] = style.color;
                ctx.set_source_rgb(red, green, blue);
                let top = y + ((height - needed) / 2.0).max(-1.0) - bearing;
                for (i, line) in lines.iter().enumerate() {
                    let used = ctx.text_extents(line)?.x_advance();
                    let start = if centered {
                        x + (width - used) / 2.0
                    } else {
                        x
                    };
                    ctx.move_to(start, top + i as f64 * advance);
                    ctx.show_text(line)?;
                    if let Some(row) = rule {
                        ctx.rectangle(start, f64::from(row), used, 1.0);
                        ctx.fill()?;
                    }
                }
            }
        }
    }
    surface.flush();
    let bytes = surface
        .data()
        .map_err(|_| anyhow::anyhow!("renderFailed"))?;
    let output = image::RgbaImage::from_fn(w, h, |x, y| {
        let i = y as usize * stride as usize + x as usize * 4;
        image::Rgba([bytes[i + 2], bytes[i + 1], bytes[i], 255])
    });
    Ok(DynamicImage::ImageRgba8(output))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tsv_groups_paragraphs_and_bounds() {
        let b=parse_tsv("header\n5\t1\t1\t1\t1\t1\t10\t20\t30\t10\t95\tHello\n5\t1\t1\t1\t2\t1\t15\t40\t40\t10\t85\tworld").unwrap();
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].source, "Hello world");
        assert_eq!(b[0].height, 30);
        assert_eq!(b[0].confidence, 90.0);
    }
    #[allow(clippy::too_many_arguments)]
    fn word(block: i32, par: i32, line: i32, x: i32, y: i32, w: i32, h: i32, text: &str) -> String {
        format!("5\t1\t{block}\t{par}\t{line}\t1\t{x}\t{y}\t{w}\t{h}\t90\t{text}\n")
    }
    #[test]
    fn table_rows_and_columns_stay_apart() {
        // One Tesseract paragraph covering a two-row, three-column table.
        let mut tsv = String::from("header\n");
        tsv += &word(1, 1, 1, 10, 10, 60, 12, "Agentic");
        tsv += &word(1, 1, 1, 75, 10, 50, 12, "coding");
        tsv += &word(1, 1, 1, 300, 10, 40, 12, "49.1%");
        tsv += &word(1, 1, 1, 500, 10, 60, 12, "Medium");
        tsv += &word(1, 1, 2, 10, 26, 90, 10, "FrontierCode");
        tsv += &word(1, 1, 3, 10, 70, 70, 12, "Robotics");
        let b = parse_tsv(&tsv).unwrap();
        let sources: Vec<_> = b.iter().map(|b| b.source.as_str()).collect();
        // The cell's two lines join; the next row and the far column do not; numbers are skipped.
        assert_eq!(sources, ["Agentic coding FrontierCode", "Medium", "Robotics"]);
        assert_eq!(b[0].lines, 2);
        assert!(b[0].width < 200, "a cell must not stretch across columns: {}", b[0].width);
    }
    #[test]
    fn navigation_bullets_separate_links_and_language_menus() {
        let mut tsv = String::from("header\n");
        for (x, y, width, height, label) in [
            (10, 10, 55, 14, "English"),
            (72, 16, 3, 3, "»"),
            (85, 10, 48, 14, "EAEPX"),
            (140, 16, 3, 3, "-"),
            (152, 10, 60, 14, "Deutsch"),
            (10, 65, 74, 14, "Features"),
        ] {
            tsv += &word(1, 1, y / 30, x, y, width, height, label);
        }
        let blocks = parse_tsv(&tsv).unwrap();
        assert_eq!(blocks.iter().map(|b| b.source.as_str()).collect::<Vec<_>>(),
                   ["English", "EAEPX", "Deutsch", "Features"]);
        assert_eq!(language_menu(&blocks), [true, true, true, false]);
    }

    #[test]
    fn prepare_inverts_dark_and_enlarges_small() {
        // Light text on a dark button stays readable: no local flattening wipes it out.
        let mut light = image::RgbImage::from_pixel(400, 200, image::Rgb([246, 245, 241]));
        for x in 100..300 {
            for y in 80..120 {
                light.put_pixel(x, y, image::Rgb([47, 111, 94]));
            }
        }
        for x in 150..160 {
            light.put_pixel(x, 100, image::Rgb([255, 255, 255]));
        }
        let (prepared, scale) = prepare(&DynamicImage::ImageRgb8(light));
        assert_eq!(scale, 2.0);
        assert_eq!(prepared.dimensions(), (800, 400));
        let button = prepared.get_pixel(260, 200)[0];
        let label = prepared.get_pixel(310, 200)[0];
        assert!(label > button + 100, "button text must contrast with the button: {label} vs {button}");
        // Light text on a dark theme is inverted to dark on light.
        let mut dark = image::RgbImage::from_pixel(2000, 100, image::Rgb([20, 22, 24]));
        dark.put_pixel(1000, 50, image::Rgb([230, 230, 230]));
        let (prepared, scale) = prepare(&DynamicImage::ImageRgb8(dark));
        assert_eq!(scale, 1.0);
        assert!(prepared.get_pixel(10, 10)[0] > 200);
        assert!(prepared.get_pixel(1000, 50)[0] < 100);
    }
    #[test]
    fn em_accounts_for_ascenders_and_descenders() {
        // Same 20 px font: boxes differ by which glyph parts are present.
        let overview = em_from_height("Overview", 20.0 * 0.74);
        let projects = em_from_height("Projects", 20.0 * 0.95);
        let some = em_from_height("some", 20.0 * 0.53);
        for em in [overview, projects, some] {
            assert!((em - 20.0).abs() < 0.5, "{em}");
        }
        assert!((em_from_height("設定", 22.0) - 25.0).abs() < 0.1);
    }
    #[test]
    fn heading_and_description_stay_apart() {
        // A 24 px bold title right above a 19 px description in one Tesseract paragraph.
        let mut tsv = String::from("header\n");
        tsv += &word(1, 1, 1, 320, 196, 130, 23, "Weekly");
        tsv += &word(1, 1, 1, 460, 200, 110, 19, "summary");
        tsv += &word(1, 1, 2, 320, 232, 60, 15, "Send");
        tsv += &word(1, 1, 2, 390, 232, 80, 18, "report");
        let b = parse_tsv(&tsv).unwrap();
        let sources: Vec<_> = b.iter().map(|b| b.source.as_str()).collect();
        assert_eq!(sources, ["Weekly summary", "Send report"]);
        assert!(b[0].em > b[1].em * 1.15, "{} vs {}", b[0].em, b[1].em);
    }
    #[test]
    fn render_paints_only_what_the_translation_needs() {
        // Light content with a slightly darker sidebar on the left; a short label in the sidebar.
        let mut img = image::RgbImage::from_pixel(400, 60, image::Rgb([246, 245, 241]));
        for x in 0..130 {
            for y in 0..60 {
                img.put_pixel(x, y, image::Rgb([236, 235, 229]));
            }
        }
        for y in 20..34 {
            for x in [25, 33, 40, 46, 51] {
                img.put_pixel(x, y, image::Rgb([41; 3]));
            }
        }
        let original = DynamicImage::ImageRgb8(img);
        let mut blocks = vec![Block {
            confidence: 95.0,
            x: 20,
            y: 20,
            width: 80,
            height: 15,
            lines: 1,
            em: 20.0,
            translated: "Kurz".into(),
            ..Default::default()
        }];
        let out = render(&original, &mut blocks, 12).unwrap();
        assert!(!blocks[0].overflow);
        assert!(out.to_rgb8() != original.to_rgb8());
        // The content area right of the sidebar keeps its own colour.
        assert_eq!(out.get_pixel(140, 27).0[..3], [246, 245, 241]);
    }
    #[test]
    fn passes_merge_without_duplicates() {
        let header = String::from("header\n");
        // Sparse pass: the title read whole, a typo in the table, no button.
        let sparse = header.clone()
            + &word(1, 1, 1, 72, 94, 283, 54, "Workspace")
            + &word(2, 1, 1, 640, 400, 160, 30, "OSWaorld");
        // Layout pass: the title split in two overlapping words, the table word read
        // right with more confidence, and the button label.
        let layout = header
            + &word(1, 1, 1, 72, 94, 137, 42, "Work")
            + &word(1, 1, 2, 123, 106, 232, 42, "orkspace")
            + "5\t1\t2\t1\t1\t1\t640\t400\t160\t30\t96\tOSWorld\n"
            + &word(3, 1, 1, 698, 1246, 98, 30, "Save")
            + &word(3, 1, 1, 811, 1244, 179, 42, "changes");
        let b = parse_tsv_merged(&sparse, &layout).unwrap();
        let sources: Vec<_> = b.iter().map(|b| b.source.as_str()).collect();
        assert_eq!(sources, ["Workspace", "OSWorld", "Save changes"]);
    }
    #[test]
    fn uncertain_ocr_does_not_paint_over_the_original() {
        let bg = image::Rgb([14, 17, 22]);
        let blue = image::Rgb([68, 147, 248]);
        let mut img = image::RgbImage::from_pixel(120, 50, bg);
        for y in 10..25 {
            for x in [13, 17, 22, 26] {
                img.put_pixel(x, y, blue);
            }
        }
        for x in 12..40 {
            img.put_pixel(x, 27, blue);
        }
        let original = DynamicImage::ImageRgb8(img);
        let mut blocks = [Block {
            x: 12,
            y: 10,
            width: 28,
            height: 15,
            lines: 1,
            em: 18.0,
            source: "azer".into(),
            translated: "азер".into(),
            confidence: 37.0,
            ..Default::default()
        }];
        let out = render(&original, &mut blocks, 12).unwrap();
        assert!(out.to_rgb8() == original.to_rgb8());

        // Once corrected by hand, the replacement uses the original blue and
        // extends the underline to cover the new label.
        blocks[0].corrected = true;
        let out = render(&original, &mut blocks, 12).unwrap().to_rgb8();
        assert!(out != original.to_rgb8());
        assert_eq!(out.get_pixel(41, 27), &blue);
        assert!(out.rows().skip(10).take(15).flatten().any(|p| p == &blue));
    }

    #[test]
    fn numbers_alone_are_no_text() {
        assert!(parse_tsv(&format!("header\n{}", word(1, 1, 1, 10, 10, 40, 12, "73.4%"))).is_err());
    }
    #[test]
    fn render_marks_overflow_and_keeps_original() {
        let mut img = image::RgbImage::new(100, 50);
        for y in 10..20 {
            img.put_pixel(15, y, image::Rgb([255; 3]));
        }
        let original = DynamicImage::ImageRgb8(img);
        let mut blocks = vec![Block {
            x: 10,
            y: 10,
            width: 10,
            height: 10,
            confidence: 95.0,
            translated: "Очень длинный перевод".into(),
            ..Default::default()
        }];
        let out = render(&original, &mut blocks, 12).unwrap();
        assert!(blocks[0].overflow);
        assert_eq!(out.dimensions(), original.dimensions());
        // Nothing is painted over text whose translation does not fit.
        assert_eq!(out.get_pixel(12, 12)[0], 0);
    }
}
