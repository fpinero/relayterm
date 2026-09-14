"""Focused tests for deterministic Relayterm release archives."""

import importlib.util
import io
import json
import pathlib
import tarfile
import tempfile
import unittest
import zipfile


MODULE_PATH = pathlib.Path(__file__).with_name("package_release.py")
SPEC = importlib.util.spec_from_file_location("package_release", MODULE_PATH)
package_release = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(package_release)


class PackageReleaseTests(unittest.TestCase):
    def test_tar_output_is_byte_identical_and_has_normalized_metadata(self):
        entries = [("relayterm/rt", b"binary", 0o755), ("relayterm/LICENSE", b"license", 0o644)]
        with tempfile.TemporaryDirectory() as directory:
            first = pathlib.Path(directory) / "first.tar.gz"
            second = pathlib.Path(directory) / "second.tar.gz"
            package_release.make_tar(first, entries, 1_700_000_000)
            package_release.make_tar(second, entries, 1_700_000_000)
            self.assertEqual(first.read_bytes(), second.read_bytes())
            with tarfile.open(first, "r:gz") as archive:
                members = archive.getmembers()
                self.assertEqual([item.name for item in members], [item[0] for item in entries])
                self.assertEqual([item.mode for item in members], [0o755, 0o644])
                self.assertTrue(all(item.uid == 0 and item.gid == 0 for item in members))

    def test_zip_output_is_byte_identical_and_has_normalized_modes(self):
        entries = [("relayterm/rt.exe", b"binary", 0o755), ("relayterm/LICENSE", b"license", 0o644)]
        with tempfile.TemporaryDirectory() as directory:
            first = pathlib.Path(directory) / "first.zip"
            second = pathlib.Path(directory) / "second.zip"
            package_release.make_zip(first, entries, 1_700_000_000)
            package_release.make_zip(second, entries, 1_700_000_000)
            self.assertEqual(first.read_bytes(), second.read_bytes())
            with zipfile.ZipFile(first) as archive:
                self.assertEqual(archive.namelist(), [item[0] for item in entries])
                self.assertEqual([item.external_attr >> 16 & 0o777 for item in archive.infolist()], [0o755, 0o644])

    def test_archive_inspection_rejects_parent_traversal(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "relayterm-fixture.tar.gz"
            with tarfile.open(path, "w:gz") as archive:
                info = tarfile.TarInfo("relayterm-fixture/../sentinel")
                info.size = 1
                archive.addfile(info, io.BytesIO(b"x"))
            with self.assertRaises(RuntimeError):
                package_release.inspect_archive(path)

    def test_extract_refuses_nonempty_destination(self):
        with tempfile.TemporaryDirectory() as directory:
            output = pathlib.Path(directory)
            sentinel = output / "sentinel"
            sentinel.write_text("keep", encoding="utf-8")
            with self.assertRaises(RuntimeError):
                package_release.extract_archive(output / "missing.tar.gz", output)
            self.assertEqual(sentinel.read_text(encoding="utf-8"), "keep")

    def test_validate_build_rejects_modified_binary(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            binary = root / "rt"
            binary.write_bytes(b"changed")
            (root / "build-record.json").write_text(
                '{"format_version":1,"product":"relayterm","binary":{"name":"rt","bytes":1,"sha256":"00"}}',
                encoding="utf-8",
            )
            with self.assertRaises(RuntimeError):
                package_release.validate_build(root)

    def test_compare_requires_identical_archive_and_inputs(self):
        record = {
            "version": "0.1.0",
            "source_sha": "0" * 40,
            "target": "aarch64-apple-darwin",
            "binary": {"name": "rt", "bytes": 1, "sha256": "a" * 64},
            "archive": {"name": "relayterm.tar.gz", "bytes": 2, "sha256": "b" * 64},
            "signing": "unsigned",
        }
        with tempfile.TemporaryDirectory() as directory:
            first = pathlib.Path(directory) / "first.json"
            second = pathlib.Path(directory) / "second.json"
            first.write_text(json.dumps(record), encoding="utf-8")
            second.write_text(json.dumps(record), encoding="utf-8")
            package_release.compare(first, second)
            changed = {**record, "archive": {**record["archive"], "sha256": "c" * 64}}
            second.write_text(json.dumps(changed), encoding="utf-8")
            with self.assertRaises(RuntimeError):
                package_release.compare(first, second)


if __name__ == "__main__":
    unittest.main()
