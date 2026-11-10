# ─────────────────────────── shell pieces ───────────────────────────

def card(name, w=640, h=None, x=0, y=0, clip=True):
    c = F(None, name, d="v", w=w, h=h, sv=None if h else "hug", fill=BG, stroke=LINE, r=12, clip=clip, x=x, y=y,
          effects=[{"type": "dropShadow", "color": "#00000080", "offset": [0, 18], "radius": 48}])
    cmd("variable.bind", ids=[ref(c)], field="effects/0/color", variable="color/shadow")
    return c


def topbar(c, active="Перевод", history_count="24", extra=None):
    tb = F(c, "Top bar", d="h", gap=10, pad=[10, 10, 10, 14], cross="center", sh="fill")
    m = F(tb, "Mark", d="h", w=28, h=28, r=7, fill=ACC_SOFT, main="center", cross="center")
    T(m, "\U000F05CA", size=16, color=ACC, name="Glyph nf-md-translate")
    seg = F(tb, "Tabs", d="h", gap=2, pad=3, r=9, fill=SURF, cross="center")
    tab(seg, "Перевод", active == "Перевод")
    tab(seg, "История", active == "История", count=history_count)
    if extra:
        extra(tb)
    spacer(tb)
    btn(tb, icon="sliders", kind="ghost", name="IconButton/Настройки")
    btn(tb, icon="close", kind="ghost", name="IconButton/Закрыть")
    return tb


def langbar(c, src="Английский", src_badge="авто", dst="Русский", dst_open=False, right=None):
    lb = F(c, "Language bar", d="h", gap=6, pad=[2, 14, 10, 14], cross="center", sh="fill")
    lang_chip(lb, src, auto=src_badge, name="Источник")
    btn(lb, icon="swap", kind="ghost", name="IconButton/Поменять языки")
    lang_chip(lb, dst, open_=dst_open, name="Цель")
    if right:
        spacer(lb)
        right(lb)
    return lb


def footer(c, provider="Google", status=ACC, hints=True, left_extra=None):
    f = F(c, "Status bar", d="h", gap=14, pad=[9, 14, 9, 12], cross="center", sh="fill", fill=SURF,
          stroke=LINE, strokeTopWeight=1, strokeRightWeight=0, strokeBottomWeight=0, strokeLeftWeight=0)
    p = F(f, "ProviderPicker", d="h", gap=7, pad=[3, 6, 3, 6], r=6, cross="center")
    node("ellipse", p, "Status dot", w=7, h=7, fill=status)
    T(p, provider, size=12, color=MUTED)
    I(p, "chev", color=FAINT, size=16)
    if left_extra:
        left_extra(f)
    spacer(f)
    if hints:
        for keys, label in ((("Ctrl", "Enter"), "перевести"), (("Ctrl", "Shift", "C"), "копировать"), (("Esc",), "закрыть")):
            g = F(f, f"Hint/{label}", d="h", gap=6, cross="center")
            kbd(g, *keys)
            T(g, label, size=11, color=FAINT)
    return f


def source_panel(parent, text, placeholder=False, focused=True, count="34 / 5 000", detected=None):
    sp = F(parent, "Source", d="v", gap=6, pad=[12, 8, 8, 14], r=10, fill=SURF, sh="fill",
           stroke=ACC_LINE if focused else None)
    tx = T(sp, text, size=15, color=FAINT if placeholder else TEXT, fill_w=True, name="Input", lineHeight="150%")
    setp(tx, sizingV="fixed", height=66, textAutoResize="none")
    row = F(sp, "Input toolbar", d="h", gap=4, cross="center", sh="fill")
    btn(row, "Вставить", icon="paste", kind="ghost", size="sm")
    btn(row, "Область экрана", icon="region", kind="ghost", size="sm")
    spacer(row)
    if count:
        T(row, count, size=11, color=FAINT)
    if not placeholder:
        btn(row, icon="close", kind="ghost", size="sm", name="IconButton/Очистить")
    return sp


def result_panel(parent, text=None, meta="Google · 184 ms", state="done"):
    tint = {"done": ACC_TINT, "loading": ACC_TINT, "error": DANGER_TINT, "empty": "none"}[state]
    line = {"done": ACC_LINE, "loading": ACC_LINE, "error": DANGER_LINE, "empty": LINE}[state]
    rp = F(parent, "Result", d="v", gap=10, pad=[14, 10, 10, 14], r=10, fill=tint, stroke=line, sh="fill", clip=True)
    return rp


def screen_text(name, x, y, *, src_text, placeholder=False, focused=True, count="34 / 5 000", body=None,
                provider="Google", status=ACC, src_lang="Английский", src_badge="авто", dst_open=False, clip=True):
    c = card(name, x=x, y=y, clip=clip)
    topbar(c)
    langbar(c, src=src_lang, src_badge=src_badge, dst_open=dst_open)
    ws = F(c, "Workspace", d="v", gap=8, pad=[0, 14, 14, 14], sh="fill")
    source_panel(ws, src_text, placeholder=placeholder, focused=focused, count=count)
    if body:
        body(ws)
    footer(c, provider=provider, status=status)
    return c


