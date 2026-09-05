"""Scan candidate sources and prove that Gitleaks rejects a synthetic marker."""

import shutil
import random
import string
import subprocess
import tempfile
from pathlib import Path

from check_repository import ROOT, candidate_paths


def scan(path, expected):
    result = subprocess.run(
        ["gitleaks", "dir", "--redact", "--no-banner", str(path)],
        capture_output=True,
    )
    # Do not echo scanner diagnostics or paths, even for unexpected failures.
    if result.returncode != expected:
        raise RuntimeError(f"Scanner returned {result.returncode}; expected {expected}")


def main():
    with tempfile.TemporaryDirectory(prefix="relayterm-source-scan-") as directory:
        destination = Path(directory)
        for name in candidate_paths():
            source = ROOT / name
            if not source.is_file() or source.is_symlink():
                continue
            target = destination / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
        scan(destination, 0)
    with tempfile.TemporaryDirectory(prefix="relayterm-scanner-control-") as directory:
        # Assemble a recognizable but deliberately fake token outside tracked files.
        generator = random.Random(42)
        marker = "ghp_" + "".join(generator.choices(string.ascii_letters + string.digits, k=36))
        (Path(directory) / "synthetic.txt").write_text("token = " + marker + "\n", encoding="utf-8")
        scan(directory, 1)
    print("Passed: candidate sources are clean; synthetic negative control was rejected.")


if __name__ == "__main__":
    main()
