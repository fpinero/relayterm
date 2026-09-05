"""Prove license and dependency bans fail using disposable policy copies."""

import re
import subprocess
import tempfile
from pathlib import Path

from check_repository import ROOT


def main():
    policy = (ROOT / "deny.toml").read_text(encoding="utf-8")
    cases = [
        ("licenses", re.sub(r"^allow = \[.*\]$", "allow = []", policy, flags=re.M), "rejected"),
        ("bans", policy.replace('[bans]\n', '[bans]\ndeny = [{ name = "uuid" }]\n'), "banned"),
    ]
    with tempfile.TemporaryDirectory(prefix="relayterm-audit-control-") as directory:
        config = Path(directory) / "deny.toml"
        for check, contents, diagnostic in cases:
            config.write_text(contents, encoding="utf-8")
            result = subprocess.run(
                ["cargo", "deny", "--offline", "--locked", "--config", str(config), "check", check],
                cwd=ROOT, capture_output=True, text=True,
            )
            output = result.stdout + result.stderr
            codes = re.findall(r"(?:error|warning)\[([^\]]+)\]", output)
            assert result.returncode != 0 and diagnostic in codes, f"Negative {check} control: exit {result.returncode}, codes {codes}"
    print("Passed: license rejection and dependency ban negative controls.")


if __name__ == "__main__":
    main()
