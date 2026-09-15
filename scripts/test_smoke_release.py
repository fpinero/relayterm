"""Focused tests for the installed release smoke environment."""

import importlib.util
import pathlib
import subprocess
import sys
import unittest
from unittest import mock


MODULE_PATH = pathlib.Path(__file__).with_name("smoke_release.py")
sys.path.insert(0, str(MODULE_PATH.parent))
SPEC = importlib.util.spec_from_file_location("smoke_release", MODULE_PATH)
smoke_release = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(smoke_release)


class SmokeReleaseTests(unittest.TestCase):
    def test_windows_system_root_lookup_is_case_insensitive(self):
        self.assertEqual(
            smoke_release.windows_system_root({"SYSTEMROOT": "C:/Windows"}),
            pathlib.Path("C:/Windows"),
        )
        self.assertEqual(
            smoke_release.windows_system_root({"windir": "C:/Alternate"}),
            pathlib.Path("C:/Alternate"),
        )

    def test_missing_windows_system_root_has_value_free_error(self):
        with self.assertRaisesRegex(RuntimeError, "Windows system directory") as raised:
            smoke_release.windows_system_root({"PRIVATE_VALUE": "sensitive"})
        self.assertNotIn("sensitive", str(raised.exception))

    def test_invoke_bounds_installed_command_runtime(self):
        completed = subprocess.CompletedProcess([], 0, '{"ok":true,"result":{}}\n', "")
        with mock.patch.object(smoke_release, "constrained_environment", return_value={}), mock.patch.object(
            smoke_release.subprocess, "run", return_value=completed
        ) as run:
            smoke_release.invoke(pathlib.Path("rt"), pathlib.Path("project"), pathlib.Path("home"), ["workspace", "status"])
        self.assertEqual(run.call_args.kwargs["timeout"], smoke_release.COMMAND_TIMEOUT_SECONDS)


if __name__ == "__main__":
    unittest.main()
