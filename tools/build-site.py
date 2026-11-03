#!/usr/bin/env python3
"""Builds the TermZip website into site/dist, one page per language.

    python3 tools/build-site.py                      # BASE_URL defaults to the GitHub Pages address
    SITE_URL=https://termzip.dev python3 tools/build-site.py

The template is site/src/index.html.tmpl, the words are site/src/i18n/<lang>.json, and everything else in site/src
(styles, script, fonts, images) is copied as it is. Standard library only, so it runs in CI without installing anything.

Template language (small on purpose): {{name}} (HTML-escaped), {{{name}}} (raw), {% for x in list %}...{% endfor %}
and {% if x %}...{% endif %}. Names may be dotted (item.field).
"""
import datetime
import hashlib
import html
import json
import os
import re
import shutil
import sys
from urllib.parse import urlparse

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "site", "src")
DIST = os.path.join(ROOT, "site", "dist")
SITE_URL = os.environ.get("SITE_URL", "https://luispeixoto.github.io/TermZip").rstrip("/")
REPO = "https://github.com/LuisPeixoto/TermZip"

# language id -> folder ("" is the root). English is the default and lives at the root.
LANGS = {"en": "", "es": "es", "pt-br": "pt-br", "zh": "zh"}
WORD = "TERMZIP"

TOKEN = re.compile(r"(\{%.*?%\}|\{\{\{.*?\}\}\}|\{\{.*?\}\})", re.S)


class TemplateError(Exception):
    pass


def parse(template):
    """A tree of ('text', s), ('var', name, raw), ('for', var, list, children), ('if', expr, children)."""
    tokens = TOKEN.split(template)
    stack = [[]]
    openers = []
    for token in tokens:
        if token.startswith("{{{") and token.endswith("}}}"):
            stack[-1].append(("var", token[3:-3].strip(), True))
        elif token.startswith("{{") and token.endswith("}}"):
            stack[-1].append(("var", token[2:-2].strip(), False))
        elif token.startswith("{%"):
            body = token[2:-2].strip()
            words = body.split()
            if words[0] == "for":
                if len(words) != 4 or words[2] != "in":
                    raise TemplateError(f"bad for: {body}")
                openers.append(("for", words[1], words[3]))
                stack.append([])
            elif words[0] == "if":
                openers.append(("if", " ".join(words[1:])))
                stack.append([])
            elif words[0] in ("endfor", "endif"):
                if not openers:
                    raise TemplateError(f"unexpected {body}")
                opener = openers.pop()
                children = stack.pop()
                if (opener[0], words[0]) not in (("for", "endfor"), ("if", "endif")):
                    raise TemplateError(f"{words[0]} closes a {opener[0]}")
                stack[-1].append((*opener, children))
            else:
                raise TemplateError(f"unknown tag: {body}")
        elif token:
            stack[-1].append(("text", token))
    if openers:
        raise TemplateError(f"unclosed {openers[-1][0]}")
    return stack[0]


def lookup(scope, dotted):
    value = scope
    for part in dotted.split("."):
        if isinstance(value, dict) and part in value:
            value = value[part]
        else:
            raise TemplateError(f"missing value: {dotted}")
    return value


def scope_get(scopes, dotted):
    head = dotted.split(".")[0]
    for scope in reversed(scopes):
        if head in scope:
            return lookup(scope, dotted)
    raise TemplateError(f"missing value: {dotted}")


def render(nodes, scopes):
    out = []
    for node in nodes:
        kind = node[0]
        if kind == "text":
            out.append(node[1])
        elif kind == "var":
            value = scope_get(scopes, node[1])
            text = value if isinstance(value, str) else str(value)
            out.append(text if node[2] else html.escape(text, quote=True))
        elif kind == "for":
            for item in scope_get(scopes, node[2]):
                out.append(render(node[3], scopes + [{node[1]: item}]))
        elif kind == "if":
            try:
                value = scope_get(scopes, node[1])
            except TemplateError:
                value = False
            if value:
                out.append(render(node[2], scopes))
    return "".join(out)


def workspace_version():
    with open(os.path.join(ROOT, "Cargo.toml"), encoding="utf-8") as handle:
        match = re.search(r'^\[workspace\.package\].*?^version = "([^"]+)"', handle.read(), re.S | re.M)
    if not match:
        raise SystemExit("cannot read the workspace version from Cargo.toml")
    return match.group(1)


def load_language(lang, version):
    with open(os.path.join(SRC, "i18n", f"{lang}.json"), encoding="utf-8") as handle:
        text = handle.read()
    return json.loads(text.replace("{{version}}", version))


def page_path(lang):
    folder = LANGS[lang]
    return f"/{folder}/" if folder else "/"


def url_for(lang):
    return SITE_URL + ("/" + LANGS[lang] + "/" if LANGS[lang] else "/")


