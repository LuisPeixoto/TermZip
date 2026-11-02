import importlib.util
import json
import os
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("make_latest_json", os.path.join(os.path.dirname(__file__), "make-latest-json.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def release(directory, version="1.0.0", skip=()):
    for pattern in module.PLATFORMS.values():
        name = pattern.format(v=version)
        if name in skip:
            continue
        open(os.path.join(directory, name), "wb").write(b"binary")
        open(os.path.join(directory, name + ".sig"), "w").write(f"SIGNATURE-OF-{name}\n")


class MakeLatestJson(unittest.TestCase):
    def test_every_platform_gets_its_url_and_its_signature(self):
        with tempfile.TemporaryDirectory() as assets:
            release(assets)

            latest = module.build("1.0.0", "Notes\n", assets, "https://example.com/dl/v1.0.0/", "2026-11-10T00:00:00Z")

        self.assertEqual(latest["version"], "1.0.0")
        self.assertEqual(latest["notes"], "Notes")
        self.assertEqual(latest["pub_date"], "2026-11-10T00:00:00Z")
        self.assertEqual(set(latest["platforms"]), {"darwin-aarch64", "darwin-x86_64", "linux-x86_64", "windows-x86_64"})
        mac = latest["platforms"]["darwin-aarch64"]
        self.assertEqual(mac["url"], "https://example.com/dl/v1.0.0/TermZip_1.0.0_aarch64.app.tar.gz")
        self.assertEqual(mac["signature"], "SIGNATURE-OF-TermZip_1.0.0_aarch64.app.tar.gz")

    def test_the_two_mac_builds_have_different_files(self):
        with tempfile.TemporaryDirectory() as assets:
            release(assets)

            platforms = module.build("1.0.0", "", assets, "https://x", "2026-01-01T00:00:00Z")["platforms"]

        self.assertNotEqual(platforms["darwin-aarch64"]["url"], platforms["darwin-x86_64"]["url"])
        self.assertNotEqual(platforms["darwin-aarch64"]["signature"], platforms["darwin-x86_64"]["signature"])

    def test_a_missing_platform_is_an_error_and_not_left_out(self):
        with tempfile.TemporaryDirectory() as assets:
            release(assets, skip=("TermZip_1.0.0_amd64.AppImage",))

            with self.assertRaises(SystemExit) as caught:
                module.build("1.0.0", "", assets, "https://x")

        self.assertIn("TermZip_1.0.0_amd64.AppImage", str(caught.exception))

    def test_a_file_without_its_signature_is_an_error(self):
        with tempfile.TemporaryDirectory() as assets:
            release(assets)
            os.remove(os.path.join(assets, "TermZip_1.0.0_x64-setup.exe.sig"))

            with self.assertRaises(SystemExit):
                module.build("1.0.0", "", assets, "https://x")

    def test_the_output_is_json_the_updater_can_read(self):
        with tempfile.TemporaryDirectory() as assets:
            release(assets)
            notes = os.path.join(assets, "notes.md")
            open(notes, "w").write("What is new\n")
            out = os.path.join(assets, "latest.json")
            import contextlib, io

            buffer = io.StringIO()
            with contextlib.redirect_stdout(buffer):
                module.main(["--version", "1.0.0", "--notes-file", notes, "--assets", assets, "--base-url", "https://x", "--date", "2026-11-10T00:00:00Z"])
            latest = json.loads(buffer.getvalue())

        self.assertEqual(latest["notes"], "What is new")
        self.assertTrue(all(p["url"].startswith("https://x/") for p in latest["platforms"].values()))


if __name__ == "__main__":
    unittest.main()
