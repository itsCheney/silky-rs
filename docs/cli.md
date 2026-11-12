# Command line

English | [简体中文](cli.zh-CN.md)

## silky-tool

```sh
silky-tool info  <game folder>                  # archives and file counts
silky-tool check <game folder> [--images]       # decode every script, MAP and image
silky-tool dis   <game folder> FK001.MES        # disassemble a script
silky-tool map   <game folder> FK001.MAP --section 1
silky-tool akb   <game folder> TITLE_BG_01.AKB title.png
silky-tool ls      <file.arc>
silky-tool extract <file.arc> <out folder>
```

## silky-run

```sh
silky-run trace <game folder>                        # boot log until the title menu
silky-run trace <game folder> --result 29.110=1      # pick "Start" on the title menu
silky-run trace <game folder> --result 29.110=1 --text-only --lang zh-hans --max-calls 50000
silky-run trace <kimaten2 folder> --result 29.110=1 --result 29.42=1,2 --result 29.43=1 --text-only
```

```sh
silky-run snap <game folder> --out snaps --font DejaVuSans.ttf --lines 0,10,50
silky-run snap <game folder> --lang 1 --font msyh.ttc --lines 170
silky-run snap <game folder> --lines 99999999 --path 0,1,1   # play a route to its ending
```

`snap` runs the game with `silky-runtime` and saves PNG frames of the title screen and of the listed message lines (counted from the first line of the game). Fonts are not part of the game files: pass one or more with `--font`, in fallback order; without a font the text is left out. `--path` gives the options for the first selections in order (later ones take `--choose`); with it the run stops when the game comes back to the title, and the frame held after each movie is saved too. On ふゆから、くるる。 the routes `--path 1` and `--path 0,0` end at the two bad endings, `--path 0,1,0` and `--path 0,1,1` at the true ending with its ending movie and end card. On きまぐれテンプテーション2, `--actions` gives the buttons to take at the first action menus (later ones take the buttons not done yet in turn) and the map goes to the first place not done yet; the first five menus and maps are saved as `actionsN.png` and `mapN.png`. Its investigations need the right answers at their selections, which `--path` and `--choose` do not know, so a run can go round in one.

`trace` performs no engine function; it logs each call with its arguments. Calls whose result the scripts use return 0 unless `--result CLASS.SUB=VALUE` says otherwise; with several values (`29.42=1,1,2`) successive calls get them in turn and the last one stays. Selections (`Game.20`) take `--choose`, counted from 0 (default the first option). `--lang` is `en`, `zh-hans`, `zh-hant` or `ja` (0, 1, 2 still name the `.MAP` sections); a game offers the languages it has text for: ふゆから、くるる。 English and both Chinese, きまぐれテンプテーション2 Japanese. Fan translation patches are not read. On きまぐれテンプテーション2, `29.42` answers the action menu and `29.43` the map.
