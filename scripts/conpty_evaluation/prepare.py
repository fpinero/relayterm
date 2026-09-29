"""Prepare an isolated, pinned x64 Windows ConPTY evaluation dependency."""

import argparse
import hashlib
import io
import json
import platform
import subprocess
import tarfile
import urllib.request
import zipfile
from pathlib import Path


def download(url, expected, destination):
    with urllib.request.urlopen(url, timeout=60) as response:
        data = response.read(16 * 1024 * 1024 + 1)
    if len(data) > 16 * 1024 * 1024 or hashlib.sha256(data).hexdigest() != expected:
        raise RuntimeError("Package size or SHA-256 verification failed")
    destination.write_bytes(data)
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("artifact_root", type=Path)
    args = parser.parse_args()
    if platform.system() != "Windows" or platform.machine().lower() not in ("amd64", "x86_64"):
        raise RuntimeError("This evaluation is validated only on Windows x64")
    root = args.artifact_root.resolve()
    repo = Path(__file__).resolve().parents[2]
    if root == repo or repo in root.parents:
        raise RuntimeError("Choose a new artifact directory outside the checkout")
    root.mkdir(parents=True, exist_ok=False)
    crate = download(
        "https://static.crates.io/crates/portable-pty-psmux/portable-pty-psmux-0.9.7.crate",
        "db56953cd034f7147cbd4ff39797b5ed3aacbdbd93cca41d3a621aaa35be2b3c",
        root / "portable-pty-psmux-0.9.7.crate",
    )
    with tarfile.open(fileobj=io.BytesIO(crate), mode="r:gz") as archive:
        archive.extractall(root, filter="data")
    dependency = root / "portable-pty-psmux-0.9.7"
    patch = Path(__file__).with_name("runtime-adapter.patch")
    for arguments in (("--check",), ()):
        subprocess.run(
            ["git", "apply", *arguments, str(patch)], cwd=dependency, check=True, timeout=30
        )
    package = download(
        "https://api.nuget.org/v3-flatcontainer/microsoft.windows.console.conpty/"
        "1.24.260710001/microsoft.windows.console.conpty.1.24.260710001.nupkg",
        "175640566a3b59c4b132070ee96c2c77e5ab7edd2e92732a5eb3610bbf63d90e",
        root / "conpty-1.24.260710001.nupkg",
    )
    runtime = root / "runtime-x64"
    runtime.mkdir()
    with zipfile.ZipFile(io.BytesIO(package)) as archive:
        for member in (
            "runtimes/win-x64/native/conpty.dll",
            "build/native/runtimes/x64/OpenConsole.exe",
        ):
            (runtime / Path(member).name).write_bytes(archive.read(member))
    signatures = subprocess.run(
        ["pwsh", "-NoProfile", "-Command",
         "$ErrorActionPreference='Stop'; foreach ($f in @('conpty.dll','OpenConsole.exe')) { "
         "$s=Get-AuthenticodeSignature -LiteralPath $f; "
         "if ($s.Status -ne 'Valid' -or $s.SignerCertificate.Subject -notmatch '^CN=Microsoft Corporation,') "
         "{ throw 'Microsoft signature verification failed' }; "
         "$s | Select-Object Status,@{n='Signer';e={$_.SignerCertificate.Subject}} | ConvertTo-Json }"],
        cwd=runtime, capture_output=True, text=True, timeout=60, check=True,
    )
    (root / "signatures.json.log").write_text(signatures.stdout, encoding="utf-8")
    (root / "evaluation.toml").write_text(
        "[patch.crates-io]\nportable-pty-psmux = { path = "
        + json.dumps(dependency.as_posix()) + " }\n", encoding="utf-8"
    )
    files = [dependency / "src/win/psuedocon.rs", dependency / "src/win/conpty.rs", *runtime.iterdir()]
    (root / "identities.json").write_text(json.dumps({
        path.relative_to(root).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in files
    }, indent=2) + "\n", encoding="utf-8")
    print("Prepared isolated evaluation. No repository manifest or host settings changed.")


if __name__ == "__main__":
    main()
