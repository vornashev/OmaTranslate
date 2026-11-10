# Дизайн OmaTranslate

`../OmaTranslate.figcraft` — макет. Страницы: «Редизайн · Dark», «Редизайн · Light», «Компоненты», «Иконка», «As-is · текущий плагин».

- `build_design.py` собирает страницы редизайна, компонентов и иконки из `ds.py` (токены, иконки, компоненты) и `screens.py` (экраны):

      python3 design/build_design.py /tmp/r.json
      figcraft-cli run /tmp/r.json --out /tmp/redesign.figcraft

  Затем импортировать `/tmp/redesign.figcraft` в макет (File ▸ Import). После импорта режим Light у страницы «Редизайн · Light» нужно выставить заново.
- `preview_states.py OUT_DIR` рендерит настоящий QML плагина во всех состояниях, в тёмной и светлой теме (нужен PySide6). Состояния описаны в `tests/visual_states.py`; их же проверяет визуальная регрессия `tests/visual_regression.py` (эталоны в `tests/visual/`, обновить: `--update`).
- `icons/a.svg` — иконка приложения (`pkgbuild/omatranslate.svg`), `b.svg` и `c.svg` — отклонённые варианты.
