"""OmaTranslate design system for FigCraft scripts: tokens, icons, components and primitives."""

FONT = "JetBrainsMono Nerd Font"

# name: (dark, light) — dark follows the plugin's default palette, light is a warm Omarchy-like light theme.
TOKENS = {
    "color/bg": ("#13171A", "#F7F6F2"),
    "color/surface": ("#1A1F23", "#EFEEE8"),
    "color/surface-2": ("#232A2E", "#E4E2DA"),
    "color/line": ("#2A3236", "#D9D7CE"),
    "color/text": ("#E3E7E8", "#1E2326"),
    "color/text-muted": ("#8B9296", "#5D656A"),
    "color/text-faint": ("#5C656A", "#8E9599"),
    "color/accent": ("#AAC8AD", "#4A7A57"),
    "color/on-accent": ("#111714", "#FFFFFF"),
    "color/accent-soft": ("#AAC8AD24", "#4A7A5724"),
    "color/accent-tint": ("#AAC8AD0D", "#4A7A570D"),
    "color/accent-line": ("#AAC8AD33", "#4A7A5740"),
    "color/danger": ("#E9A69A", "#B4483A"),
    "color/danger-tint": ("#E9A69A12", "#B4483A10"),
    "color/danger-line": ("#E9A69A40", "#B4483A40"),
    "color/warn": ("#E2C58F", "#94661A"),
    "color/warn-tint": ("#E2C58F1A", "#94661A14"),
    "color/overlay": ("#FFFFFF0D", "#0000000A"),
    "color/skeleton": ("#FFFFFF12", "#00000010"),
    "color/canvas": ("#0C0F11", "#E3E2DB"),
    "color/shadow": ("#00000080", "#0000001F"),
}
BG, SURF, SURF2, LINE = "color/bg", "color/surface", "color/surface-2", "color/line"
TEXT, MUTED, FAINT = "color/text", "color/text-muted", "color/text-faint"
ACC, INK, ACC_SOFT, ACC_TINT, ACC_LINE = ("color/accent", "color/on-accent", "color/accent-soft", "color/accent-tint",
                                          "color/accent-line")
DANGER, DANGER_TINT, DANGER_LINE = "color/danger", "color/danger-tint", "color/danger-line"
WARN, WARN_TINT, OVERLAY, SKEL, CANVAS = "color/warn", "color/warn-tint", "color/overlay", "color/skeleton", "color/canvas"

TONES = {"Muted": MUTED, "Text": TEXT, "Faint": FAINT, "Accent": ACC, "OnAccent": INK, "Danger": DANGER, "Warn": WARN}
TONE_OF = {v: k for k, v in TONES.items()}

