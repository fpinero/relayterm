"""Verify and stage pinned runtime files, never install or download at runtime."""
import argparse
import hashlib
import json
import pathlib
import shutil
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[1]
PROVENANCE = ROOT / "third_party/conpty/provenance.json"
INVENTORY = ("conpty.dll", "OpenConsole.exe", "CONPTY_LICENSE.txt", "CONPTY_PROVENANCE.json")


def verify(directory, signatures=False):
    identity = json.loads(PROVENANCE.read_text())
    for name, digest in identity["files"].items():
        path = directory / name
        if not path.is_file() or path.is_symlink() or hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            raise RuntimeError(f"Pinned runtime integrity failed: {name}")
    if signatures:
        result = subprocess.run([
            "pwsh", "-NoProfile", "-NonInteractive", "-Command",
            "$ErrorActionPreference='Stop'; foreach ($f in @('conpty.dll','OpenConsole.exe')) { "
            "$s=Get-AuthenticodeSignature -LiteralPath $f; "
            "if ($s.Status -ne 'Valid' -or $s.SignerCertificate.Subject -notmatch '^CN=Microsoft Corporation,') "
            "{ throw 'Microsoft signature verification failed' }; "
            "$s | Select-Object Status,@{n='Signer';e={$_.SignerCertificate.Subject}} | ConvertTo-Json }"
        ], cwd=directory, capture_output=True, text=True, timeout=60, check=True)
        print(result.stdout)
    return identity


def stage(source, destination):
    if source is None:
        raise RuntimeError("Windows build requires --runtime-root with explicitly prepared pinned inputs")
    identity = verify(source, signatures=True)
    destination.mkdir(parents=True, exist_ok=True)
    for name in identity["files"]:
        target = destination / name
        if target.exists():
            if hashlib.sha256(target.read_bytes()).hexdigest() != identity["files"][name]:
                raise RuntimeError("Refusing to replace a different runtime")
        else:
            shutil.copyfile(source / name, target)
    for source_path, name in ((ROOT / "third_party/conpty/LICENSE.txt", "CONPTY_LICENSE.txt"),
                              (PROVENANCE, "CONPTY_PROVENANCE.json")):
        target = destination / name
        value = source_path.read_text(encoding="utf-8").encode("utf-8")
        if name == "CONPTY_LICENSE.txt" and hashlib.sha256(value).hexdigest() != identity["license_sha256"]:
            raise RuntimeError("Pinned runtime license hash mismatch")
        if target.exists() and target.read_bytes() != value:
            raise RuntimeError("Refusing to replace different runtime metadata")
        if not target.exists():
            target.write_bytes(value)
    return identity


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=pathlib.Path)
    parser.add_argument("destinations", nargs="+", type=pathlib.Path)
    args = parser.parse_args()
    for destination in args.destinations:
        stage(args.source.resolve(), destination.resolve())