# ─────────────────────────── page header + notes ───────────────────────────

def _unused_label_and_notes(x, y, w, title, notes):
    T(None, title, size=18, color=TEXT, weight="SemiBold", x=x, y=y - 64)
    n = T(None, notes, size=12, color=MUTED, w=w, x=x, y=y - 38, lineHeight="150%")
    return n


def place(title, notes, x, y, w):
    T(None, title, size=20, color=TEXT, weight="SemiBold", name="Label · " + title, x=x, y=y)
    T(None, notes, size=12, color=MUTED, w=w, x=x, y=y + 36, lineHeight="160%", name="Notes · " + title)


# ─────────────────────────── screens ───────────────────────────
GAP = 120
W = 640
ROW1_Y = 200
COLX = [i * (W + GAP) for i in range(6)]

# page title
T(None, "OmaTranslate · редизайн", size=36, color=TEXT, weight="Bold", x=0, y=-120)
T(None, "Окно переводчика Omarchy: быстрый перевод выделенного текста, буфера и области экрана. "
        "Цвета и шрифт берутся из темы Omarchy (здесь — тема по умолчанию).",
  size=14, color=MUTED, w=1100, x=0, y=-66, lineHeight="150%")

# R1 — done
def r1_body(ws):
    rp = result_panel(ws)
    T(rp, "Маленькие шаги многое меняют.", size=18, color=TEXT, fill_w=True, name="Translation", lineHeight="145%")
    row = F(rp, "Result actions", d="h", gap=6, cross="center", sh="fill")
    T(row, "Google · 184 ms", size=11, color=FAINT)
    spacer(row)
    btn(row, icon="retry", kind="ghost", name="IconButton/Перевести заново")
    btn(row, "Копировать", icon="copy", kind="primary")

place("01 · Перевод готов", "Шапка стала строкой вкладок: «Перевод» и «История» вместо логотипа и слогана — "
      "+50 px под текст. Действия стоят там, где ими пользуются: «Вставить» и «Область экрана» у поля ввода, "
      "«Копировать» у результата. Результат выделен тоном акцента, чтобы взгляд сразу падал на него. "
      "Провайдер ушёл в строку статуса — его меняют редко.", COLX[0], 0, W)
screen_text("01 · Перевод — готово", COLX[0], ROW1_Y, src_text="Small steps make a big difference.", focused=True,
            body=r1_body)

# R2 — empty / onboarding
def r2_body(ws):
    rp = F(ws, "Onboarding", d="v", gap=4, pad=[12, 12, 12, 14], r=10, stroke=LINE, sh="fill")
    T(rp, "Ещё три способа перевести, не открывая окно", size=11, color=FAINT, name="Hint title")
    for icon, title, desc, keys in (
            ("cursor", "Выделенный текст", "выдели в любом приложении", ("Super", "U")),
            ("paste", "Буфер обмена", "текст или картинка", ("Super", "Shift", "U")),
            ("region", "Область экрана", "текст прямо на изображении", ("Super", "Alt", "U"))):
        r = F(rp, f"Way/{title}", d="h", gap=12, pad=[8, 0, 8, 0], cross="center", sh="fill")
        ic = F(r, "Icon", d="h", w=30, h=30, r=8, fill=SURF, main="center", cross="center")
        I(ic, icon, color=ACC)
        col = F(r, "Text", d="v", gap=2)
        T(col, title, size=13, color=TEXT)
        T(col, desc, size=11, color=MUTED)
        spacer(r)
        kbd(r, *keys)

place("02 · Пустое окно", "Пустой результат больше не висит «Здесь появится перевод». Пока поле пустое, "
      "окно учит: показывает три глобальных сочетания (выделение, буфер, область экрана). Курсор уже в поле "
      "ввода. Счётчик символов и «Очистить» скрыты, пока нечего считать.", COLX[1], 0, W)
screen_text("02 · Перевод — пусто", COLX[1], ROW1_Y, src_text="Введи текст или вставь из буфера…", placeholder=True,
            count=None, body=r2_body, src_lang="Автоопределение", src_badge=None)

# R3 — loading
def r3_body(ws):
    rp = result_panel(ws, state="loading")
    node("rectangle", rp, "Progress", x=0, y=0, w=220, h=2, fill=ACC, absolute=True)
    sk = F(rp, "Skeleton", d="v", gap=10, pad=[6, 0, 6, 0], sh="fill")
    for wd in (470, 380, 210):
        node("rectangle", sk, "Line", w=wd, h=13, fill=SKEL, cornerRadius=5)
    row = F(rp, "Result actions", d="h", gap=6, cross="center", sh="fill")
    T(row, "Перевожу через Google…", size=11, color=MUTED)
    spacer(row)
    b = btn(row, "Копировать", icon="copy", kind="primary")
    setp(b, opacity=0.35)

