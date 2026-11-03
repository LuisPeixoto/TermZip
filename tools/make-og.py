#!/usr/bin/env python3
"""Renders the social cards (1200x630) of the website with a headless Chrome and a throwaway profile.

    python3 tools/make-og.py            # writes site/src/img/og-<lang>.png

The card is the departure board: the split-flap letters of TERMZIP and the title of the page in its language. Run it
when the titles or the design change and commit the images; the site build only copies them.
"""
import html
import json
import os
import shutil
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "site", "src")
CHROME = os.environ.get("CHROME", "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")
LANGS = ["en", "es", "pt-br", "zh"]

PAGE = """<!doctype html><html lang="{lang}"><meta charset="utf-8"><style>
@font-face {{ font-family: BC; font-weight: 700; src: url("file://{fonts}/barlow-condensed-700.woff2"); }}
@font-face {{ font-family: B; font-weight: 400; src: url("file://{fonts}/barlow-400.woff2"); }}
* {{ box-sizing: border-box; }}
body {{ margin: 0; width: 1200px; height: 630px; background: #0b0d10; color: #ece9df; font-family: B, "PingFang SC", "Noto Sans SC", "Microsoft YaHei", sans-serif; padding: 56px 64px; position: relative; overflow: hidden; }}
.sign {{ font: 700 26px/1 BC, sans-serif; letter-spacing: 0.22em; text-transform: uppercase; color: #f4b63c; display: flex; gap: 20px; align-items: center; }}
.sign i {{ flex: 1; height: 3px; background: repeating-linear-gradient(90deg, #f4b63c 0 12px, transparent 12px 20px); opacity: .55; }}
.flaps {{ display: flex; justify-content: space-between; margin: 28px 0 36px; }}
.t {{ width: 140px; height: 190px; border-radius: 10px; background: linear-gradient(#1d222a 50%, #11151a 50%); font: 700 158px/190px BC, sans-serif; text-align: center; position: relative; box-shadow: inset 0 0 0 1px rgba(255,255,255,.06); }}
.t::after {{ content: ""; position: absolute; left: 0; right: 0; top: 94px; height: 3px; background: #05070a; }}
h1 {{ margin: 0; text-wrap: balance; font: 700 58px/1.08 BC, "PingFang SC", "Noto Sans SC", sans-serif; text-transform: uppercase; max-width: 1000px; }}
.url {{ position: absolute; right: 64px; bottom: 44px; font: 700 28px/1 BC, sans-serif; letter-spacing: .12em; color: #8d929b; text-transform: uppercase; }}
.bar {{ position: absolute; left: 0; right: 0; bottom: 0; height: 12px; background: #2a62f0; }}
</style><body>
<div class="sign"><span>{board}</span><i></i></div>
<div class="flaps">{tiles}</div>
<h1>{title}</h1>
<div class="url">termzip</div><div class="bar"></div></body></html>"""


def main():
    if not os.path.exists(CHROME):
        sys.exit(f"Chrome not found at {CHROME}; set CHROME")
    work = tempfile.mkdtemp(prefix="termzip-og-")
    try:
        for lang in LANGS:
            strings = json.load(open(os.path.join(SRC, "i18n", f"{lang}.json"), encoding="utf-8"))
            page = PAGE.format(
                lang=strings["html_lang"],
                fonts=os.path.join(SRC, "fonts"),
                board=html.escape(strings["board_title"]),
                tiles="".join(f'<div class="t">{c}</div>' for c in "TERMZIP"),
                title=html.escape(strings["og_title"]),
            )
            path = os.path.join(work, f"{lang}.html")
            open(path, "w", encoding="utf-8").write(page)
            out = os.path.join(SRC, "img", f"og-{lang}.png")
            subprocess.run(
                [CHROME, "--headless=new", "--disable-gpu", "--hide-scrollbars", f"--user-data-dir={os.path.join(work, 'profile')}",
                 "--window-size=1200,630", "--force-device-scale-factor=1", "--allow-file-access-from-files",
                 "--virtual-time-budget=3000", f"--screenshot={out}", "file://" + path],
                check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            )
            print("wrote", os.path.relpath(out, ROOT))
    finally:
        shutil.rmtree(work, ignore_errors=True)


if __name__ == "__main__":
    main()
