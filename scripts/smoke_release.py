"""Smoke-test an extracted Relayterm archive outside the source checkout."""

import argparse
import json
import os
import pathlib
import platform
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time
import zipfile

import package_release


def extract(archive, destination):
    package_release.inspect_archive(archive)
    if archive.name.endswith(".tar.gz"):
        with tarfile.open(archive, "r:gz") as source:
            source.extractall(destination)
    else:
        with zipfile.ZipFile(archive) as source:
            source.extractall(destination)
    roots = list(destination.iterdir())
    if len(roots) != 1 or not roots[0].is_dir():
        raise RuntimeError("archive did not extract to one release root")
    return roots[0]


def invoke(binary, project, home, arguments, *, input_text=None, expect=0):
    environment = os.environ.copy()
    if os.name == "nt":
        system_root = pathlib.Path(environment["SystemRoot"])
        environment["PATH"] = os.pathsep.join(
            [os.fspath(system_root / "System32"), os.fspath(system_root)]
        )
    else:
        environment["PATH"] = "/usr/bin:/bin"
    result = subprocess.run(
        [
            os.fspath(binary),
            "--workspace",
            os.fspath(project),
            "--home",
            os.fspath(home),
            "--format",
            "json",
            *arguments,
        ],
        input=input_text,
        capture_output=True,
        text=True,
        env=environment,
        check=False,
    )
    if result.returncode != expect:
        raise RuntimeError(f"installed command failed in stage {arguments[0]}")
    value = json.loads(result.stdout)
    if expect == 0 and not value.get("ok"):
        raise RuntimeError(f"installed command rejected stage {arguments[0]}")
    return value.get("result")


def smoke(archive):
    scratch_parent = pathlib.Path("/tmp") if os.name != "nt" and pathlib.Path("/tmp").is_dir() else None
    temporary = pathlib.Path(tempfile.mkdtemp(prefix="rtm12-", dir=scratch_parent))
    project = temporary / "project"
    home = temporary / "private"
    extracted = temporary / "extracted"
    project.mkdir()
    home.mkdir(mode=0o700)
    extracted.mkdir()
    release_root = extract(archive.resolve(), extracted)
    binary = release_root / ("rt.exe" if os.name == "nt" else "rt")
    daemon_started = False
    try:
        for argument in ("--help", "--version"):
            result = subprocess.run(
                [os.fspath(binary), argument],
                cwd=temporary,
                env={
                    **os.environ,
                    "PATH": (
                        os.pathsep.join(
                            [
                                os.fspath(pathlib.Path(os.environ["SystemRoot"]) / "System32"),
                                os.fspath(pathlib.Path(os.environ["SystemRoot"])),
                            ]
                        )
                        if os.name == "nt"
                        else "/usr/bin:/bin"
                    ),
                },
                stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE,
                check=False,
            )
            if result.returncode != 0:
                raise RuntimeError(f"installed {argument} failed")
        initialized = invoke(binary, project, home, ["workspace", "init", "--name", "Release smoke"])
        daemon_started = True
        if not initialized.get("started"):
            raise RuntimeError("installed daemon did not start")
        status = invoke(binary, project, home, ["workspace", "status"])
        revision = status["revision"]
        if os.name == "nt":
            command = os.fspath(pathlib.Path(os.environ["SystemRoot"]) / "System32" / "cmd.exe")
            arguments = ["/Q"]
            input_text = "echo release-smoke-ok\r\nexit\r\n"
        else:
            command = "/bin/sh"
            arguments = []
            input_text = "echo release-smoke-ok\nexit\n"
        definition_file = temporary / "definition.json"
        definition_file.write_text(
            json.dumps(
                {
                    "display_name": "Release smoke shell",
                    "command": command,
                    "arguments": arguments,
                    "environment_allowlist": ["PATH"],
                    "capabilities": ["terminal"],
                    "enabled": True,
                }
            ),
            encoding="utf-8",
        )
        registered = invoke(
            binary,
            project,
            home,
            ["agent", "register", "--expected-revision", revision, "--file", os.fspath(definition_file)],
        )
        definition_id = registered["entity_ids"][0]
        session = invoke(binary, project, home, ["session", "create", "--definition-id", definition_id])
        invoke(
            binary,
            project,
            home,
            ["session", "input", session["session_id"], "--stdin"],
            input_text=input_text,
        )
        deadline = time.monotonic() + 10
        observed = None
        while time.monotonic() < deadline:
            sessions = invoke(binary, project, home, ["session", "list"])
            observed = next(
                (item for item in sessions["items"] if item["session_id"] == session["session_id"]),
                None,
            )
            if observed and observed["status"] == "exited":
                break
            time.sleep(0.1)
        if not observed or observed["status"] != "exited" or observed["exit_code"] != 0:
            raise RuntimeError("installed synthetic PTY did not exit cleanly")
        stopped = invoke(binary, project, home, ["daemon", "stop"])
        daemon_started = False
        if stopped["lifecycle"] != "stopped":
            raise RuntimeError("installed daemon did not stop")
        print(
            json.dumps(
                {
                    "format_version": 1,
                    "target_platform": platform.system().lower(),
                    "help": "passed",
                    "version": "passed",
                    "workspace_initialization": "passed",
                    "detached_daemon": "passed",
                    "synthetic_pty": "passed",
                    "orderly_shutdown": "passed",
                },
                sort_keys=True,
            )
        )
    finally:
        if daemon_started:
            try:
                invoke(binary, project, home, ["daemon", "stop", "--terminate-sessions"])
            except Exception:
                pass
        shutil.rmtree(temporary, ignore_errors=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=pathlib.Path)
    arguments = parser.parse_args()
    try:
        smoke(arguments.archive)
    except (OSError, RuntimeError, subprocess.SubprocessError, json.JSONDecodeError) as error:
        print(f"Release smoke test failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