place("03 · Перевожу", "Вместо слова «Перевожу…» — скелетон длины будущего текста и тонкая полоса прогресса: "
      "окно не прыгает по высоте, когда приходит ответ. Язык источника уже определён и подписан бейджем, "
      "«Копировать» заблокирована до результата.", COLX[2], 0, W)
screen_text("03 · Перевод — загрузка", COLX[2], ROW1_Y,
            src_text="The quick brown fox jumps over the lazy dog, and the dog doesn’t even notice.",
            count="78 / 5 000", body=r3_body, src_badge="определён")

# R4 — error
def r4_body(ws):
    rp = result_panel(ws, state="error")
    h = F(rp, "Error title", d="h", gap=10, cross="center", sh="fill")
    I(h, "alert", color=DANGER)
    T(h, "Google не отвечает", size=14, color=DANGER, weight="SemiBold")
    T(rp, "Похоже, нет связи с сервисом. Повтори запрос или переведи через другой — "
          "текст останется в поле.", size=12, color=MUTED, fill_w=True, lineHeight="150%")
    row = F(rp, "Recovery", d="h", gap=6, cross="center", sh="fill")
    btn(row, "Повторить", icon="retry", kind="subtle", size="sm")
    btn(row, "Через MyMemory", kind="outline", size="sm")
    btn(row, "Офлайн", icon="cpu", kind="outline", size="sm")

place("04 · Ошибка сети", "Ошибка показывается там, где ждали перевод, и сразу даёт выход: повторить, "
      "переключиться на другой сервис или на офлайн-модель — одним нажатием, без похода в настройки. "
      "Точка статуса в нижней строке тоже становится красной.", COLX[3], 0, W)
screen_text("04 · Перевод — ошибка", COLX[3], ROW1_Y, src_text="Small steps make a big difference.", body=r4_body,
            status=DANGER, focused=False)

# R5 — language picker open
def r5_body(ws):
    rp = result_panel(ws)
    T(rp, "Маленькие шаги многое меняют.", size=18, color=TEXT, fill_w=True, name="Translation", lineHeight="145%")
    row = F(rp, "Result actions", d="h", gap=6, cross="center", sh="fill")
    T(row, "Google · 184 ms", size=11, color=FAINT)
    spacer(row)
    btn(row, icon="retry", kind="ghost", name="IconButton/Перевести заново")
    btn(row, "Копировать", icon="copy", kind="primary")

place("05 · Выбор языка", "Список языков с поиском: печатаешь «нем» — остаётся Deutsch. Сверху недавние, "
      "у языков с офлайн-моделью — пометка «офлайн». Выбор сразу переводит заново; Esc закрывает список, "
      "не окно.", COLX[4], 0, W)
c5 = screen_text("05 · Перевод — выбор языка", COLX[4], ROW1_Y, src_text="Small steps make a big difference.",
                 body=r5_body, dst_open=True, clip=False, focused=False)
pop = F(None, "05 · Попап выбора языка", d="v", gap=2, pad=6, w=290, r=10, fill=SURF, stroke=LINE, x=COLX[4] + 196, y=ROW1_Y + 94,
        effects=[{"type": "dropShadow", "color": "#00000099", "offset": [0, 12], "radius": 32}])
field(pop, "Найти язык…", "Focus", icon="search", sizingH="fill")
def lang_row(p, name, code, sel=False, offline=False, hover=False):
    r = F(p, f"Lang/{name}", d="h", gap=8, pad=[0, 10, 0, 10], h=32, r=6, fill=SURF2 if hover else "none",
          cross="center", sh="fill")
    if sel:
        I(r, "check", color=ACC)
    else:
        F(r, "NoCheck", d=None, w=16, h=16)
    T(r, name, size=13, color=TEXT)
    spacer(r)
    if offline:
        badge(r, "офлайн", color=MUTED, bg="#FFFFFF0D")
    T(r, code, size=11, color=FAINT)
grp(pop, "НЕДАВНИЕ")
lang_row(pop, "Русский", "RU", sel=True, offline=True)
lang_row(pop, "English", "EN", offline=True, hover=True)
lang_row(pop, "Deutsch", "DE")
node("rectangle", pop, "Divider", h=1, fill=LINE, sizingH="fill")
grp(pop, "ВСЕ ЯЗЫКИ")
for nm, cd in (("Español", "ES"), ("Français", "FR"), ("Italiano", "IT"), ("Português", "PT"), ("日本語", "JA"),
               ("中文", "ZH"), ("한국어", "KO")):
    lang_row(pop, nm, cd)


# R6 — history
place("06 · История", "История — вкладка, а не отдельный режим, который прячет всё остальное. Поиск по "
      "исходнику и переводу, группы по дням, у записи видна пара языков и время. Клик возвращает запись "
      "в поле перевода, иконка копирует результат. «Очистить» — внизу и с подтверждением.", COLX[5], 0, W)