ICONS = {
    "copy": "M5.5 5.5h7v7h-7z M3.5 10.5v-7h7",
    "paste": "M5.5 2h5v2.5h-5z M4.5 3.25H3v10.25h10V3.25h-1.5",
    "region": "M2 5.5V2h3.5 M10.5 2H14v3.5 M14 10.5V14h-3.5 M5.5 14H2v-3.5",
    "swap": "M2.5 5h10 M10 2.5L12.5 5L10 7.5 M13.5 11h-10 M6 8.5L3.5 11L6 13.5",
    "close": "M4 4l8 8 M12 4l-8 8",
    "sliders": "M2.5 4.5h5 M10.5 4.5h3 M2.5 11.5h2 M7.5 11.5h6 M9 3v3 M6 10v3",
    "history": "M8 1.75a6.25 6.25 0 1 0 0.01 0 M8 4.5V8l2.5 1.5",
    "retry": "M13 8a5 5 0 1 1-1.46-3.54 M13 2.25v2.5h-2.5",
    "chev": "M4.5 6.5L8 10l3.5-3.5",
    "check": "M3 8.5l3 3l7-7",
    "search": "M7 2.5a4.5 4.5 0 1 0 0.01 0 M10.4 10.4L13.5 13.5",
    "back": "M13 8H3.5 M7.5 4L3.5 8l4 4",
    "alert": "M8 2L14.5 13.5H1.5Z M8 6.5v3 M8 11.5v0.25",
    "download": "M8 2.5v8 M4.5 7L8 10.5L11.5 7 M3 13.5h10",
    "image": "M2.5 3h11v10h-11z M2.5 10.5l3.5-3.5l3 3l2-2l2.5 2.5",
    "pencil": "M10.5 2.75l2.75 2.75L6 12.75H3.25V10z",
    "globe": "M8 2a6 6 0 1 0 0.01 0 M2 8h12 M8 2c2.2 2 2.2 10 0 12 M8 2c-2.2 2-2.2 10 0 12",
    "cpu": "M4.5 4.5h7v7h-7z M6.5 2v2.5 M9.5 2v2.5 M6.5 11.5V14 M9.5 11.5V14 M2 6.5h2.5 M2 9.5h2.5 M11.5 6.5H14 M11.5 9.5H14",
    "keyboard": "M1.5 4h13v8h-13z M4 6.75h0.5 M6.5 6.75h0.5 M9 6.75h0.5 M11.5 6.75h0.5 M5 9.5h6",
    "trash": "M3 4.5h10 M6 4.5V2.75h4V4.5 M4.5 4.5l0.75 9h5.5l0.75-9",
    "shield": "M8 1.75l5 2v4c0 3-2.2 5.2-5 6.5c-2.8-1.3-5-3.5-5-6.5v-4z",
    "plus": "M8 3v10 M3 8h10",
    "minus": "M3 8h10",
    "fit": "M2.5 6V2.5H6 M10 2.5h3.5V6 M13.5 10v3.5H10 M6 13.5H2.5V10",
    "arrow": "M3 8h9.5 M9 4.5L12.5 8L9 11.5",
    "info": "M8 2a6 6 0 1 0 0.01 0 M8 7.25v4 M8 4.75v0.25",
    "server": "M2.5 2.5h11v4.5h-11z M2.5 9h11v4.5h-11z M5 4.75h0.25 M5 11.25h0.25",
    "cursor": "M3.5 2.5h3 M5 2.5v11 M3.5 13.5h3 M9 5.5h4.5 M9 8h4.5 M9 10.5h3",
    "folder": "M2 4.5V3h4.5l1.5 1.5h6V13H2z",
    "scan": "M2 5V2h3 M11 2h3v3 M14 11v3h-3 M5 14H2v-3 M5 6h6 M5 8h6 M5 10h4",
}

cmds = []
_n = [0]
C = {}  # component refs: "Icon/copy", "Button", ...


def ref(r):
    return {"$ref": r} if isinstance(r, str) else r


def col(v):
    if isinstance(v, str) and v.startswith("color/"):
        return {"$var": v}
    return v


def cmd(command, as_=None, **params):
    c = {"command": command, "params": params}
    if as_:
        c["as"] = as_
    cmds.append(c)
    return as_


def new_ref():
    _n[0] += 1
    return f"r{_n[0]}"


def node(type_, parent=None, name=None, text=None, path=None, x=None, y=None, w=None, h=None, index=None, **props):
    r = new_ref()
    p = {"type": type_}
    if parent is not None:
        p["parent"] = ref(parent)
    if name:
        p["name"] = name
    if text is not None:
        p["text"] = text
    if path:
        p["path"] = path
    for k, v in (("x", x), ("y", y), ("width", w), ("height", h), ("index", index)):
        if v is not None:
            p[k] = v
    for k in ("fill", "stroke"):
        if k in props:
            props[k] = col(props[k])
    p["props"] = props
    cmds.append({"command": "node.create", "as": r, "params": p})
    return r


def setp(r, **props):
    for k in ("fill", "stroke"):
        if k in props:
            props[k] = col(props[k])
    cmd("node.set", ids=[ref(r)], **props)


def F(parent, name="Frame", d="v", gap=0, pad=0, fill="none", w=None, h=None, sh=None, sv=None,
      main=None, cross=None, r=None, stroke=None, sw=1, clip=None, x=None, y=None, **extra):
    props = {"fill": fill}
    if d:
        props.update(layout="horizontal" if d == "h" else "vertical", gap=gap, padding=pad)
        if main: props["mainAlign"] = main
        if cross: props["crossAlign"] = cross
    props["sizingH"] = sh or ("fixed" if w is not None else "hug")
    props["sizingV"] = sv or ("fixed" if h is not None else "hug")
    if r is not None: props["cornerRadius"] = r
    if stroke:
        props.update(stroke=stroke, strokeWeight=sw, strokeAlign="inside")
    if clip is not None: props["clip"] = clip
    props.update(extra)
    return node("frame", parent, name, x=x, y=y, w=w, h=h, **props)


