"""Focused POSIX tests for collision-safe release installation."""

import os
import pathlib
import subprocess
import tempfile
import unittest


SCRIPT = pathlib.Path(__file__).with_name("install_release.sh")


@unittest.skipIf(os.name == "nt", "POSIX installer tests run on Unix")
class InstallReleaseTests(unittest.TestCase):
    def fixture(self, root):
        release = root / "release λ"
        destination = root / "destination λ"
        release.mkdir()
        destination.mkdir()
        source = release / "rt"
        source.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        source.chmod(0o755)
        for name in (
            "LICENSE",
            "THIRD_PARTY_NOTICES.txt",
            "THIRD_PARTY_LICENSES.txt",
            "INSTALL.md",
            "RECOVERY.md",
            "manifest.json",
            "install_release.sh",
            "install_release.ps1",
        ):
            (release / name).write_text("fixture", encoding="utf-8")
        return release, destination

    def invoke(self, release, destination):
        return subprocess.run(
            ["/bin/sh", os.fspath(SCRIPT), os.fspath(release), os.fspath(destination)],
            capture_output=True,
            text=True,
            check=False,
        )

    def test_installs_regular_executable_with_spaces_and_unicode(self):
        with tempfile.TemporaryDirectory() as directory:
            release, destination = self.fixture(pathlib.Path(directory))
            result = self.invoke(release, destination)
            self.assertEqual(result.returncode, 0)
            installed = destination / "rt"
            self.assertTrue(installed.is_file())
            self.assertTrue(os.access(installed, os.X_OK))

    def test_existing_destination_is_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            release, destination = self.fixture(pathlib.Path(directory))
            sentinel = destination / "rt"
            sentinel.write_text("unrelated-command", encoding="utf-8")
            result = self.invoke(release, destination)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(sentinel.read_text(encoding="utf-8"), "unrelated-command")

    def test_symlink_source_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            release, destination = self.fixture(root)
            source = release / "rt"
            source.unlink()
            source.symlink_to("missing")
            result = self.invoke(release, destination)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((destination / "rt").exists())

    def test_partial_inventory_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            release, destination = self.fixture(pathlib.Path(directory))
            (release / "manifest.json").unlink()
            result = self.invoke(release, destination)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((destination / "rt").exists())


if __name__ == "__main__":
    unittest.main()