c6 = card("06 · История", x=COLX[5], y=ROW1_Y)
topbar(c6, active="История")
hb = F(c6, "History", d="v", gap=6, pad=[2, 14, 14, 14], sh="fill")
field(hb, "Искать в истории", "Placeholder", icon="search", fill=SURF, sizingH="fill", stroke="none")
def hgroup(title):
    grp(hb, title, pad=[8, 12, 0, 12])
def hentry(pair, time, src, dst, hover=False):
    e = F(hb, "Entry", d="h", gap=12, pad=[10, 10, 10, 12], r=8, fill=SURF2 if hover else "none", sh="fill",
          cross="min")
    col = F(e, "Text", d="v", gap=4, sh="fill")
    meta = F(col, "Meta", d="h", gap=8, cross="center")
    badge(meta, pair, color=MUTED, bg="#FFFFFF0D")
    T(meta, time, size=11, color=FAINT)
    T(col, src, size=12, color=MUTED, fill_w=True)
    T(col, dst, size=13, color=TEXT, fill_w=True, lineHeight="145%")
    if hover:
        acts = F(e, "Row actions", d="h", gap=2, cross="center")
        btn(acts, icon="arrow", kind="ghost", size="sm", name="IconButton/Вернуть в перевод")
        btn(acts, icon="copy", kind="ghost", size="sm", name="IconButton/Копировать")
F(hb, "Gap", d=None, h=4, sh="fill")
hgroup("СЕГОДНЯ")
hentry("EN → RU", "14:32", "Small steps make a big difference.", "Маленькие шаги многое меняют.", hover=True)
hentry("EN → RU", "13:05", "The build failed because the lockfile is out of date.",
       "Сборка не удалась: lock-файл устарел.")
hentry("RU → EN", "11:48", "Спасибо, что посмотрел пул-реквест!", "Thanks for reviewing the pull request!")
hgroup("ВЧЕРА")
hentry("EN → RU", "22:17", "Keep the window open while the model downloads.",
       "Не закрывай окно, пока загружается модель.")
fh = F(c6, "Status bar", d="h", gap=10, pad=[9, 14, 9, 14], cross="center", sh="fill", fill=SURF,
       stroke=LINE, strokeTopWeight=1, strokeRightWeight=0, strokeBottomWeight=0, strokeLeftWeight=0)
I(fh, "shield", color=FAINT)
T(fh, "24 записи · хранятся только на этом устройстве", size=11, color=FAINT)
spacer(fh)
btn(fh, "Очистить…", icon="trash", kind="ghost", size="sm", icon_color=DANGER)

# ─────────────────────────── Row 2: image mode ───────────────────────────
ROW2_LABEL_Y = 1000
ROW2_Y = ROW2_LABEL_Y + 230
IW = 1040
place("07 · Перевод изображения", "Переключатель «Перевод / Оригинал» — сегменты с видимым состоянием, а не "
      "кнопка, у которой меняется подпись. Номера фрагментов есть и на картинке, и в списке справа — легко "
      "сопоставить. Правка фрагмента — прямо в карточке, с явной кнопкой «Перевести». Главное действие "
      "режима — «Копировать изображение»; текст и PNG — рядом, но тише.", 0, ROW2_LABEL_Y, IW)
c7 = card("07 · Изображение — перевод", w=IW, h=700, x=0, y=ROW2_Y)
def img_ctx(tb):
    badge(tb, "Изображение · 1280 × 720", color=MUTED, bg="#FFFFFF0D", size=11)
topbar(c7, extra=img_ctx)
def img_tools(lb):
    seg = F(lb, "View", d="h", gap=2, pad=3, r=9, fill=SURF, cross="center")
    for i, l in enumerate(("Перевод", "Оригинал")):
        segment(seg, l, i == 0)
    z = F(lb, "Zoom", d="h", gap=0, pad=3, r=9, fill=SURF, cross="center")
    btn(z, icon="minus", kind="ghost", size="sm", name="IconButton/Уменьшить")
    zz = F(z, "Zoom value", d="h", pad=[0, 6, 0, 6], h=28, cross="center")
    T(zz, "100%", size=12, color=TEXT)
    btn(z, icon="plus", kind="ghost", size="sm", name="IconButton/Увеличить")
    btn(z, icon="fit", kind="ghost", size="sm", name="IconButton/Вписать")
langbar(c7, src_badge="определён", right=img_tools)
body = F(c7, "Body", d="h", gap=12, pad=[0, 14, 12, 14], sh="fill", sv="fill")
cv = F(body, "Canvas", d="h", pad=16, r=10, fill=CANVAS, stroke=LINE, sh="fill", sv="fill", main="center",
       cross="center", clip=True)
pic = F(cv, "Picture · translated", d="v", gap=14, pad=[30, 34, 34, 34], w=620, r=4, fill="#F3F0E8", clip=False)
def mark(block, n, sel=False):
    m = F(block, f"Marker {n}", d="h", w=18, h=18, r=9, fill=ACC if sel else "#2F3A36", main="center", cross="center",
          x=-26, y=2, absolute=True)
    T(m, str(n), size=10, color=INK if sel else "#F3F0E8", weight="Bold")