def T(parent, s, size=13, color=TEXT, weight=None, fill_w=False, name=None, w=None, h=None, x=None, y=None, **extra):
    props = {"fontFamily": FONT, "fontSize": size, "fill": color}
    if weight: props["fontStyle"] = weight
    if fill_w:
        props.update(sizingH="fill", textAutoResize="height")
    if w is not None:
        props["textAutoResize"] = "height"
    if h is not None:
        props.update(textAutoResize="none", sizingV="fixed")
    props.update(extra)
    return node("text", parent, name or s[:40], text=s, w=w, h=h, x=x, y=y, **props)


def spacer(parent):
    return F(parent, "Spacer", d=None, h=1, sh="fill")


def inst(parent, comp, props=None, name=None, **nodeprops):
    r = new_ref()
    p = {"component": ref(comp)}
    if parent is not None:
        p["parent"] = ref(parent)
    cmds.append({"command": "instance.create", "as": r, "params": p})
    if props:
        cmd("instance.setProperties", ids=[ref(r)], properties=props)
    if name:
        nodeprops["name"] = name
    if nodeprops:
        setp(r, **nodeprops)
    return r


def tone(r, t):
    cmd("variable.setMode", ids=[ref(r)], collection="Icon tone", mode=t)


# ─────────────────────────── tokens ───────────────────────────

def build_tokens():
    cmd("variable.createCollection", name="Omarchy theme", modes=["Dark", "Light"])
    for name, (d, l) in TOKENS.items():
        cmd("variable.create", collection="Omarchy theme", name=name, type="color", values={"Dark": d, "Light": l})
    cmd("variable.createCollection", name="Icon tone", modes=list(TONES))
    cmd("variable.create", collection="Icon tone", name="icon/stroke", type="color",
        values={m: {"alias": v} for m, v in TONES.items()})


# ─────────────────────────── components ───────────────────────────

def heading(s, x, y, desc=None):
    T(None, s, size=16, color=TEXT, weight="SemiBold", x=x, y=y)
    if desc:
        T(None, desc, size=11, color=MUTED, x=x, y=y + 26, w=900)


def make_component(frame):
    cmd("component.create", ids=[ref(frame)])
    return frame


def variant_set(name, variants, props, x, y, gap=24, per_row=6, row_h=60):
    """variants: [(variant_name, build(frame_ref) -> {layer_key: ref})]; props: [(name, type, key, field)]."""
    built = []
    for i, (vname, build) in enumerate(variants):
        fr, layers = build(vname, x + (i % per_row) * 0, y)
        built.append((fr, layers))
        make_component(fr)
    s = new_ref()
    cmds.append({"command": "component.combineVariants", "as": s, "params": {"ids": [ref(f) for f, _ in built]}})
    setp(s, name=name, layout="horizontal", wrap=True, gap=gap, crossGap=16, padding=20, sizingH="hug",
         sizingV="hug", x=x, y=y, fill="none", stroke=LINE, strokeWeight=1, dashes=[4, 4], cornerRadius=10,
         maxWidth=1500)
    for pname, ptype, key, field in props:
        first = built[0][1][key]
        p = {"id": ref(s), "name": pname, "type": ptype, "layer": ref(first)}
        cmd("component.addProperty", **p)
        for fr, layers in built[1:]:
            cmd("component.bindProperty", layer=ref(layers[key]), property=pname, field=field)
    C[name] = s
    return s


