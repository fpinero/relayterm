"""Fail-closed tests for the runtime audit, without simulating native acceptance."""

import copy
import hashlib
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from scripts import check_windows_runtime_independence as audit


class RuntimeBoundaryTests(unittest.TestCase):
    def fixture(self):
        candidate = r"C:\validation\candidate"
        system = r"C:\Windows"
        pins = {"rt.exe": "rt", "OpenConsole.exe": "helper", "conpty.dll": "conpty"}
        snapshot = {"elevated": False, "processes": [
            {"executable": candidate + r"\rt.exe", "modules": [
                {"path": candidate + r"\rt.exe", "sha256": "rt"},
                {"path": candidate + r"\conpty.dll", "sha256": "conpty"},
                {"path": system + r"\System32\VCRUNTIME140.dll", "sha256": "system-runtime"}]},
            {"executable": candidate + r"\OpenConsole.exe", "modules": [
                {"path": candidate + r"\OpenConsole.exe", "sha256": "helper"}]},
            {"executable": system + r"\System32\cmd.exe", "modules": [
                {"path": system + r"\System32\cmd.exe", "sha256": "cmd"}]}]}
        return snapshot, candidate, system, pins

    def classify(self, snapshot):
        _, candidate, system, pins = self.fixture()
        return audit.classify_modules(snapshot, candidate, system, pins, "system-runtime")

    def test_positive_dependency_closure_control(self):
        modules = self.classify(self.fixture()[0])
        self.assertEqual({row["origin"] for row in modules}, {"candidate", "Windows"})

    def test_foreign_and_misleading_prefix_paths_are_rejected(self):
        for path in (r"C:\development\extra.dll", r"C:\WindowsExtra\extra.dll",
                     r"C:\validation\candidate-old\extra.dll", r"C:\Windows\..\extra.dll"):
            with self.subTest(path=path):
                snapshot = copy.deepcopy(self.fixture()[0])
                snapshot["processes"][0]["modules"].append({"path": path, "sha256": "foreign"})
                with self.assertRaises(RuntimeError):
                    self.classify(snapshot)

    def test_altered_and_nested_package_modules_are_rejected(self):
        for path, sha in ((r"C:\validation\candidate\conpty.dll", "altered"),
                          (r"C:\validation\candidate\nested\conpty.dll", "conpty")):
            snapshot = copy.deepcopy(self.fixture()[0])
            snapshot["processes"][0]["modules"][1] = {"path": path, "sha256": sha}
            with self.assertRaises(RuntimeError):
                self.classify(snapshot)

    def test_private_runtime_and_wrong_system_runtime_are_rejected(self):
        for path, sha in ((r"C:\development\VCRUNTIME140.dll", "system-runtime"),
                          (r"C:\Windows\SysWOW64\VCRUNTIME140.dll", "system-runtime"),
                          (r"C:\Windows\System32\VCRUNTIME140.dll", "changed")):
            snapshot = copy.deepcopy(self.fixture()[0])
            snapshot["processes"][0]["modules"][2] = {"path": path, "sha256": sha}
            with self.assertRaises(RuntimeError):
                self.classify(snapshot)

    def test_missing_helper_or_runtime_never_qualifies(self):
        original = self.fixture()[0]
        cases = [dict(original, elevated=True), dict(original, processes=[])]
        missing_helper = copy.deepcopy(original)
        missing_helper["processes"].pop(1)
        cases.append(missing_helper)
        missing_runtime = copy.deepcopy(original)
        missing_runtime["processes"][0]["modules"].pop(1)
        cases.append(missing_runtime)
        for snapshot in cases:
            with self.assertRaises(RuntimeError):
                self.classify(snapshot)

    def test_product_environment_does_not_inherit_credentials_or_overrides(self):
        with patch.dict(os.environ, {"SYNTHETIC_SECRET": "fake", "RELAYTERM_TEST_RT": "foreign", "CI": "true"}):
            env = audit.clean_environment(Path("Windows"), Path("fixture"))
        self.assertFalse({"SYNTHETIC_SECRET", "RELAYTERM_TEST_RT", "CI"}.intersection(env))
        self.assertEqual(env["PATH"], str(Path("Windows") / "System32"))
        self.assertEqual(env["USERPROFILE"], str(Path("fixture") / "profile"))

    def test_package_tampering_and_extra_files_are_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            pins = {}
            for name in ["rt.exe"] + [f"support-{index}.txt" for index in range(12)]:
                data = name.encode()
                (root / name).write_bytes(data)
                pins[name] = hashlib.sha256(data).hexdigest()
            inventory = {"product_source": audit.EXPECTED_SOURCE, "installation": {"inventory": pins}}
            with patch.object(audit, "EXPECTED_BINARY", pins["rt.exe"]):
                audit.verify_package(root, inventory)
                (root / "extra.txt").write_text("synthetic")
                with self.assertRaises(RuntimeError):
                    audit.verify_package(root, inventory)
                (root / "extra.txt").unlink()
                (root / "support-0.txt").write_text("tampered")
                with self.assertRaises(RuntimeError):
                    audit.verify_package(root, inventory)
    def test_symbolic_link_cannot_qualify_as_package_file(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            sha = hashlib.sha256(b"synthetic").hexdigest()
            pins = {"rt.exe": sha}
            pins.update({f"support-{index}.txt": sha for index in range(12)})
            for name in pins:
                (root / name).write_text("synthetic")
            inventory = {"product_source": audit.EXPECTED_SOURCE, "installation": {"inventory": pins}}
            with patch.object(audit, "EXPECTED_BINARY", pins["rt.exe"]):
                audit.verify_package(root, inventory)
            (root / "support-0.txt").unlink()
            try:
                (root / "support-0.txt").symlink_to(root / "rt.exe")
            except OSError:
                self.skipTest("symbolic link creation is unavailable to this account")
            with patch.object(audit, "EXPECTED_BINARY", pins["rt.exe"]):
                with self.assertRaises(RuntimeError):
                    audit.verify_package(root, inventory)

    @unittest.skipIf(os.name == "nt", "requires a non-Windows host")
    def test_non_native_execution_refuses_before_creating_output(self):
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder) / "evidence"
            with self.assertRaisesRegex(RuntimeError, "native Windows"):
                audit.audit(Path("missing"), Path("missing"), output)
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