b1 = F(pic, "Block 1", d="v", pad=2, clip=False)
T(b1, "Быстрый старт", size=22, color="#1D2125", weight="Bold")
mark(b1, 1)
b2 = F(pic, "Block 2 · selected", d="v", pad=2, sh="fill", stroke=ACC, sw=2, r=2, clip=False)
T(b2, "Установите пакет и запустите сервис одной командой. Настройки сохраняются автоматически.", size=14,
  color="#3A4046", fill_w=True, lineHeight="150%")
mark(b2, 2, sel=True)
code = F(pic, "Code (no text)", d="h", pad=[11, 14, 11, 14], r=4, fill="#22272B", sh="fill")
T(code, "$ yay -S omatranslate", size=12, color="#C9D3CF")
b3 = F(pic, "Block 3", d="v", pad=2, sh="fill", clip=False)
T(b3, "Сочетания клавиш меняются в настройках приложения.", size=14, color="#3A4046", fill_w=True,
  lineHeight="150%")
mark(b3, 3)
b4 = F(pic, "Block 4", d="h", pad=[8, 16, 8, 16], r=6, fill="#2F6F5E", clip=False)
T(b4, "Начать сейч…", size=12, color="#FFFFFF", weight="Bold")
mark(b4, 4)
side = F(body, "Fragments", d="v", gap=8, w=300, sv="fill", clip=True)
sh_ = F(side, "Header", d="h", gap=8, pad=[4, 2, 2, 2], cross="center", sh="fill")
T(sh_, "Фрагменты", size=12, color=TEXT, weight="SemiBold")
T(sh_, "4", size=11, color=FAINT)
spacer(sh_)
T(sh_, "клик — правка", size=11, color=FAINT)
def frag(n, src, dst, state="idle", note=None):
    st = {"idle": None, "selected": ACC, "edit": ACC}[state]
    k = F(side, f"Fragment {n}" + (f" · {state}" if state != "idle" else ""), d="v", gap=7, pad=[10, 10, 12, 12],
          r=8, fill=SURF, stroke=st, sh="fill")
    hd = F(k, "Head", d="h", gap=8, cross="center", sh="fill")
    m = F(hd, "Num", d="h", w=18, h=18, r=9, fill=ACC if state != "idle" else SURF2, main="center", cross="center")
    T(m, str(n), size=10, color=INK if state != "idle" else MUTED, weight="Bold")
    if note:
        badge(hd, note, color=WARN, bg=WARN_TINT)
    spacer(hd)
    if state != "edit":
        I(hd, "pencil", color=FAINT)
    if state == "edit":
        fld = F(k, "Edit field", d="v", pad=[8, 10, 8, 10], r=6, fill=BG, stroke=ACC_LINE, sh="fill")
        T(fld, src, size=12, color=TEXT, fill_w=True, lineHeight="145%")
        acts = F(k, "Edit actions", d="h", gap=6, cross="center", sh="fill")
        btn(acts, "Перевести", kind="primary", size="sm")
        btn(acts, "Отмена", kind="ghost", size="sm")
    else:
        T(k, src, size=11, color=MUTED, fill_w=True, lineHeight="145%")
        T(k, dst, size=13, color=TEXT, fill_w=True, lineHeight="145%")
frag(1, "Quick start", "Быстрый старт")
frag(2, "Install the package and start the service with one command. Settings are saved automatically.",
     "Установите пакет и запустите сервис одной командой. Настройки сохраняются автоматически.", "selected")
frag(3, "Shortcuts can be changed in the app settings", "", "edit")
frag(4, "Get started now", "Начать сейчас", note="не влезло · полный текст")
ab = F(c7, "Action bar", d="h", gap=8, pad=[10, 14, 10, 14], cross="center", sh="fill", fill=SURF,
       stroke=LINE, strokeTopWeight=1, strokeRightWeight=0, strokeBottomWeight=0, strokeLeftWeight=0)
I(ab, "shield", color=FAINT)
T(ab, "Картинка не покидает устройство · OCR eng+rus · 612 ms", size=11, color=FAINT)
spacer(ab)
btn(ab, "PNG", icon="download", kind="ghost", size="sm")
btn(ab, "Текст", icon="copy", kind="subtle", size="sm")
btn(ab, "Копировать изображение", icon="image", kind="primary", size="sm")

# ─────────────────────────── Settings ───────────────────────────
SX0 = IW + 160
NAV = (("sliders", "Основное"), ("globe", "Сервисы"), ("cpu", "Офлайн-модели"), ("image", "Изображения"),
       ("keyboard", "Горячие клавиши"))


