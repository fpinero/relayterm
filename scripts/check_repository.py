"""Validate candidate documentation and ignore boundaries without modifying source."""

import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def native_ci_commands_are_independent(text):
    required = [
        "- name: Install compiler\n",
        "- name: Report compiler\n",
        "- name: Report Cargo\n",
        "- name: Test M11 hardening, pass 1\n",
        "- name: Test M11 hardening, pass 2\n",
        "- name: Test M11 protocol faults, pass 1\n",
        "- name: Test M11 protocol faults, pass 2\n",
        "- name: Test M11 durable scale, pass 1\n",
        "- name: Test M11 durable scale, pass 2\n",
    ]
    return all(text.count(marker) == 1 for marker in required)


def candidate_paths():
    result = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=ROOT, check=True, capture_output=True,
    )
    return sorted(set(result.stdout.decode().split("\0")) - {""})


def main():
    count = 0
    for name in candidate_paths():
        path = ROOT / name
        if not path.is_file() or path.suffix not in {".md", ".rs", ".toml", ".yml", ".py"}:
            continue
        text = path.read_text(encoding="utf-8")
        assert "\u2014" not in text, f"Prohibited punctuation: {name}"
        assert text.endswith("\n"), f"Missing final newline: {name}"
        assert all(line == line.rstrip() for line in text.splitlines()), f"Trailing whitespace: {name}"
        if path.suffix == ".md":
            count += 1
            assert len(re.findall(r"^```", text, re.M)) % 2 == 0, f"Unbalanced fences: {name}"
            for link in re.findall(r"\]\(([^)]+)\)", text):
                if re.match(r"https?://", link) or link.startswith("#"):
                    continue
                assert (path.parent / link.split("#")[0]).is_file(), f"Broken link: {name}: {link}"
            assert not re.search(r"/(?:Users|home)/[\w.-]+/", text), f"Personal path: {name}"

    adrs = sorted((ROOT / "docs/decisions").glob("*.md"))
    assert len(adrs) == 8, "Expected eight bootstrap ADRs"
    for number, path in enumerate(adrs, 1):
        assert path.name.startswith(f"{number:04d}-")
        text = path.read_text(encoding="utf-8")
        for section in ["Context", "Decision", "Invariants and behavior", "Alternatives", "Consequences", "Verification and ownership"]:
            assert f"## {section}\n" in text, f"Missing ADR section: {path.name}: {section}"

    queue = (ROOT / "TODO.md").read_text(encoding="utf-8")
    assert not re.search(r"\[[xX]\]", queue), "Completed checkbox in pending queue"
    ignored = ["private.sqlite", "private.sqlite-wal", "private.sqlite-shm", "private.db",
               "private.sock", "private.log", "private.cast", ".relayterm/config.toml",
               "relayterm.local.toml", "target/test", ".idea/workspace.xml"]
    visible = ["Cargo.lock", "Cargo.toml", "migrations/0001.sql", "fixtures/synthetic/example.txt",
               "fixtures/synthetic/agent.example.toml", "docs/M01_details.md"]
    for path, expected in [(p, 0) for p in ignored] + [(p, 1) for p in visible]:
        result = subprocess.run(["git", "check-ignore", "--no-index", "-q", path], cwd=ROOT)
        assert result.returncode == expected, f"Incorrect ignore policy: {path}"
    production = "\n".join(
        path.read_text(encoding="utf-8")
        for path in sorted((ROOT / "crates").glob("*/src/*.rs"))
    )
    for forbidden in ["TcpListener", "TcpStream", "UdpSocket", "reqwest::", "hyper::", "ureq::"]:
        assert forbidden not in production, f"Unexpected product network surface: {forbidden}"
    cli = (ROOT / "crates/relayterm-cli/src/main.rs").read_text(encoding="utf-8")
    for forbidden in ["fault-injection", "synthetic-supervisor", "fake-session"]:
        assert forbidden not in cli, f"Production test capability: {forbidden}"
    quality = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
    assert native_ci_commands_are_independent(quality), "Native CI commands or repetitions were combined"
    combined_negative_control = quality.replace(
        "- name: Install compiler\n",
        "- name: Install compiler and report host\n",
        1,
    )
    assert not native_ci_commands_are_independent(combined_negative_control), (
        "Native CI independence negative control was not detected"
    )
    print(f"Passed: {count} Markdown files, eight ADRs, style and candidate links, pending queue, and ignore boundaries.")


if __name__ == "__main__":
    main()
