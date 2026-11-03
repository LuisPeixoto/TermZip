#!/usr/bin/env python3
"""Checks the website build: every language is complete, and every page carries what search engines and readers need."""
import json
import os
import re
import subprocess
import sys
import unittest
from html.parser import HTMLParser

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIST = os.path.join(ROOT, "site", "dist")
I18N = os.path.join(ROOT, "site", "src", "i18n")
LANGS = {"en": "", "es": "es", "pt-br": "pt-br", "zh": "zh"}
URL = "https://example.test/TermZip"


def shape(value):
    """The keys and list lengths of a translation: what every language must share."""
    if isinstance(value, dict):
        return {key: shape(item) for key, item in value.items()}
    if isinstance(value, list):
        return [shape(item) for item in value]
    return None


class Page(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.tags = []
        self.h1 = 0
        self.ids = set()
        self.links = []
        self.imgs = []
        self.meta = {}
        self.canonical = None
        self.alternates = {}
        self.title = ""
        self.jsonld = []
        self._in_title = False
        self._in_ld = False
        self.lang = None

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if tag == "html":
            self.lang = a.get("lang")
        if tag == "h1":
            self.h1 += 1
        if "id" in a:
            self.ids.add(a["id"])
        if tag == "a" and "href" in a:
            self.links.append(a["href"])
        if tag == "img":
            self.imgs.append(a)
        if tag == "title":
            self._in_title = True
        if tag == "meta":
            key = a.get("name") or a.get("property")
            if key:
                self.meta[key] = a.get("content", "")
        if tag == "link" and a.get("rel") == "canonical":
            self.canonical = a["href"]
        if tag == "link" and a.get("rel") == "alternate" and "hreflang" in a:
            self.alternates[a["hreflang"]] = a["href"]
        if tag == "script" and a.get("type") == "application/ld+json":
            self._in_ld = True
            self.jsonld.append("")

    def handle_endtag(self, tag):
        if tag == "title":
            self._in_title = False
        if tag == "script":
            self._in_ld = False

    def handle_data(self, data):
        if self._in_title:
            self.title += data
        if self._in_ld:
            self.jsonld[-1] += data


def read(lang):
    folder = LANGS[lang]
    path = os.path.join(DIST, folder, "index.html") if folder else os.path.join(DIST, "index.html")
    text = open(path, encoding="utf-8").read()
    page = Page()
    page.feed(text)
    return text, page


class SiteTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        env = dict(os.environ, SITE_URL=URL)
        subprocess.run([sys.executable, os.path.join(ROOT, "tools", "build-site.py")], check=True, env=env, stdout=subprocess.DEVNULL)
        cls.strings = {lang: json.load(open(os.path.join(I18N, f"{lang}.json"), encoding="utf-8")) for lang in LANGS}

    def test_every_language_has_the_same_keys(self):
        reference = shape(self.strings["en"])
        for lang, strings in self.strings.items():
            self.assertEqual(shape(strings), reference, lang)

    def test_every_string_is_filled_in(self):
        def walk(value, where):
            if isinstance(value, dict):
                for key, item in value.items():
                    if key in ("tag", "lines"):
                        continue  # a tag may be empty, and Linux has no command line
                    walk(item, f"{where}.{key}")
            elif isinstance(value, list):
                for index, item in enumerate(value):
                    walk(item, f"{where}[{index}]")
            elif isinstance(value, str):
                self.assertTrue(value.strip(), f"empty string at {where}")
        for lang, strings in self.strings.items():
            walk(strings, lang)

    def test_titles_and_descriptions_fit_a_search_result(self):
        for lang in LANGS:
            _, page = read(lang)
            self.assertLessEqual(len(page.title), 60, f"{lang} title is {len(page.title)} characters")
            self.assertGreaterEqual(len(page.title), 15, lang)
            description = page.meta["description"]
            self.assertLessEqual(len(description), 160, f"{lang} description is {len(description)} characters")
            self.assertGreaterEqual(len(description), 70, lang)
        titles = {read(lang)[1].title for lang in LANGS}
        self.assertEqual(len(titles), len(LANGS), "every language needs its own title")

    def test_each_page_has_one_h1_a_language_and_a_canonical_address(self):
        for lang, folder in LANGS.items():
            _, page = read(lang)
            self.assertEqual(page.h1, 1, lang)
            self.assertEqual(page.lang, self.strings[lang]["html_lang"])
            expected = URL + "/" + (folder + "/" if folder else "")
            self.assertEqual(page.canonical, expected)
            self.assertEqual(page.meta["og:url"], expected)
            self.assertTrue(page.meta["og:image"].startswith(URL + "/img/og-"))
            self.assertEqual(page.meta["twitter:card"], "summary_large_image")

    def test_hreflang_is_complete_and_reciprocal(self):
        wanted = {self.strings[lang]["hreflang"]: URL + "/" + (LANGS[lang] + "/" if LANGS[lang] else "") for lang in LANGS}
        wanted["x-default"] = URL + "/"
        for lang in LANGS:
            _, page = read(lang)
            self.assertEqual(page.alternates, wanted, lang)

    def test_structured_data_parses_and_describes_the_app(self):
        for lang in LANGS:
            _, page = read(lang)
            self.assertEqual(len(page.jsonld), 1)
            graph = json.loads(page.jsonld[0])["@graph"]
            kinds = {item["@type"] for item in graph}
            self.assertEqual(kinds, {"SoftwareApplication", "WebSite", "WebPage", "FAQPage"})
            app = next(item for item in graph if item["@type"] == "SoftwareApplication")
            self.assertEqual(app["offers"]["price"], "0")
            self.assertNotIn("aggregateRating", app, "no ratings: there are none")
            faq = next(item for item in graph if item["@type"] == "FAQPage")
            self.assertEqual(len(faq["mainEntity"]), len(self.strings[lang]["faq"]))

    def test_nothing_of_the_template_is_left(self):
        for lang in LANGS:
            text, _ = read(lang)
            self.assertNotIn("{%", text, lang)
            leftover = re.findall(r"\{\{[^}]*\}\}", text)
            allowed = [w for w in re.findall(r"\{\{[^}]*\}\}", json.dumps(self.strings[lang], ensure_ascii=False).replace("\\", ""))]
            self.assertLessEqual(len(leftover), len(allowed), f"{lang}: {leftover}")

    def test_images_and_anchors_exist(self):
        for lang in LANGS:
            _, page = read(lang)
            for img in page.imgs:
                src = img["src"]
                local = src[len(URL.split("/TermZip")[0]):] if src.startswith("http") else src
                local = local.replace("/TermZip", "", 1) if local.startswith("/TermZip") else local
                self.assertTrue(os.path.exists(os.path.join(DIST, local.lstrip("/"))), f"{lang}: {src}")
                self.assertTrue(img.get("alt") is not None, f"{lang}: {src} has no alt")
                self.assertTrue(img.get("width") and img.get("height"), f"{lang}: {src} has no size")
            for link in page.links:
                if link.startswith("#"):
                    self.assertIn(link[1:], page.ids, f"{lang}: {link}")

    def test_the_site_files_exist(self):
        sitemap = open(os.path.join(DIST, "sitemap.xml"), encoding="utf-8").read()
        for folder in LANGS.values():
            self.assertIn(f"<loc>{URL}/{folder + '/' if folder else ''}</loc>", sitemap)
        self.assertIn("hreflang=\"zh-CN\"", sitemap)
        robots = open(os.path.join(DIST, "robots.txt"), encoding="utf-8").read()
        self.assertIn(f"Sitemap: {URL}/sitemap.xml", robots)
        for name in ("404.html", "favicon.svg", "site.webmanifest", ".nojekyll", "styles.css", "main.js"):
            self.assertTrue(os.path.exists(os.path.join(DIST, name)), name)

    def test_the_install_commands_are_the_ones_of_the_docs(self):
        text, _ = read("en")
        self.assertIn("brew install --cask luispeixoto/termzip/termzip", text)
        self.assertIn("scoop bucket add termzip https://github.com/LuisPeixoto/scoop-termzip", text)


if __name__ == "__main__":
    unittest.main()