def settings(name, x, active, title, desc, fill_content):
    c = card(name, w=IW, h=660, x=x, y=ROW2_Y)
    tb = F(c, "Top bar", d="h", gap=10, pad=[10, 10, 10, 10], cross="center", sh="fill",
           stroke=LINE, strokeTopWeight=0, strokeRightWeight=0, strokeBottomWeight=1, strokeLeftWeight=0)
    btn(tb, icon="back", kind="ghost", name="IconButton/Назад к переводу")
    T(tb, "Настройки", size=14, color=TEXT, weight="SemiBold")
    spacer(tb)
    T(tb, "изменения сохраняются сразу", size=11, color=FAINT)
    btn(tb, icon="close", kind="ghost", name="IconButton/Закрыть")
    body = F(c, "Body", d="h", sh="fill", sv="fill")
    nav = F(body, "Nav", d="v", gap=2, pad=10, w=220, sv="fill", fill=SURF,
            stroke=LINE, strokeTopWeight=0, strokeRightWeight=1, strokeBottomWeight=0, strokeLeftWeight=0)
    for icon, lab in NAV:
        nav_item(nav, icon, lab, lab == active)
    spacer_v = F(nav, "Spacer", d=None, w=1, sv="fill")
    T(nav, "OmaTranslate 0.1.0", size=11, color=FAINT, name="Version")
    ct = F(body, "Content", d="v", gap=16, pad=[22, 28, 22, 28], sh="fill", sv="fill", clip=True)
    T(ct, title, size=18, color=TEXT, weight="SemiBold", name="Title")
    T(ct, desc, size=12, color=MUTED, fill_w=True, lineHeight="150%", name="Description")
    fill_content(ct)
    return c


def option_card(parent, sel, icon, title, desc, tags=(), extra=None):
    k = F(parent, f"Provider/{title}", d="h", gap=14, pad=[14, 16, 14, 16], r=10, fill=ACC_TINT if sel else "none",
          stroke=ACC if sel else LINE, sh="fill", cross="min")
    radio(k, sel)
    col = F(k, "Text", d="v", gap=6, sh="fill")
    hd = F(col, "Head", d="h", gap=8, cross="center")
    I(hd, icon, color=ACC if sel else MUTED)
    T(hd, title, size=14, color=TEXT, weight="SemiBold")
    for t, c, b in tags:
        badge(hd, t, color=c, bg=b)
    T(col, desc, size=12, color=MUTED, fill_w=True, lineHeight="150%")
    if extra:
        extra(col)
    return k


def services(ct):
    option_card(ct, True, "globe", "Google", "Быстро и без ключа. Неофициальный адрес — при частых запросах "
                "может временно ограничить.", tags=(("по умолчанию", ACC, ACC_SOFT),))
    def mm(col):
        r = F(col, "Quota", d="h", gap=10, cross="center", sh="fill")
        bar = F(r, "Bar", d=None, w=180, h=4, r=2, fill=SURF2)
        node("rectangle", bar, "Value", x=0, y=0, w=45, h=4, fill=MUTED, cornerRadius=2)
        T(r, "сегодня 1 240 из 5 000 символов", size=11, color=FAINT)
    option_card(ct, False, "globe", "MyMemory", "Без регистрации, до 5 000 символов в день.", extra=mm)
    def lt(col):
        r = F(col, "Fields", d="h", gap=8, cross="center", sh="fill")
        field(r, "http://localhost:5000", "Filled", sizingH="fill")
        field(r, "API-ключ, если нужен", "Placeholder", width=180)
        btn(r, "Проверить", kind="outline", size="md")
        s = F(col, "Check", d="h", gap=6, cross="center")
        I(s, "check", color=ACC)
        T(s, "Сервер отвечает · 42 ms", size=11, color=ACC)
    option_card(ct, False, "server", "LibreTranslate", "Свой или публичный сервер. Удалённый — только по HTTPS.",
                extra=lt)
    def off(col):
        r = F(col, "Models", d="h", gap=10, cross="center")
        T(r, "Модели: 1 из 2 загружены", size=11, color=FAINT)
        l = F(r, "Link", d="h", gap=4, cross="center")
        T(l, "Управлять моделями", size=11, color=ACC)
        I(l, "arrow", color=ACC)
    option_card(ct, False, "cpu", "Офлайн", "Английский ↔ русский прямо на устройстве. Текст не уходит в сеть.",
                tags=(("EN ↔ RU", MUTED, "#FFFFFF0D"),), extra=off)