def build_components(x0=0, y0=0):
    T(None, "Компоненты", size=36, color=TEXT, weight="Bold", x=x0, y=y0 - 120)
    T(None, "Все цвета привязаны к коллекции «Omarchy theme» (режимы Dark / Light). Цвет иконки задаётся режимом "
            "коллекции «Icon tone» на экземпляре: Muted, Text, Faint, Accent, OnAccent, Danger, Warn.",
      size=13, color=MUTED, x=x0, y=y0 - 66, w=1100, lineHeight="150%")

    # icons
    heading("Иконки · 16 px, линия 1.5", x0, y0)
    for i, (k, path) in enumerate(ICONS.items()):
        fr = F(None, f"Icon/{k}", d=None, w=16, h=16, x=x0 + (i % 16) * 56, y=y0 + 50 + (i // 16) * 56)
        node("vector", fr, k, path=path, x=0, y=0, fill="none", stroke={"$var": "icon/stroke"}, strokeWeight=1.5,
             strokeCap="round", strokeJoin="round")
        make_component(fr)
        C[f"Icon/{k}"] = fr
    y = y0 + 260

    def icon_in(parent, key, t):
        r = inst(parent, C[f"Icon/{key}"], name="Icon")
        tone(r, t)
        return r

    # Button
    heading("Button", x0, y, "Kind × Size. Свойства: Label, Has icon, Icon (подмена). Один Primary на экран.")
    kinds = {  # fill, text, icon tone, stroke
        "Primary": (ACC, INK, "OnAccent", None),
        "Subtle": (SURF2, TEXT, "Muted", None),
        "Ghost": ("none", MUTED, "Muted", None),
        "Outline": ("none", TEXT, "Text", LINE),
        "Danger": ("none", DANGER, "Danger", None),
    }
    variants = []
    for size, h, fs in (("MD", 34, 13), ("SM", 28, 12)):
        for kind, (fill, tc, it, st) in kinds.items():
            def b(vname, x, yy, fill=fill, tc=tc, it=it, st=st, h=h, fs=fs, kind=kind):
                fr = F(None, vname, d="h", gap=7, pad=[0, 13, 0, 10], h=h, r=8, fill=fill, cross="center", stroke=st,
                       x=x, y=yy)
                ic = icon_in(fr, "copy", it)
                lb = T(fr, "Копировать", size=fs, color=tc, weight="SemiBold" if kind == "Primary" else None,
                       name="Label")
                return fr, {"icon": ic, "label": lb}
            variants.append((f"Kind={kind}, Size={size}", b))
    variant_set("Button", variants, [("Label", "text", "label", "characters"),
                                     ("Has icon", "boolean", "icon", "visible"),
                                     ("Icon", "instanceSwap", "icon", "mainComponent")], x0, y + 50)
    y += 220

    heading("IconButton", x0, y, "Квадратная кнопка с одной иконкой; подпись — в подсказке.")
    variants = []
    for size, h in (("MD", 34), ("SM", 28)):
        for kind, (fill, it, st) in {"Ghost": ("none", "Muted", None), "Subtle": (SURF2, "Muted", None),
                                     "Outline": ("none", "Text", LINE)}.items():
            def b(vname, x, yy, fill=fill, it=it, st=st, h=h):
                fr = F(None, vname, d="h", w=h, h=h, r=8, fill=fill, main="center", cross="center", stroke=st, x=x,
                       y=yy)
                return fr, {"icon": icon_in(fr, "close", it)}
            variants.append((f"Kind={kind}, Size={size}", b))
    variant_set("IconButton", variants, [("Icon", "instanceSwap", "icon", "mainComponent")], x0, y + 50)
    y += 190

    heading("Key · Badge · Toggle · Radio", x0, y)
    k = F(None, "Key", d="h", pad=[0, 5, 0, 5], h=19, r=4, fill=SURF2, stroke=LINE, cross="center", main="center",
          x=x0, y=y + 50)
    kl = T(k, "Ctrl", size=10, color=MUTED, name="Label")
    make_component(k)
    cmd("component.addProperty", id=ref(k), name="Label", type="text", layer=ref(kl))
    C["Key"] = k
    variants = []
    for t, (c, bg) in {"Accent": (ACC, ACC_SOFT), "Neutral": (MUTED, OVERLAY), "Warn": (WARN, WARN_TINT)}.items():
        def b(vname, x, yy, c=c, bg=bg):
            fr = F(None, vname, d="h", pad=[2, 6, 2, 6], r=5, fill=bg, cross="center", x=x, y=yy)
            return fr, {"label": T(fr, "авто", size=10, color=c, name="Label")}
        variants.append((f"Tone={t}", b))
    variant_set("Badge", variants, [("Label", "text", "label", "characters")], x0 + 120, y + 30)
    variants = []
    for st in ("On", "Off"):
        def b(vname, x, yy, on=st == "On"):
            fr = F(None, vname, d=None, w=34, h=20, r=10, fill=ACC if on else SURF2, stroke=None if on else LINE,
                   x=x, y=yy)
            node("ellipse", fr, "Knob", x=17 if on else 3, y=3, w=14, h=14, fill=INK if on else FAINT)
            return fr, {}
        variants.append((f"State={st}", b))
    variant_set("Toggle", variants, [], x0 + 420, y + 30)
    variants = []
    for st in ("On", "Off"):
        def b(vname, x, yy, on=st == "On"):
            fr = F(None, vname, d="h", w=18, h=18, r=9, fill="none", stroke=ACC if on else FAINT, sw=1.5,
                   main="center", cross="center", x=x, y=yy)
            if on:
                node("ellipse", fr, "Dot", w=8, h=8, fill=ACC)
            return fr, {}
        variants.append((f"State={st}", b))
    variant_set("Radio", variants, [], x0 + 600, y + 30)
    y += 170

    heading("Tab · Segment · NavItem", x0, y)
    variants = []
    for st in ("Active", "Default"):
        def b(vname, x, yy, on=st == "Active"):
            fr = F(None, vname, d="h", gap=6, pad=[0, 12, 0, 12], h=26, r=6, fill=SURF2 if on else "none",
                   cross="center", x=x, y=yy)
            lb = T(fr, "Перевод", size=12, color=TEXT if on else MUTED, weight="SemiBold" if on else None,
                   name="Label")
            ct = T(fr, "24", size=10, color=FAINT, name="Count")
            return fr, {"label": lb, "count": ct}
        variants.append((f"State={st}", b))
    variant_set("Tab", variants, [("Label", "text", "label", "characters"), ("Count", "text", "count", "characters"),
                                  ("Has count", "boolean", "count", "visible")], x0, y + 50)
    variants = []
    for st in ("Active", "Default"):
        def b(vname, x, yy, on=st == "Active"):
            fr = F(None, vname, d="h", pad=[0, 10, 0, 10], h=26, r=6, fill=SURF2 if on else "none", cross="center",
                   x=x, y=yy)
            return fr, {"label": T(fr, "Перевод", size=12, color=TEXT if on else MUTED,
                                   weight="SemiBold" if on else None, name="Label")}
        variants.append((f"State={st}", b))
    variant_set("Segment", variants, [("Label", "text", "label", "characters")], x0 + 340, y + 50)
    variants = []
    for st in ("Active", "Default"):
        def b(vname, x, yy, on=st == "Active"):
            fr = F(None, vname, d="h", gap=10, pad=[0, 10, 0, 10], h=34, w=200, r=7, fill=SURF2 if on else "none",
                   cross="center", x=x, y=yy)
            ic = icon_in(fr, "globe", "Accent" if on else "Muted")
            lb = T(fr, "Сервисы", size=13, color=TEXT if on else MUTED, weight="SemiBold" if on else None,
                   name="Label")
            return fr, {"icon": ic, "label": lb}
        variants.append((f"State={st}", b))
    variant_set("NavItem", variants, [("Label", "text", "label", "characters"),
                                      ("Icon", "instanceSwap", "icon", "mainComponent")], x0 + 640, y + 50)
    y += 170

    heading("LanguageChip · Field", x0, y)
    variants = []
    for st in ("Default", "Open"):
        def b(vname, x, yy, op=st == "Open"):
            fr = F(None, vname, d="h", gap=8, pad=[0, 10, 0, 12], h=34, r=8, fill=SURF2 if op else SURF,
                   cross="center", stroke=ACC if op else None, x=x, y=yy)
            lb = T(fr, "Английский", size=13, color=TEXT, name="Label")
            bd = F(fr, "Badge", d="h", pad=[2, 6, 2, 6], r=5, fill=ACC_SOFT, cross="center")
            bt = T(bd, "авто", size=10, color=ACC, name="Badge text")
            ic = icon_in(fr, "chev", "Muted")
            return fr, {"label": lb, "badge": bd, "bt": bt}
        variants.append((f"State={st}", b))
    variant_set("LanguageChip", variants, [("Label", "text", "label", "characters"),
                                           ("Badge", "text", "bt", "characters"),
                                           ("Has badge", "boolean", "badge", "visible")], x0, y + 50)
    variants = []
    for st, (tc, stc) in {"Filled": (TEXT, LINE), "Placeholder": (FAINT, LINE), "Focus": (TEXT, ACC)}.items():
        def b(vname, x, yy, tc=tc, stc=stc):
            fr = F(None, vname, d="h", gap=8, pad=[0, 12, 0, 12], h=34, w=260, r=7, fill=BG, stroke=stc,
                   cross="center", x=x, y=yy)
            ic = icon_in(fr, "search", "Muted")
            lb = T(fr, "http://localhost:5000", size=12, color=tc, name="Value")
            return fr, {"icon": ic, "label": lb}
        variants.append((f"State={st}", b))
    variant_set("Field", variants, [("Value", "text", "label", "characters"),
                                    ("Has icon", "boolean", "icon", "visible"),
                                    ("Icon", "instanceSwap", "icon", "mainComponent")], x0 + 440, y + 50)


# ─────────────────────────── primitives (instances) ───────────────────────────

def I(parent, key, color=MUTED, size=16, sw=1.5):
    r = inst(parent, C[f"Icon/{key}"], name=f"icon/{key}")
    tone(r, TONE_OF.get(color, "Muted"))
    return r


def kbd(parent, *keys, size=11):
    g = F(parent, "Keys", d="h", gap=3, cross="center")
    for k in keys:
        inst(g, C["Key"], {"Label": k})
    return g


def btn(parent, label=None, icon=None, kind="subtle", size="md", name=None, tip=None, icon_color=None, **np):
    if kind == "ghost" and icon_color == DANGER:
        kind = "danger"
    S = "MD" if size == "md" else "SM"
    if label is None:
        k = {"ghost": "Ghost", "subtle": "Subtle", "outline": "Outline"}.get(kind, "Ghost")
        p = {"Kind": k, "Size": S, "Icon": ref(C[f"Icon/{icon}"])}
        return inst(parent, C["IconButton"], p, name=name or f"IconButton/{icon}", **np)
    p = {"Kind": kind.capitalize(), "Size": S, "Label": label, "Has icon": bool(icon)}
    if icon:
        p["Icon"] = ref(C[f"Icon/{icon}"])
    return inst(parent, C["Button"], p, name=name or f"Button/{label}", **np)


def badge(parent, s, color=ACC, bg=None, size=10):
    t = {ACC: "Accent", MUTED: "Neutral", WARN: "Warn"}.get(color, "Neutral")
    return inst(parent, C["Badge"], {"Tone": t, "Label": s}, name=f"Badge/{s}")


def toggle(parent, on=True):
    return inst(parent, C["Toggle"], {"State": "On" if on else "Off"})


def radio(parent, on):
    return inst(parent, C["Radio"], {"State": "On" if on else "Off"})


def tab(parent, label, active, count=None):
    p = {"State": "Active" if active else "Default", "Label": label, "Has count": bool(count)}
    if count:
        p["Count"] = count
    return inst(parent, C["Tab"], p, name=f"Tab/{label}")


def segment(parent, label, active):
    return inst(parent, C["Segment"], {"State": "Active" if active else "Default", "Label": label},
                name=f"Segment/{label}")


def field(parent, value, state="Filled", icon=None, fill=None, **np):
    p = {"State": state, "Value": value, "Has icon": bool(icon)}
    if icon:
        p["Icon"] = ref(C[f"Icon/{icon}"])
    if fill:
        np["fill"] = fill
    return inst(parent, C["Field"], p, name=f"Field/{value[:20]}", **np)


def nav_item(parent, icon, label, active):
    return inst(parent, C["NavItem"], {"State": "Active" if active else "Default", "Label": label,
                                       "Icon": ref(C[f"Icon/{icon}"])}, name=f"NavItem/{label}", sizingH="fill")


def lang_chip(parent, lang, auto=False, open_=False, name="Language"):
    p = {"State": "Open" if open_ else "Default", "Label": lang, "Has badge": bool(auto)}
    if auto:
        p["Badge"] = auto
    return inst(parent, C["LanguageChip"], p, name=f"LanguageChip/{name}")


def grp(parent, s, pad=[6, 10, 2, 10]):
    g = F(parent, "Group/" + s, d="h", pad=pad, sh="fill")
    T(g, s, size=10, color=FAINT, letterSpacing="6%")
    return g
