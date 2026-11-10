#!/usr/bin/env python3
"""Build the OmaTranslate redesign: tokens, components, dark + light screens, icon page."""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from ds import *  # noqa: E402,F403

build_tokens()
cmd("page.rename", name="Редизайн · Dark")
cmd("page.setBackground", color="#0B0E10")
cmd("page.add", as_="pg_comp", name="Компоненты")
cmd("page.select", id=ref("pg_comp"))
cmd("page.setBackground", id=ref("pg_comp"), color="#0B0E10")
build_components()
cmd("page.select", index=0)
exec(open(os.path.join(HERE, "screens.py")).read())

# Light: the same layers with the theme collection switched to Light.
cmd("page.duplicate", as_="pg_light")
cmd("page.rename", id=ref("pg_light"), name="Редизайн · Light")
cmd("variable.setMode", ids=[ref("pg_light")], collection="Omarchy theme", mode="Light")
cmd("page.setBackground", id=ref("pg_light"), color="#DCDBD3")
cmd("page.move", id=ref("pg_light"), index=1)

# Icon page
cmd("page.add", as_="pg_icon", name="Иконка")
cmd("page.select", id=ref("pg_icon"))
cmd("page.setBackground", id=ref("pg_icon"), color="#0B0E10")
T(None, "Иконка", size=36, color=TEXT, weight="Bold", x=0, y=-120)
T(None, "Бар: готовый глиф Nerd Font nf-md-translate (U+F05CA) — тот же набор, что у остальных иконок бара Omarchy, "
        "поэтому он совпадает по размеру и оптическому центру. Приложение (.desktop, лаунчер): своя иконка — "
        "два облачка речи A и Я.", size=13, color=MUTED, w=1100, x=0, y=-66, lineHeight="150%")
for i, (n, title, note) in enumerate((("a", "A · Облачка (выбрано)", "Читается и на 24 px, связана с «A↔Я»."),
                                      ("b", "B · Акцентная плитка", "Ярко, но спорит с иконками других приложений."),
                                      ("c", "C · Моно-рамка", "Строго, на маленьком размере буквы сливаются."))):
    x = i * 360
    T(None, title, size=16, color=TEXT, weight="SemiBold", x=x, y=0)
    T(None, note, size=12, color=MUTED, x=x, y=26, w=320)
    cmd("file.import", path=os.path.join(HERE, "icons", f"{n}.svg"), x=x, y=70)
    cmd("file.import", path=os.path.join(HERE, "icons", f"{n}.svg"), x=x + 100, y=70)
T(None, "Бар Omarchy", size=16, color=TEXT, weight="SemiBold", x=0, y=230)
bar = F(None, "Bar · preview", d="h", gap=0, pad=[0, 8, 0, 8], w=520, h=30, fill=BG, cross="center", x=0, y=270)
for g in ("1", "2", "3"):
    c = F(bar, "Workspace", d="h", w=26, h=30, main="center", cross="center")
    T(c, g, size=12, color=MUTED if g != "1" else TEXT)
spacer(bar)
for g, hl in (("\U000F05CA", True), ("\U000F05A9", False), ("\U000F057E", False), ("\U000F0079", False)):
    c = F(bar, "Item", d="h", w=30, h=26, r=6, main="center", cross="center", fill=SURF2 if hl else "none")
    T(c, g, size=15, color=TEXT, name="Glyph")
T(bar, "  21:40", size=12, color=TEXT)
tip = F(None, "Tooltip", d="v", gap=4, pad=[8, 10, 8, 10], r=6, fill=SURF, stroke=LINE, x=330, y=308)
T(tip, "OmaTranslate", size=12, color=TEXT, weight="SemiBold")
T(tip, "ЛКМ — открыть · СКМ — из буфера · ПКМ — настройки", size=11, color=MUTED)
T(None, "Глиф в шапке окна — тот же знак, что в баре:", size=12, color=MUTED, x=0, y=390)
m = F(None, "Mark", d="h", w=28, h=28, r=7, fill=ACC_SOFT, main="center", cross="center", x=360, y=384)
T(m, "\U000F05CA", size=16, color=ACC)

cmd("page.select", index=0)
json.dump(cmds, open(sys.argv[1], "w"), ensure_ascii=False)
print(len(cmds), "commands")