def models(ct):
    st = F(ct, "Engine", d="h", gap=10, pad=[10, 14, 10, 14], r=8, fill=ACC_TINT, stroke=ACC_LINE, sh="fill",
           cross="center")
    I(st, "check", color=ACC)
    T(st, "Офлайн-движок установлен и работает на процессоре — GPU не нужен.", size=12, color=TEXT)
    def model(pair, status, action, progress=None, installed=False):
        k = F(ct, f"Model/{pair}", d="v", gap=10, pad=[14, 16, 14, 16], r=10, stroke=LINE, sh="fill")
        r = F(k, "Row", d="h", gap=12, cross="center", sh="fill")
        ic = F(r, "Icon", d="h", w=34, h=34, r=8, fill=SURF, main="center", cross="center")
        I(ic, "cpu", color=ACC if installed else MUTED)
        col = F(r, "Text", d="v", gap=3, sh="fill")
        T(col, pair, size=14, color=TEXT, weight="SemiBold")
        T(col, status, size=11, color=MUTED)
        action(r)
        if progress:
            p = F(k, "Progress", d="h", gap=10, cross="center", sh="fill")
            bar = F(p, "Bar", d=None, h=4, r=2, fill=SURF2, sh="fill", clip=True)
            node("rectangle", bar, "Value", x=0, y=0, w=420, h=4, fill=ACC, cornerRadius=2)
            T(p, progress, size=11, color=MUTED)
    model("English → Русский", "Установлена · Argos 1.9 · 62 МБ",
          lambda r: btn(r, "Удалить", icon="trash", kind="ghost", size="sm", icon_color=DANGER), installed=True)
    model("Русский → English", "Загружается с официального сервера Argos",
          lambda r: btn(r, "Отмена", kind="outline", size="sm"), progress="38.4 из 62.0 МБ · ~15 с")
    row = F(ct, "Keep warm", d="h", gap=14, pad=[4, 0, 4, 0], cross="center", sh="fill")
    col = F(row, "Text", d="v", gap=3, sh="fill")
    T(col, "Держать модели в памяти", size=13, color=TEXT)
    T(col, "Первый перевод без задержки. Иначе модели выгружаются через 10 минут простоя.", size=11,
      color=MUTED, fill_w=True)
    toggle(row, False)


def shortcuts(ct):
    def sc(title, desc, keys=None, recording=False, warn=None):
        k = F(ct, f"Shortcut/{title}", d="h", gap=14, pad=[12, 14, 12, 16], r=10,
              stroke=ACC if recording else LINE, fill=ACC_TINT if recording else "none", sh="fill", cross="center")
        col = F(k, "Text", d="v", gap=3, sh="fill")
        T(col, title, size=13, color=TEXT, weight="SemiBold")
        T(col, desc, size=11, color=MUTED, fill_w=True)
        if warn:
            w = F(col, "Warning", d="h", gap=6, cross="center")
            I(w, "alert", color=WARN)
            T(w, warn, size=11, color=WARN)
        if recording:
            f = F(k, "Recorder", d="h", gap=8, pad=[0, 12, 0, 12], h=34, r=7, fill=BG, stroke=ACC, cross="center")
            node("ellipse", f, "Rec", w=7, h=7, fill=DANGER)
            T(f, "Нажми сочетание…", size=12, color=TEXT)
            kbd(f, "Esc")
        else:
            kbd(k, *keys, size=12)
            btn(k, icon="pencil", kind="ghost", size="sm", name="IconButton/Изменить")
    sc("Перевести выделение", "Выделенный текст в любом приложении", ("Super", "U"))
    sc("Перевести буфер обмена", "Текст или картинка из буфера", ("Super", "Shift", "U"),
       warn="Уже назначено в Hyprland — выбери другое сочетание")
    sc("Перевести область экрана", "Выдели область — текст переведётся прямо на ней", recording=True)
    n = F(ct, "Note", d="h", gap=8, cross="center")
    I(n, "info", color=FAINT)
    T(n, "Сочетания применяются в Hyprland сразу, отдельная кнопка «Применить» не нужна.", size=11, color=FAINT)


def images(ct):
    def srow(title, desc, control, warn=None):
        r = F(ct, f"Option/{title}", d="h", gap=14, pad=[12, 0, 12, 0], cross="center", sh="fill",
              stroke=LINE, strokeTopWeight=0, strokeRightWeight=0, strokeBottomWeight=1, strokeLeftWeight=0)
        c = F(r, "Text", d="v", gap=3, sh="fill")
        T(c, title, size=13, color=TEXT)
        T(c, desc, size=11, color=MUTED, fill_w=True, lineHeight="145%")
        if warn:
            w = F(c, "Warning", d="h", gap=8, pad=[4, 0, 0, 0], cross="center")
            I(w, "alert", color=WARN)
            T(w, warn, size=11, color=WARN)
            btn(w, "Скопировать команду", icon="copy", kind="ghost", size="sm")
        control(r)
    def seg3(r, labels, active=0):
        seg = F(r, "Segmented", d="h", gap=2, pad=3, r=8, fill=SURF, cross="center")
        for i, v in enumerate(labels):
            segment(seg, v, i == active)
    srow("Языки распознавания", "Чем меньше языков, тем быстрее и точнее OCR.",
         lambda r: seg3(r, ("EN + RU", "EN", "RU")),
         warn="Для русского нужен пакет tesseract-data-rus")
    def slider(r):
        g = F(r, "Slider", d="h", gap=12, cross="center")
        tr = F(g, "Track", d=None, w=180, h=20)
        node("rectangle", tr, "Rail", x=0, y=8, w=180, h=4, fill=SURF2, cornerRadius=2)
        node("rectangle", tr, "Value", x=0, y=8, w=22, h=4, fill=ACC, cornerRadius=2)
        node("ellipse", tr, "Knob", x=15, y=3, w=14, h=14, fill=ACC)
        T(g, "12 px", size=12, color=TEXT)
    srow("Минимальный размер шрифта", "Мельче перевод на картинке не станет — длинный текст уйдёт в карточку "
         "фрагмента с пометкой «не влезло».", slider)
    srow("Сначала показывать", "Что открывается после распознавания области.",
         lambda r: seg3(r, ("Перевод", "Оригинал")))
    srow("Спрашивать, куда сохранить PNG", "Иначе файл сразу сохраняется в ~/Pictures/OmaTranslate.",
         lambda r: toggle(r, True))
    n = F(ct, "Privacy", d="h", gap=10, pad=[10, 14, 10, 14], r=8, fill=SURF, sh="fill", cross="center")
    I(n, "shield", color=ACC)
    T(n, "Распознавание (Tesseract) идёт на устройстве. Сервису перевода уходит только текст.", size=12,
      color=TEXT)


