"""Focused POSIX tests for collision-safe release installation."""

import os
import json
import hashlib
import shutil
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
        if os.name == "nt":
            inputs = pathlib.Path(os.environ["RELAYTERM_BUILD_RUNTIME_ROOT"])
            for name in ("conpty.dll", "OpenConsole.exe"):
                shutil.copyfile(inputs / name, release / name)
            for name in ("CONPTY_LICENSE.txt", "CONPTY_PROVENANCE.json"):
                (release / name).write_text("fixture")
            entries = [{"path": p.name, "bytes": p.stat().st_size, "sha256": hashlib.sha256(p.read_bytes()).hexdigest()} for p in release.iterdir() if p.name != "manifest.json"]
            (release / "manifest.json").write_text(json.dumps({"format_version": 1, "product": "relayterm", "target": "x86_64-pc-windows-msvc", "contents": entries}))
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


@unittest.skipUnless(os.name == "nt", "PowerShell installer tests run on Windows")
class PowerShellInstallReleaseTests(unittest.TestCase):
    def fixture(self, root):
        release = root / "release λ"
        destination = root / "destination λ"
        release.mkdir()
        destination.mkdir()
        (release / "rt.exe").write_bytes(b"fixture-executable")
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
        if os.name == "nt":
            inputs = pathlib.Path(os.environ["RELAYTERM_BUILD_RUNTIME_ROOT"])
            for name in ("conpty.dll", "OpenConsole.exe"):
                shutil.copyfile(inputs / name, release / name)
            for name in ("CONPTY_LICENSE.txt", "CONPTY_PROVENANCE.json"):
                (release / name).write_text("fixture")
            entries = [{"path": p.name, "bytes": p.stat().st_size, "sha256": hashlib.sha256(p.read_bytes()).hexdigest()} for p in release.iterdir() if p.name != "manifest.json"]
            (release / "manifest.json").write_text(json.dumps({"format_version": 1, "product": "relayterm", "target": "x86_64-pc-windows-msvc", "contents": entries}))
        return release, destination

    def invoke(self, release, destination):
        return subprocess.run(
            [
                "powershell.exe",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy", "RemoteSigned",
                "-File",
                os.fspath(SCRIPT.with_suffix(".ps1")),
                "-ReleaseRoot",
                os.fspath(release),
                "-Destination",
                os.fspath(destination),
            ],
            capture_output=True,
            text=True,
            check=False,
        )

    def test_install_and_existing_destination_preservation(self):
        with tempfile.TemporaryDirectory() as directory:
            release, destination = self.fixture(pathlib.Path(directory))
            self.assertEqual(self.invoke(release, destination).returncode, 0)
            target = destination / "rt.exe"
            target.write_bytes(b"unrelated-command")
            self.assertNotEqual(self.invoke(release, destination).returncode, 0)
            self.assertEqual(target.read_bytes(), b"unrelated-command")

    def test_helper_collision_and_owned_removal_preserve_foreign_files(self):
        with tempfile.TemporaryDirectory() as directory:
            release, destination = self.fixture(pathlib.Path(directory))
            marker = destination / "foreign.txt"
            marker.write_bytes(b"keep")
            helper = destination / "OpenConsole.exe"
            helper.write_bytes(b"unrelated-helper")
            self.assertNotEqual(self.invoke(release, destination).returncode, 0)
            self.assertEqual(helper.read_bytes(), b"unrelated-helper")
            self.assertFalse((destination / "rt.exe").exists())
            helper.unlink()
            self.assertEqual(self.invoke(release, destination).returncode, 0)
            result = subprocess.run(["powershell.exe", "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "RemoteSigned", "-File", str(SCRIPT.with_suffix(".ps1")), "-Destination", str(destination), "-Remove"], capture_output=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(list(destination.iterdir()), [marker])
            self.assertEqual(marker.read_bytes(), b"keep")

    def test_corrupt_helper_rejects_install_before_mutation(self):
        with tempfile.TemporaryDirectory() as directory:
            release, destination = self.fixture(pathlib.Path(directory))
            (release / "OpenConsole.exe").write_bytes(b"corrupt")
            self.assertNotEqual(self.invoke(release, destination).returncode, 0)
            self.assertEqual(list(destination.iterdir()), [])

    def test_failed_staging_rolls_back_without_touching_foreign_files(self):
        with tempfile.TemporaryDirectory() as directory:
            release, destination = self.fixture(pathlib.Path(directory))
            marker = destination / "foreign.txt"
            marker.write_bytes(b"keep")
            expression = "function Copy-Item { param($LiteralPath,$Destination) if ([IO.Path]::GetFileName($LiteralPath) -eq 'OpenConsole.exe') { throw 'synthetic copy failure' }; Microsoft.PowerShell.Management\\Copy-Item -LiteralPath $LiteralPath -Destination $Destination }; & $args[0] -ReleaseRoot $args[1] -Destination $args[2]"
            wrapper = pathlib.Path(directory) / "fail-copy.ps1"
            wrapper.write_text("param($Installer,$Release,$Target)\n" + expression.replace("$args[0]", "$Installer").replace("$args[1]", "$Release").replace("$args[2]", "$Target"))
            result = subprocess.run(["pwsh", "-NoProfile", "-NonInteractive", "-File", str(wrapper), str(SCRIPT.with_suffix(".ps1")), str(release), str(destination)], capture_output=True, timeout=30)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b"synthetic copy failure", result.stderr)
            self.assertEqual(list(destination.iterdir()), [marker])
            self.assertEqual(marker.read_bytes(), b"keep")

    def test_partial_inventory_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            release, destination = self.fixture(pathlib.Path(directory))
            (release / "manifest.json").unlink()
            result = self.invoke(release, destination)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((destination / "rt.exe").exists())


if __name__ == "__main__":
    unittest.main()
