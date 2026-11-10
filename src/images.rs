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
fn background(image: &DynamicImage, b: &Block) -> [f64; 3] {
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
        f64::from(c.get(c.len() / 2).copied().unwrap_or(255)) / 255.0
    })
}
/// How far a block may grow to the right over plain background: up to the next
/// text, rule or image edge, so a longer translation can use the empty part of
/// its cell instead of overflowing.
fn free_right(image: &DynamicImage, b: &Block, bg: [f64; 3]) -> i32 {
    let limit = (image.width() as i32 - 1).min(b.x + b.width + (b.width * 2).max(160));
    let rows: Vec<i32> = (b.y.max(0)..(b.y + b.height).min(image.height() as i32))
        .step_by((b.height / 12).max(1) as usize)
        .collect();
    let close = |x: i32| {
        rows.iter().all(|&y| {
            let p = image.get_pixel(x as u32, y as u32);
            // Tight enough to stop at the edge of a sidebar or a card a few shades off.
            (0..3).all(|i| (f64::from(p[i]) / 255.0 - bg[i]).abs() < 0.03)
        })
    };
    let start = b.x + b.width + 2;
    let mut x = start;
    while x <= limit && close(x) {
        x += 1;
    }
    // Keep a gap before whatever stopped the scan.
    (x - start - 10).max(0)
}

/// Share of a block's pixels that are text ink, relative to its font size.
///
/// Bold text puts roughly half as much ink again into the same box as regular
/// text, so comparing a block with the image's median tells headings apart.
fn ink(image: &DynamicImage, b: &Block, bg: [f64; 3], em: f64) -> f64 {
    let (mut dark, mut total) = (0u32, 0u32);
    let step = ((b.width.min(b.height)) / 40).max(1) as usize;
    for y in (b.y.max(0)..(b.y + b.height).min(image.height() as i32)).step_by(step) {
        for x in (b.x.max(0)..(b.x + b.width).min(image.width() as i32)).step_by(step) {
            let p = image.get_pixel(x as u32, y as u32);
            let distance = (0..3).map(|i| (f64::from(p[i]) / 255.0 - bg[i]).abs()).fold(0.0, f64::max);
            total += 1;
            if distance > 0.35 {
                dark += 1;
            }
        }
    }
    // Stroke width grows with the font, ink share does not: normalise by height/em.
    f64::from(dark) / f64::from(total.max(1)) * f64::from(b.height) / em.max(1.0)
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
        ctx.select_font_face("sans-serif", FontSlant::Normal, FontWeight::Normal);
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
        let inks: Vec<f64> = blocks
            .iter()
            .map(|b| ink(original, b, background(original, b), em_of(b)))
            .collect();
        let mut sorted = inks.clone();
        sorted.sort_by(f64::total_cmp);
        // The lower quartile is regular text even when many blocks are bold.
        let usual_ink = sorted.get(sorted.len() / 4).copied().unwrap_or(0.0);
        for (index, b) in blocks.iter_mut().enumerate() {
            if b.translated.is_empty() {
                continue;
            }
            let x = (b.x - 2).max(0) as f64;
            let y = (b.y - 2).max(0) as f64;
            let width = ((b.width + 4) as f64).min(w as f64 - x);
            let height = ((b.height + 4) as f64).min(h as f64 - y);
            if width <= 0.0 || height <= 0.0 {
                continue;
            }
            let [r, g, blue] = background(original, b);
            let room = (width + f64::from(free_right(original, b, [r, g, blue]))).min(w as f64 - x);
            ctx.save()?;
            let bold = usual_ink > 0.0 && inks[index] > usual_ink * 1.35;
            ctx.select_font_face(
                "sans-serif",
                FontSlant::Normal,
                if bold { FontWeight::Bold } else { FontWeight::Normal },
            );
            // Size the translation like the original text, but never below the
            // user's minimum unless the original itself is smaller than that.
            let em = em_of(b).max(1.0);
            let largest = em.min(typical * 2.5).clamp(7.0, 72.0).round() as u32;
            let smallest = minimum.min(em as u32).max(7).min(largest);
            // A single line may use a little height around its box: the box only
            // covers the glyphs present ("Overview" has no descender, "Überblick" has dots).
            let slack = if b.lines <= 1 { (em * 0.3).max(3.0) } else { 0.0 };
            let mut fitted = None;
            for size in (smallest..=largest).rev() {
                ctx.set_font_size(size as f64);
                if let Some(lines) = wrap(&ctx, &b.translated, room - 4.0)? {
                    let extent = ctx.font_extents()?;
                    if extent.height() * lines.len() as f64 <= height + slack + extent.descent() {
                        fitted = Some((lines, extent));
                        break;
                    }
                }
            }
            // A translation that does not fit is left out of the image: the
            // original stays readable and the fragment card holds the full text.
            b.overflow = fitted.is_none();
            if let Some((lines, extent)) = fitted {
                // Cover the original box and only as much extra room as the translation uses.
                let mut used = 0.0f64;
                for line in &lines {
                    used = used.max(ctx.text_extents(line)?.x_advance());
                }
                // A little past the original box too: OCR often leaves out a final
                // period or the tail of a glyph, which would show next to a shorter translation.
                let width = width
                    .max(used + 6.0)
                    .min(room)
                    .max((width + em * 0.4).min(w as f64 - x));
                let needed = extent.height() * lines.len() as f64;
                let (y, height) = if needed > height {
                    let top = (y - (needed - height) / 2.0).max(0.0);
                    (top, needed.min(h as f64 - top))
                } else {
                    (y, height)
                };
                ctx.rectangle(x, y, width, height);
                ctx.clip();
                ctx.set_source_rgb(r, g, blue);
                ctx.paint()?;
                let foreground = if 0.2126 * r + 0.7152 * g + 0.0722 * blue > 0.55 {
                    0.08
                } else {
                    0.98
                };
                ctx.set_source_rgb(foreground, foreground, foreground);
                let top =
                    y + (height - extent.height() * lines.len() as f64) / 2.0 + extent.ascent();
                for (i, line) in lines.iter().enumerate() {
                    ctx.move_to(x + 2.0, top + i as f64 * extent.height());
                    ctx.show_text(line)?;
                }
            }
            ctx.restore()?;
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
        let original = DynamicImage::ImageRgb8(img);
        let mut blocks = vec![Block {
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
    fn numbers_alone_are_no_text() {
        assert!(parse_tsv(&format!("header\n{}", word(1, 1, 1, 10, 10, 40, 12, "73.4%"))).is_err());
    }
    #[test]
    fn render_marks_overflow_and_keeps_original() {
        let original = DynamicImage::new_rgb8(100, 50);
        let mut blocks = vec![Block {
            x: 10,
            y: 10,
            width: 10,
            height: 10,
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