def json_ld(strings, lang, version):
    base = SITE_URL
    app = {
        "@type": "SoftwareApplication",
        "@id": base + "/#app",
        "name": "TermZip",
        "alternateName": "TermZip terminal",
        "description": strings["meta_description"],
        "applicationCategory": "DeveloperApplication",
        "applicationSubCategory": "Terminal emulator",
        "operatingSystem": "macOS, Windows, Linux",
        "softwareVersion": version,
        "license": "https://opensource.org/licenses/MIT",
        "isAccessibleForFree": True,
        "offers": {"@type": "Offer", "price": "0", "priceCurrency": "USD"},
        "downloadUrl": REPO + "/releases/latest",
        "installUrl": REPO + "/releases/latest",
        "codeRepository": REPO,
        "programmingLanguage": "Rust",
        "inLanguage": [strings["hreflang"]],
        "image": base + "/img/dracula-1600.jpg",
        "screenshot": [base + "/img/dracula-1600.jpg", base + "/img/themes-1600.jpg", base + "/img/plugin-catalog-1600.jpg"],
        "author": {"@type": "Person", "name": "Luis Peixoto", "url": "https://github.com/LuisPeixoto"},
        "featureList": [feature["name"] for feature in strings["features"]],
    }
    site = {
        "@type": "WebSite",
        "@id": base + "/#site",
        "url": base + "/",
        "name": "TermZip",
        "inLanguage": strings["hreflang"],
    }
    page = {
        "@type": "WebPage",
        "@id": url_for(lang) + "#page",
        "url": url_for(lang),
        "name": strings["meta_title"],
        "description": strings["meta_description"],
        "inLanguage": strings["hreflang"],
        "isPartOf": {"@id": base + "/#site"},
        "about": {"@id": base + "/#app"},
    }
    faq = {
        "@type": "FAQPage",
        "@id": url_for(lang) + "#faq",
        "mainEntity": [
            {"@type": "Question", "name": item["q"], "acceptedAnswer": {"@type": "Answer", "text": item["a"]}}
            for item in strings["faq"]
        ],
    }
    # `</` must not end the script element.
    return json.dumps({"@context": "https://schema.org", "@graph": [app, site, page, faq]}, ensure_ascii=False).replace("</", "<\\/")


def build_page(template, lang, version, all_strings, asset_version):
    strings = dict(all_strings[lang])
    base = urlparse(SITE_URL).path.rstrip("/")
    alternates = []
    for other, folder in LANGS.items():
        other_strings = all_strings[other]
        alternates.append(
            {
                "lang": other,
                "href": url_for(other),
                "hreflang": other_strings["hreflang"],
                "html_lang": other_strings["html_lang"],
                "og_locale": other_strings["og_locale"],
                "name": other_strings["lang_name"],
                "current": other == lang,
                "other": other != lang,
            }
        )
    installs = []
    for index, item in enumerate(strings["installs"]):
        installs.append({**item, "selected": "true" if index == 0 else "false", "tabindex": "0" if index == 0 else "-1"})
    og_image = f"{SITE_URL}/img/og-{lang}.png"
    scope = {
        **strings,
        "lang": lang,
        "base": base,
        "version": version,
        "canonical": url_for(lang),
        "default_href": url_for("en"),
        "home_href": base + page_path(lang),
        "alternates": alternates,
        "installs": installs,
        "word": list(WORD),
        "og_image": og_image,
        "asset_version": asset_version,
        "jsonld": json_ld(strings, lang, version),
        "offers_json": json.dumps(
            {
                "offers": {key: value for key, value in strings["offers"].items() if key != lang},
                "dismiss": strings["offer_dismiss"],
                "paths": {key: base + page_path(key) for key in LANGS},
            },
            ensure_ascii=False,
        ),
    }
    return render(parse(template), [scope])


def sitemap(lastmod):
    lines = ['<?xml version="1.0" encoding="UTF-8"?>', '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">']
    hreflangs = {lang: json.load(open(os.path.join(SRC, "i18n", f"{lang}.json"), encoding="utf-8"))["hreflang"] for lang in LANGS}
    for lang in LANGS:
        lines.append("  <url>")
        lines.append(f"    <loc>{url_for(lang)}</loc>")
        lines.append(f"    <lastmod>{lastmod}</lastmod>")
        for other in LANGS:
            lines.append(f'    <xhtml:link rel="alternate" hreflang="{hreflangs[other]}" href="{url_for(other)}"/>')
        lines.append(f'    <xhtml:link rel="alternate" hreflang="x-default" href="{url_for("en")}"/>')
        lines.append("  </url>")
    lines.append("</urlset>")
    return "\n".join(lines) + "\n"


def main():
    version = workspace_version()
    template = open(os.path.join(SRC, "index.html.tmpl"), encoding="utf-8").read()
    all_strings = {lang: load_language(lang, version) for lang in LANGS}

    if os.path.isdir(DIST):
        shutil.rmtree(DIST)
    os.makedirs(DIST)
    for entry in ("styles.css", "main.js"):
        shutil.copy(os.path.join(SRC, entry), os.path.join(DIST, entry))
    for folder in ("fonts", "img", "static"):
        source = os.path.join(SRC, folder)
        if os.path.isdir(source):
            shutil.copytree(source, DIST if folder == "static" else os.path.join(DIST, folder), dirs_exist_ok=True)

    digest = hashlib.sha256()
    for entry in ("styles.css", "main.js"):
        digest.update(open(os.path.join(SRC, entry), "rb").read())
    asset_version = digest.hexdigest()[:10]

    for lang, folder in LANGS.items():
        out_dir = os.path.join(DIST, folder) if folder else DIST
        os.makedirs(out_dir, exist_ok=True)
        with open(os.path.join(out_dir, "index.html"), "w", encoding="utf-8") as handle:
            handle.write(build_page(template, lang, version, all_strings, asset_version))

    today = datetime.date.today().isoformat()
    with open(os.path.join(DIST, "sitemap.xml"), "w", encoding="utf-8") as handle:
        handle.write(sitemap(today))
    with open(os.path.join(DIST, "robots.txt"), "w", encoding="utf-8") as handle:
        handle.write(f"User-agent: *\nAllow: /\n\nSitemap: {SITE_URL}/sitemap.xml\n")
    with open(os.path.join(DIST, ".nojekyll"), "w", encoding="utf-8"):
        pass
    print(f"built {len(LANGS)} pages for {SITE_URL} (version {version}) into {os.path.relpath(DIST, ROOT)}")


if __name__ == "__main__":
    try:
        main()
    except TemplateError as error:
        sys.exit(f"template error: {error}")