def general(ct):
    def row(title, desc, on, extra=None):
        r = F(ct, f"Option/{title}", d="h", gap=14, pad=[12, 0, 12, 0], cross="center", sh="fill",
              stroke=LINE, strokeTopWeight=0, strokeRightWeight=0, strokeBottomWeight=1, strokeLeftWeight=0)
        col = F(r, "Text", d="v", gap=3, sh="fill")
        T(col, title, size=13, color=TEXT)
        T(col, desc, size=11, color=MUTED, fill_w=True, lineHeight="145%")
        if extra:
            extra(r)
        toggle(r, on)
    def delay(r):
        seg = F(r, "Delay", d="h", gap=2, pad=3, r=8, fill=SURF, cross="center")
        for i, v in enumerate(("150", "250", "500 ms")):
            segment(seg, v, i == 1)
    row("Переводить при вводе", "Запрос уходит после паузы в наборе.", True, extra=delay)
    row("Русский → английский автоматически", "Если исходник на русском, цель сама меняется на английский.", True)
    row("Сохранять историю", "До 100 последних переводов, только на этом устройстве. Картинки не сохраняются.",
        True)
    row("Открывать окно по центру экрана", "Иначе — у курсора, рядом с выделенным текстом.", False)


SX = [SX0 + i * (IW + GAP) for i in range(5)]
place("08 · Настройки · Сервисы", "Длинная простыня настроек разбита на разделы с навигацией слева. Сервисы — "
      "карточки-переключатели с понятными ограничениями; LibreTranslate можно проверить прямо здесь. "
      "Изменения сохраняются сразу: нет кнопок «Применить», которые легко забыть.", SX[0], ROW2_LABEL_Y, IW)
settings("08 · Настройки — Сервисы", SX[0], "Сервисы", "Сервисы перевода",
         "Сервис по умолчанию. Сменить на лету можно в нижней строке окна перевода. "
         "Картинки не покидают устройство: сервису уходит только распознанный текст.", services)
place("09 · Настройки · Офлайн-модели", "Модели — карточки со статусом, размером и прогрессом загрузки "
      "в одном месте. «Удалить» — тихая кнопка с красной иконкой, чтобы её не нажали случайно. "
      "Прогрев памяти — переключатель с честным объяснением, зачем он нужен.", SX[1], ROW2_LABEL_Y, IW)
settings("09 · Настройки — Офлайн-модели", SX[1], "Офлайн-модели", "Офлайн-модели",
         "Перевод английский ↔ русский без интернета на моделях Argos.", models)
place("10 · Настройки · Горячие клавиши", "Вместо текстовых полей с «SUPER + SHIFT + U» — запись сочетания: "
      "нажал «изменить», нажал клавиши. Конфликты с Hyprland подсвечиваются сразу у нужной строки.",
      SX[2], ROW2_LABEL_Y, IW)
settings("10 · Настройки — Горячие клавиши", SX[2], "Горячие клавиши", "Горячие клавиши",
         "Глобальные сочетания работают из любого приложения.", shortcuts)
place("11 · Настройки · Основное", "Чекбоксы с длинными подписями стали строками «заголовок + пояснение + "
      "переключатель». Задержку автоперевода выбирают из трёх значений, а не крутят счётчик.",
      SX[3], ROW2_LABEL_Y, IW)
settings("11 · Настройки — Основное", SX[3], "Основное", "Основное",
         "Как ведёт себя окно перевода.", general)


place("12 · Настройки · Изображения", "Языки OCR — три понятных варианта вместо выпадающего списка. Если "
      "пакета языка нет, это видно прямо у настройки и команду установки можно скопировать. Минимальный шрифт — "
      "ползунок со значением. Новое: «Сначала показывать» и «Спрашивать, куда сохранить PNG».",
      SX[4], ROW2_LABEL_Y, IW)
settings("12 · Настройки — Изображения", SX[4], "Изображения", "Изображения",
         "Как распознаётся и сохраняется текст с картинок и областей экрана.", images)
