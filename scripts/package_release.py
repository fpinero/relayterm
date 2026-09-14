"""Create and inspect deterministic Relayterm portable archives."""

import argparse
import datetime
import gzip
import hashlib
import io
import json
import os
import pathlib
import re
import stat
import subprocess
import sys
import tarfile
import zipfile


ROOT = pathlib.Path(__file__).resolve().parents[1]
MAX_NOTICE_BYTES = 8 * 1024 * 1024
LICENSE_NAMES = re.compile(r"^(licen[cs]e|copying|notice)([-._].*)?$", re.IGNORECASE)


def sha256_bytes(value):
    return hashlib.sha256(value).hexdigest()


def sha256_file(path):
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8"))


def run(arguments):
    return subprocess.run(
        arguments,
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout


def production_packages(target):
    metadata = json.loads(run(["cargo", "metadata", "--locked", "--format-version", "1"]))
    by_key = {(item["name"], item["version"]): item for item in metadata["packages"]}
    workspace = set(metadata["workspace_members"])
    lines = run(
        [
            "cargo",
            "tree",
            "--locked",
            "--target",
            target,
            "-p",
            "relayterm-cli",
            "-e",
            "normal",
            "--prefix",
            "none",
            "--format",
            "{p}",
        ]
    ).splitlines()
    selected = {}
    for line in lines:
        match = re.match(r"^(.+?) v([^ ]+)(?: \(.*\))?$", line.strip())
        if not match:
            raise RuntimeError("Cargo returned an unrecognized package record")
        key = match.group(1), match.group(2)
        package = by_key.get(key)
        if package is None:
            raise RuntimeError("Cargo tree package is absent from locked metadata")
        if package["id"] not in workspace:
            selected[key] = package
    return [selected[key] for key in sorted(selected)]


def license_files(package):
    directory = pathlib.Path(package["manifest_path"]).parent
    files = []
    candidates = (path for path in directory.glob("**/*") if len(path.relative_to(directory).parts) <= 2)
    for path in sorted(candidates, key=lambda item: item.as_posix().casefold()):
        relative = path.relative_to(directory)
        in_license_directory = relative.parts[0].casefold() in ("license", "licenses")
        if path.is_file() and not path.is_symlink() and (LICENSE_NAMES.match(path.name) or in_license_directory):
            value = path.read_bytes()
            if b"\0" in value:
                raise RuntimeError("a dependency license file is not text")
            files.append((relative.as_posix(), value.decode("utf-8", errors="strict")))
    if not files:
        raise RuntimeError(
            f"dependency {package['name']} {package['version']} has no packaged license text"
        )
    return files


def third_party_material(target):
    packages = production_packages(target)
    inventory = []
    texts = {}
    for package in packages:
        license_expression = package.get("license")
        if not license_expression:
            raise RuntimeError("a production dependency has no declared license")
        references = []
        for filename, value in license_files(package):
            digest = sha256_bytes(value.encode("utf-8"))
            texts.setdefault(digest, value)
            references.append({"file": filename, "text_sha256": digest})
        inventory.append(
            {
                "name": package["name"],
                "version": package["version"],
                "license": license_expression,
                "license_files": references,
            }
        )

    notices = [
        "Relayterm third-party notices",
        "",
        f"Target production graph: {target}",
        "The SPDX expressions below come from the locked Cargo package metadata.",
        "The accompanying license-text file contains the packaged license files",
        "identified by SHA-256. Test-only and Cargo build-dependency edges are excluded.",
        "The normal closure conservatively retains proc-macro packages used at build time.",
        "SQLite is compiled from the public-domain SQLite amalgamation distributed by",
        "libsqlite3-sys; the Rust wrapper remains covered by its declared MIT license.",
        "",
    ]
    for item in inventory:
        references = ", ".join(
            f"{entry['file']} ({entry['text_sha256']})" for entry in item["license_files"]
        )
        notices.append(
            f"{item['name']} {item['version']} | {item['license']} | {references}"
        )
    notice_bytes = ("\n".join(notices) + "\n").encode("utf-8")

    licenses = ["Relayterm third-party license texts", ""]
    for digest, value in sorted(texts.items()):
        licenses.extend([f"===== SHA-256 {digest} =====", value.rstrip(), ""])
    license_bytes = ("\n".join(licenses) + "\n").encode("utf-8")
    if len(notice_bytes) + len(license_bytes) > MAX_NOTICE_BYTES:
        raise RuntimeError("third-party notice material exceeds its package bound")
    return notice_bytes, license_bytes, inventory


def documentation_files():
    return (ROOT / "docs" / "install.md").read_bytes(), (ROOT / "docs" / "upgrade.md").read_bytes()


def validate_build(build_directory):
    record = read_json(build_directory / "build-record.json")
    if record.get("format_version") != 1 or record.get("product") != "relayterm":
        raise RuntimeError("unsupported build record")
    binary = build_directory / record["binary"]["name"]
    if not binary.is_file() or binary.is_symlink():
        raise RuntimeError("build record binary is not a regular file")
    if binary.stat().st_size != record["binary"]["bytes"] or sha256_file(binary) != record["binary"]["sha256"]:
        raise RuntimeError("build record does not match the binary")
    if record["binary"]["name"] not in ("rt", "rt.exe"):
        raise RuntimeError("unexpected product executable name")
    return record, binary


def add_tar_entry(archive, name, value, mode, epoch):
    info = tarfile.TarInfo(name)
    info.size = len(value)
    info.mode = mode
    info.mtime = epoch
    info.uid = info.gid = 0
    info.uname = info.gname = ""
    archive.addfile(info, io.BytesIO(value))


def make_tar(path, entries, epoch):
    with path.open("wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=epoch) as compressed:
            with tarfile.open(fileobj=compressed, mode="w", format=tarfile.GNU_FORMAT) as archive:
                for name, value, mode in entries:
                    add_tar_entry(archive, name, value, mode, epoch)


def make_zip(path, entries, epoch):
    date = datetime.datetime.fromtimestamp(max(epoch, 315532800), tz=datetime.timezone.utc)
    stamp = (date.year, date.month, date.day, date.hour, date.minute, date.second)
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for name, value, mode in entries:
            info = zipfile.ZipInfo(name, stamp)
            info.create_system = 3
            info.external_attr = (stat.S_IFREG | mode) << 16
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, value)


def ensure_empty_output(path):
    path = path.resolve()
    if path.exists() and (not path.is_dir() or any(path.iterdir())):
        raise RuntimeError("the package output directory must be absent or empty")
    path.mkdir(parents=True, exist_ok=True)
    return path


def package(build_directory, output):
    record, binary = validate_build(build_directory.resolve())
    output = ensure_empty_output(output)
    notices, licenses, inventory = third_party_material(record["target"])
    install, recovery = documentation_files()
    root_name = f"relayterm-{record['version']}-{record['target']}"
    payloads = {
        record["binary"]["name"]: binary.read_bytes(),
        "LICENSE": (ROOT / "LICENSE").read_bytes(),
        "THIRD_PARTY_NOTICES.txt": notices,
        "THIRD_PARTY_LICENSES.txt": licenses,
        "INSTALL.md": install,
        "RECOVERY.md": recovery,
        "install_release.sh": (ROOT / "scripts" / "install_release.sh").read_bytes(),
        "install_release.ps1": (ROOT / "scripts" / "install_release.ps1").read_bytes(),
    }
    contents = [
        {
            "path": name,
            "bytes": len(value),
            "sha256": sha256_bytes(value),
            "mode": "0755" if name in (record["binary"]["name"], "install_release.sh") else "0644",
        }
        for name, value in sorted(payloads.items())
    ]
    package_manifest = {
        "format_version": 1,
        "product": "relayterm",
        "version": record["version"],
        "source_sha": record["source_sha"],
        "target": record["target"],
        "tested_runtime_baseline": record["tested_runtime_baseline"],
        "profile": record["profile"],
        "features": record["features"],
        "build_command": record["build_command"],
        "rustc": record["rustc"],
        "cargo": record["cargo"],
        "signing": record["signing"],
        "binary": record["binary"],
        "third_party_packages": inventory,
        "contents": contents,
    }
    manifest_bytes = (json.dumps(package_manifest, indent=2, sort_keys=True) + "\n").encode("utf-8")
    payloads["manifest.json"] = manifest_bytes
    entries = [
        (
            f"{root_name}/{name}",
            value,
            0o755 if name in (record["binary"]["name"], "install_release.sh") else 0o644,
        )
        for name, value in sorted(payloads.items())
    ]
    epoch = int(record["source_date_epoch"])
    extension = ".zip" if record["target"].endswith("windows-msvc") else ".tar.gz"
    archive = output / f"{root_name}{extension}"
    if extension == ".zip":
        make_zip(archive, entries, epoch)
    else:
        make_tar(archive, entries, epoch)
    archive_manifest = {
        "format_version": 1,
        "product": "relayterm",
        "version": record["version"],
        "source_sha": record["source_sha"],
        "target": record["target"],
        "archive": {
            "name": archive.name,
            "bytes": archive.stat().st_size,
            "sha256": sha256_file(archive),
        },
        "binary": record["binary"],
        "signing": record["signing"],
    }
    manifest_path = output / f"{root_name}.manifest.json"
    manifest_path.write_text(json.dumps(archive_manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    sums = output / "SHA256SUMS"
    sums.write_text(
        f"{sha256_file(archive)}  {archive.name}\n{sha256_file(manifest_path)}  {manifest_path.name}\n",
        encoding="ascii",
    )
    print(json.dumps(archive_manifest, sort_keys=True))


def safe_member(name, root):
    pure = pathlib.PurePosixPath(name)
    return not pure.is_absolute() and ".." not in pure.parts and len(pure.parts) == 2 and pure.parts[0] == root


def inspect_archive(path):
    expected_root = path.name.removesuffix(".tar.gz").removesuffix(".zip")
    members = []
    if path.name.endswith(".tar.gz"):
        with tarfile.open(path, "r:gz") as archive:
            for item in archive.getmembers():
                if not item.isfile() or item.issym() or item.islnk() or not safe_member(item.name, expected_root):
                    raise RuntimeError("archive contains an unsafe entry")
                members.append(item.name)
    elif path.name.endswith(".zip"):
        with zipfile.ZipFile(path) as archive:
            for item in archive.infolist():
                mode = item.external_attr >> 16
                if stat.S_ISLNK(mode) or item.is_dir() or not safe_member(item.filename, expected_root):
                    raise RuntimeError("archive contains an unsafe entry")
                members.append(item.filename)
    else:
        raise RuntimeError("unsupported archive format")
    expected = {
        f"{expected_root}/{name}"
        for name in (
            "LICENSE",
            "THIRD_PARTY_NOTICES.txt",
            "THIRD_PARTY_LICENSES.txt",
            "INSTALL.md",
            "RECOVERY.md",
            "install_release.sh",
            "install_release.ps1",
            "manifest.json",
        )
    }
    binaries = {f"{expected_root}/rt", f"{expected_root}/rt.exe"}
    if set(members) - binaries != expected or len(set(members) & binaries) != 1 or len(members) != 9:
        raise RuntimeError("archive inventory is not the exact release inventory")
    print(json.dumps({"archive": path.name, "entries": sorted(members)}, sort_keys=True))


def extract_archive(path, output):
    output = output.resolve()
    if output.exists() and (not output.is_dir() or any(output.iterdir())):
        raise RuntimeError("the package output directory must be absent or empty")
    inspect_archive(path)
    output = ensure_empty_output(output)
    if path.name.endswith(".tar.gz"):
        with tarfile.open(path, "r:gz") as archive:
            archive.extractall(output)
    else:
        with zipfile.ZipFile(path) as archive:
            archive.extractall(output)
    root = output / path.name.removesuffix(".tar.gz").removesuffix(".zip")
    if not root.is_dir() or root.is_symlink():
        raise RuntimeError("archive did not extract to its declared root")
    if os.name != "nt":
        executable = root / "rt"
        installer = root / "install_release.sh"
        executable.chmod(0o755)
        installer.chmod(0o755)
    print(json.dumps({"archive": path.name, "extracted": True}, sort_keys=True))


def compare(first_path, second_path):
    first = read_json(first_path)
    second = read_json(second_path)
    inputs = ["version", "source_sha", "target", "binary", "signing"]
    mismatched = [key for key in inputs if first.get(key) != second.get(key)]
    if mismatched:
        raise RuntimeError(f"package inputs differ: {', '.join(mismatched)}")
    identical = first.get("archive") == second.get("archive")
    print(
        json.dumps(
            {
                "format_version": 1,
                "source_sha": first["source_sha"],
                "target": first["target"],
                "archive_byte_identical": identical,
                "first_sha256": first["archive"]["sha256"],
                "second_sha256": second["archive"]["sha256"],
            },
            sort_keys=True,
        )
    )
    if not identical:
        raise RuntimeError("release archives are not byte-identical")


def parse_arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    create = commands.add_parser("create")
    create.add_argument("--build", required=True, type=pathlib.Path)
    create.add_argument("--output", required=True, type=pathlib.Path)
    inspect = commands.add_parser("inspect")
    inspect.add_argument("archive", type=pathlib.Path)
    compare_parser = commands.add_parser("compare")
    compare_parser.add_argument("first", type=pathlib.Path)
    compare_parser.add_argument("second", type=pathlib.Path)
    extract_parser = commands.add_parser("extract")
    extract_parser.add_argument("archive", type=pathlib.Path)
    extract_parser.add_argument("--output", required=True, type=pathlib.Path)
    return parser.parse_args()


def main():
    arguments = parse_arguments()
    try:
        if arguments.command == "create":
            package(arguments.build, arguments.output)
        elif arguments.command == "inspect":
            inspect_archive(arguments.archive)
        elif arguments.command == "compare":
            compare(arguments.first, arguments.second)
        else:
            extract_archive(arguments.archive, arguments.output)
    except (OSError, RuntimeError, subprocess.CalledProcessError, KeyError, ValueError, json.JSONDecodeError, UnicodeDecodeError) as error:
        print(f"Release packaging failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
