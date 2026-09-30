"""Prepare explicitly requested, hash-pinned Windows x64 inputs outside source."""
import argparse
import hashlib
import io
import json
import pathlib
import urllib.request
import zipfile

import windows_runtime


def prepare(destination):
    destination = destination.resolve()
    if destination == windows_runtime.ROOT or windows_runtime.ROOT in destination.parents:
        raise RuntimeError("Choose a new private directory outside the checkout")
    destination.mkdir(parents=True, exist_ok=False)
    identity = json.loads(windows_runtime.PROVENANCE.read_text())
    with urllib.request.urlopen(identity["url"], timeout=60) as response:
        value = response.read(16 * 1024 * 1024 + 1)
    if len(value) > 16 * 1024 * 1024 or hashlib.sha256(value).hexdigest() != identity["package_sha256"]:
        raise RuntimeError("Pinned NuGet package verification failed")
    (destination / "runtime.nupkg").write_bytes(value)
    with zipfile.ZipFile(io.BytesIO(value)) as archive:
        for member in ("runtimes/win-x64/native/conpty.dll", "build/native/runtimes/x64/OpenConsole.exe"):
            (destination / pathlib.PurePosixPath(member).name).write_bytes(archive.read(member))
    windows_runtime.verify(destination, signatures=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("destination", type=pathlib.Path)
    prepare(parser.parse_args().destination)
