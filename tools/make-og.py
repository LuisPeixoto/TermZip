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
@font-face {{ font-family: BR; font-weight: 800; src: url("file://{fonts}/bricolage-800.woff2"); }}
@font-face {{ font-family: HG; font-weight: 500; src: url("file://{fonts}/hanken-500.woff2"); }}
* {{ box-sizing: border-box; }}
body {{ margin: 0; width: 1200px; height: 630px; background: #5a47f5; color: #fff; font-family: HG, "PingFang SC", "Noto Sans SC", "Microsoft YaHei", sans-serif; padding: 64px 72px; position: relative; overflow: hidden; }}
.c1 {{ position: absolute; width: 520px; height: 520px; border-radius: 50%; background: #5edac8; right: -140px; bottom: -220px; }}
.c2 {{ position: absolute; width: 360px; height: 360px; border-radius: 50%; border: 34px solid rgba(255,255,255,.16); right: 140px; top: -150px; }}
.brand {{ position: relative; display: flex; align-items: center; gap: 16px; font: 800 38px/1 BR, sans-serif; }}
.mark {{ width: 52px; height: 52px; border-radius: 15px; background: #10131b; display: grid; place-items: center; font: 800 26px/1 BR, sans-serif; color: #5edac8; }}
h1 {{ position: relative; margin: 56px 0 0; max-width: 900px; font: 800 70px/1.04 BR, "PingFang SC", "Noto Sans SC", sans-serif; letter-spacing: -0.03em; text-wrap: balance; }}
.url {{ position: absolute; left: 72px; bottom: 56px; font: 500 28px/1 HG, sans-serif; opacity: .85; }}
</style><body>
<div class="c1"></div><div class="c2"></div>
<div class="brand"><span class="mark">&gt;_</span>TermZip</div>
<h1>{title}</h1>
<div class="url">luispeixoto.github.io/TermZip</div></body></html>"""


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
