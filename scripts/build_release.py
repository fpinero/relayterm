"""Build and compare native Relayterm release executables."""

import argparse
import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import sys


ROOT = pathlib.Path(__file__).resolve().parents[1]
TARGETS = {
    "x86_64-unknown-linux-gnu": ("rt", "Ubuntu 24.04"),
    "aarch64-apple-darwin": ("rt", "macOS 14"),
    "x86_64-pc-windows-msvc": ("rt.exe", "Windows 10 22H2"),
}
ENCODED_FLAG_SEPARATOR = "\x1f"


def run(arguments, *, cwd=ROOT, environment=None):
    return subprocess.run(
        arguments,
        cwd=cwd,
        env=environment,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def native_host():
    for line in run(["rustc", "-vV"]).splitlines():
        if line.startswith("host: "):
            return line.removeprefix("host: ")
    raise RuntimeError("rustc did not report its host target")


def require_clean_tracked_source():
    for arguments in (["git", "diff", "--quiet", "--"], ["git", "diff", "--cached", "--quiet", "--"]):
        result = subprocess.run(arguments, cwd=ROOT, check=False)
        if result.returncode != 0:
            raise RuntimeError("tracked source changes must be committed before a release build")


def prepare_output(path):
    path = path.resolve()
    if path.exists() and (not path.is_dir() or any(path.iterdir())):
        raise RuntimeError("the release output directory must be absent or empty")
    path.mkdir(parents=True, exist_ok=True)
    return path


def release_rustflags(environment, target):
    home = pathlib.Path.home().resolve()
    cargo_home = pathlib.Path(environment.get("CARGO_HOME", home / ".cargo")).resolve()
    mappings = (
        (ROOT.resolve(), "/source"),
        (cargo_home, "/cargo"),
        (home, "/build-home"),
    )
    flags = [f"--remap-path-prefix={source}={destination}" for source, destination in mappings]
    public_flags = [
        "--remap-path-prefix=<source>=/source",
        "--remap-path-prefix=<cargo-home>=/cargo",
        "--remap-path-prefix=<build-home>=/build-home",
    ]
    if target == "x86_64-pc-windows-msvc":
        windows_flags = [
            "-C",
            "link-arg=/Brepro",
            "-C",
            "link-arg=/PDBALTPATH:%_PDB%",
        ]
        flags.extend(windows_flags)
        public_flags.extend(windows_flags)
    private_prefixes = {os.fspath(source).encode() for source, _ in mappings}
    return flags, public_flags, private_prefixes


def reject_private_build_paths(executable, private_prefixes):
    content = executable.read_bytes()
    if any(prefix and prefix in content for prefix in private_prefixes):
        raise RuntimeError("the release executable contains a private build path")


def package_version():
    metadata = json.loads(run(["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"]))
    package = next(item for item in metadata["packages"] if item["name"] == "relayterm-cli")
    return package["version"]


def build(target, output, offline):
    if target not in TARGETS:
        raise RuntimeError("the target is not in the Relayterm release matrix")
    if native_host() != target:
        raise RuntimeError("release artifacts must be built on their native target")
    require_clean_tracked_source()
    output = prepare_output(output)
    source_sha = run(["git", "rev-parse", "HEAD"])
    source_epoch = run(["git", "show", "-s", "--format=%ct", "HEAD"])
    target_dir = output / "cargo-target"
    command = [
        "cargo",
        "build",
        "--locked",
        "--release",
        "-p",
        "relayterm-cli",
        "--bin",
        "rt",
        "--target",
        target,
    ]
    if offline:
        command.insert(2, "--offline")
    environment = os.environ.copy()
    environment["CARGO_INCREMENTAL"] = "0"
    environment["CARGO_TARGET_DIR"] = os.fspath(target_dir)
    environment["SOURCE_DATE_EPOCH"] = source_epoch
    rustflags, public_rustflags, private_prefixes = release_rustflags(environment, target)
    environment.pop("RUSTFLAGS", None)
    environment["CARGO_ENCODED_RUSTFLAGS"] = ENCODED_FLAG_SEPARATOR.join(rustflags)
    subprocess.run(command, cwd=ROOT, env=environment, check=True)
    executable_name, baseline = TARGETS[target]
    built = target_dir / target / "release" / executable_name
    if not built.is_file() or built.is_symlink():
        raise RuntimeError("Cargo did not produce the expected regular executable")
    executable = output / executable_name
    shutil.copyfile(built, executable)
    executable.chmod(0o755)
    reject_private_build_paths(executable, private_prefixes)
    shutil.rmtree(target_dir)
    record = {
        "format_version": 1,
        "product": "relayterm",
        "version": package_version(),
        "source_sha": source_sha,
        "source_date_epoch": source_epoch,
        "target": target,
        "tested_runtime_baseline": baseline,
        "profile": "release",
        "features": [],
        "build_command": f"cargo build --locked{' --offline' if offline else ''} --release -p relayterm-cli --bin rt --target {target}",
        "rustc": run(["rustc", "--version"]),
        "cargo": run(["cargo", "--version"]),
        "rustflags": public_rustflags,
        "binary": {
            "name": executable_name,
            "bytes": executable.stat().st_size,
            "sha256": sha256(executable),
        },
        "signing": "unsigned",
    }
    (output / "build-record.json").write_text(
        json.dumps(record, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(json.dumps(record, sort_keys=True))


def compare(first_path, second_path):
    first = json.loads(first_path.read_text(encoding="utf-8"))
    second = json.loads(second_path.read_text(encoding="utf-8"))
    inputs = ["version", "source_sha", "source_date_epoch", "target", "profile", "features", "rustc", "cargo", "rustflags"]
    mismatched = [key for key in inputs if first.get(key) != second.get(key)]
    if mismatched:
        raise RuntimeError(f"build inputs differ: {', '.join(mismatched)}")
    identical = first.get("binary") == second.get("binary")
    result = {
        "format_version": 1,
        "source_sha": first["source_sha"],
        "target": first["target"],
        "binary_byte_identical": identical,
        "first_sha256": first["binary"]["sha256"],
        "second_sha256": second["binary"]["sha256"],
    }
    print(json.dumps(result, sort_keys=True))
    if not identical:
        raise RuntimeError("release executables are not byte-identical")


def parse_arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="command", required=True)
    build_parser = subcommands.add_parser("build")
    build_parser.add_argument("--target", required=True, choices=sorted(TARGETS))
    build_parser.add_argument("--output", required=True, type=pathlib.Path)
    build_parser.add_argument("--offline", action="store_true")
    compare_parser = subcommands.add_parser("compare")
    compare_parser.add_argument("first", type=pathlib.Path)
    compare_parser.add_argument("second", type=pathlib.Path)
    return parser.parse_args()


def main():
    arguments = parse_arguments()
    try:
        if arguments.command == "build":
            build(arguments.target, arguments.output, arguments.offline)
        else:
            compare(arguments.first, arguments.second)
    except (OSError, RuntimeError, subprocess.CalledProcessError, StopIteration, KeyError, json.JSONDecodeError) as error:
        print(f"Release build failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
