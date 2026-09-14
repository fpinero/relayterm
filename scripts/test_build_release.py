"""Focused tests for the native release build contract."""

import importlib.util
import json
import os
import pathlib
import tempfile
import unittest


MODULE_PATH = pathlib.Path(__file__).with_name("build_release.py")
SPEC = importlib.util.spec_from_file_location("build_release", MODULE_PATH)
build_release = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(build_release)


class ReleaseBuildTests(unittest.TestCase):
    def test_sha256_reads_complete_binary(self):
        with tempfile.TemporaryDirectory() as directory:
            binary = pathlib.Path(directory) / "rt"
            binary.write_bytes(b"relayterm-candidate")
            self.assertEqual(
                build_release.sha256(binary),
                "79c1be4eff78753b69c545a46680ccf64d377182a83c125e7b193575c8f1a659",
            )

    def test_compare_requires_identical_inputs_and_binary(self):
        record = {
            "version": "0.1.0",
            "source_sha": "0" * 40,
            "source_date_epoch": "1",
            "target": "aarch64-apple-darwin",
            "profile": "release",
            "features": [],
            "rustc": "rustc fixture",
            "cargo": "cargo fixture",
            "binary": {"name": "rt", "bytes": 2, "sha256": "a" * 64},
        }
        with tempfile.TemporaryDirectory() as directory:
            first = pathlib.Path(directory) / "first.json"
            second = pathlib.Path(directory) / "second.json"
            first.write_text(json.dumps(record), encoding="utf-8")
            second.write_text(json.dumps(record), encoding="utf-8")
            build_release.compare(first, second)
            changed = dict(record)
            changed["source_sha"] = "1" * 40
            second.write_text(json.dumps(changed), encoding="utf-8")
            with self.assertRaises(RuntimeError):
                build_release.compare(first, second)

    def test_nonempty_output_is_rejected_without_removal(self):
        with tempfile.TemporaryDirectory() as directory:
            output = pathlib.Path(directory)
            sentinel = output / "sentinel"
            sentinel.write_text("keep", encoding="utf-8")
            with self.assertRaises(RuntimeError):
                build_release.prepare_output(output)
            self.assertEqual(sentinel.read_text(encoding="utf-8"), "keep")

    def test_release_flags_remap_private_paths_without_recording_them(self):
        flags, public_flags, private_prefixes = build_release.release_rustflags({})
        self.assertTrue(all(flag.startswith("--remap-path-prefix=") for flag in flags))
        self.assertTrue(any(os.fspath(pathlib.Path.home()) in flag for flag in flags))
        self.assertTrue(all(os.fspath(pathlib.Path.home()) not in flag for flag in public_flags))
        self.assertIn(os.fspath(pathlib.Path.home()).encode(), private_prefixes)

    def test_private_build_path_is_rejected_without_echoing_it(self):
        with tempfile.TemporaryDirectory() as directory:
            executable = pathlib.Path(directory) / "rt"
            executable.write_bytes(b"prefix/private/build/root/source.rs")
            with self.assertRaisesRegex(RuntimeError, "private build path") as raised:
                build_release.reject_private_build_paths(executable, {b"/private/build/root"})
            self.assertNotIn("/private/build/root", str(raised.exception))


if __name__ == "__main__":
    unittest.main()
