"""Verify metadata staging across clean Windows and Unix checkouts."""
import hashlib
import importlib.util
import json
import pathlib
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("runtime", pathlib.Path(__file__).with_name("windows_runtime.py"))
runtime = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runtime)


class StagingTests(unittest.TestCase):
    def test_checkout_line_endings_produce_identical_pinned_metadata(self):
        license_text = (runtime.ROOT / "third_party/conpty/LICENSE.txt").read_text(encoding="utf-8")
        identity = json.loads(runtime.PROVENANCE.read_text())
        self.assertEqual(hashlib.sha256(license_text.encode()).hexdigest(), identity["license_sha256"])
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            source = root / "inputs"
            source.mkdir()
            identity["files"] = {name: hashlib.sha256(b"synthetic binary").hexdigest() for name in identity["files"]}
            for name in identity["files"]:
                (source / name).write_bytes(b"synthetic binary")
            outputs = []
            for number, newline in enumerate(("\n", "\r\n")):
                checkout = root / str(number)
                metadata = checkout / "third_party/conpty"
                metadata.mkdir(parents=True)
                (metadata / "LICENSE.txt").write_bytes(license_text.replace("\n", newline).encode())
                (metadata / "provenance.json").write_bytes((json.dumps(identity, indent=2)+"\n").replace("\n", newline).encode())
                destination = root / (str(number) + "-staged")
                with patch.object(runtime, "ROOT", checkout), patch.object(runtime, "PROVENANCE", metadata / "provenance.json"), patch.object(runtime, "verify", return_value=identity):
                    runtime.stage(source, destination)
                    runtime.stage(source, destination)
                    outputs.append([(destination / name).read_bytes() for name in runtime.INVENTORY])
                    (metadata / "LICENSE.txt").write_bytes(b"altered license")
                    with self.assertRaisesRegex(RuntimeError, "license hash mismatch"):
                        runtime.stage(source, destination)
            self.assertEqual(outputs[0], outputs[1])


if __name__ == "__main__":
    unittest.main()
