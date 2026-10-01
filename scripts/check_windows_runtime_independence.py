"""Audit the retained Windows package in a fresh, allowlisted process environment.

Python and system PowerShell are external test drivers, not product dependencies.
This audit does not deny arbitrary filesystem access or claim an OS sandbox.
"""

import argparse
import hashlib
import json
import ntpath
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time


EXPECTED_BINARY = "ce1ae04c16430bcd37fa91810fd613f38ef06feca83d5cea29c1056525c92efa"
EXPECTED_SOURCE = "f2b3f6f446466f5b3657c3cd91746735cab59d78"
COMMAND_LIMIT = 30
OUTPUT_LIMIT = 4 * 1024 * 1024

COLLECTOR = r'''
param([string]$CandidateRoot, [string]$FixtureRoot, [string]$RecordedPath)
$ErrorActionPreference = 'Stop'
function File-Hash([string]$Path) {
    # System PowerShell can inherit a foreign module path from the external driver.
    # Hash with the framework directly instead of relying on cmdlet auto-loading.
    $stream = [IO.File]::OpenRead($Path)
    $hash = [Security.Cryptography.SHA256]::Create()
    try { [BitConverter]::ToString($hash.ComputeHash($stream)).Replace('-', '').ToLowerInvariant() }
    finally { $stream.Dispose(); $hash.Dispose() }
}
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
$elevated = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
$groups = & "$env:SystemRoot\System32\whoami.exe" /groups /fo csv /nh
if ($LASTEXITCODE -ne 0) { throw 'Token inspection failed.' }
$sids = @(($groups | ConvertFrom-Csv -Header Name,Type,SID,Attributes).SID)
if (-not $sids.Count) { throw 'Missing token groups.' }
$binary = Join-Path $CandidateRoot 'rt.exe'
$all = @(Get-CimInstance Win32_Process)
$owned = @{}
$recorded = @()
if ($RecordedPath) { $recorded = @(Get-Content -Raw -LiteralPath $RecordedPath | ConvertFrom-Json) }
foreach ($p in $all) {
    if ($p.ExecutablePath -eq $binary -and $p.CommandLine -and
        $p.CommandLine.Contains($FixtureRoot)) { $owned[[string]$p.ProcessId] = $p }
    foreach ($prior in $recorded) {
        if ($p.ProcessId -eq $prior.pid -and
            $p.CreationDate.ToUniversalTime().ToString('o') -eq $prior.created_utc) {
            $owned[[string]$p.ProcessId] = $p
        }
    }
}
do {
    $added = $false
    foreach ($p in $all) {
        $key = [string]$p.ProcessId
        $parent = [string]$p.ParentProcessId
        if (-not $owned.ContainsKey($key) -and $owned.ContainsKey($parent) -and
            $p.CreationDate -ge $owned[$parent].CreationDate) {
            $owned[$key] = $p
            $added = $true
        }
    }
} while ($added)
$rows = @($owned.Values | ForEach-Object {
    $row = $_
    $process = Get-Process -Id $row.ProcessId -ErrorAction Stop
    $current = Get-CimInstance Win32_Process -Filter ('ProcessId=' + $row.ProcessId)
    if (-not $current -or $current.CreationDate -ne $row.CreationDate) {
        throw 'Process identity changed.'
    }
    $modules = @($process.Modules | ForEach-Object {
        @{ path=$_.FileName; sha256=(File-Hash $_.FileName) }
    })
    if (-not $modules.Count) { throw 'Missing module inventory.' }
    @{ pid=[int]$row.ProcessId; created_utc=$row.CreationDate.ToUniversalTime().ToString('o');
       executable=$row.ExecutablePath; modules=$modules }
})
@{ elevated=$elevated; standard_account=(-not $elevated -and -not ($sids -contains 'S-1-5-32-544'));
   os_version=[Environment]::OSVersion.Version.ToString(); processes=$rows } | ConvertTo-Json -Depth 8
'''


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_package(root, inventory):
    pins = inventory["installation"]["inventory"]
    if len(pins) != 13 or inventory["product_source"] != EXPECTED_SOURCE:
        raise RuntimeError("unexpected retained package identity")
    if pins.get("rt.exe") != EXPECTED_BINARY:
        raise RuntimeError("unexpected executable identity")
    entries = list(root.iterdir())
    if {p.name for p in entries} != set(pins):
        raise RuntimeError("package file inventory differs")
    for name, expected in pins.items():
        path = root / name
        if name != ntpath.basename(name) or ":" in name or path.is_symlink() or not path.is_file():
            raise RuntimeError("package contains a non-regular entry")
        if digest(path) != expected:
            raise RuntimeError("package hash differs")


def clean_environment(system_root, scratch):
    # Do not inherit provider credentials, runtime overrides or startup configuration.
    return {
        "SystemRoot": str(system_root),
        "WINDIR": str(system_root),
        "PATH": str(system_root / "System32"),
        "COMSPEC": str(system_root / "System32" / "cmd.exe"),
        "TEMP": str(scratch / "temp"),
        "TMP": str(scratch / "temp"),
        "USERPROFILE": str(scratch / "profile"),
        "HOME": str(scratch / "profile"),
        "LOCALAPPDATA": str(scratch / "profile" / "local"),
        "APPDATA": str(scratch / "profile" / "roaming"),
    }


def within_windows(path, root):
    path = ntpath.normcase(ntpath.normpath(path))
    root = ntpath.normcase(ntpath.normpath(root))
    if not ntpath.isabs(path) or not ntpath.isabs(root):
        return False
    try:
        return ntpath.commonpath([path, root]) == root
    except ValueError:
        return False


def classify_modules(snapshot, candidate, system_root, pins, runtime_pin):
    if snapshot["elevated"]:
        raise RuntimeError("use a non-elevated test driver")
    if not snapshot["processes"]:
        raise RuntimeError("missing owned daemon process")
    modules = []
    executables = set()
    for process in snapshot["processes"]:
        executables.add(ntpath.basename(process["executable"]).casefold())
        if not process["modules"]:
            raise RuntimeError("empty process module inventory")
        for item in process["modules"]:
            name = ntpath.basename(item["path"])
            if within_windows(item["path"], candidate):
                origin = "candidate"
                if ntpath.normcase(ntpath.dirname(item["path"])) != ntpath.normcase(ntpath.normpath(candidate)):
                    raise RuntimeError("nested candidate module is not in the inventory")
                expected = next((sha for file, sha in pins.items() if file.casefold() == name.casefold()), None)
                if expected != item["sha256"]:
                    raise RuntimeError("candidate module differs from package inventory")
            elif within_windows(item["path"], system_root):
                origin = "Windows"
            else:
                raise RuntimeError("module loaded outside candidate and Windows directories")
            if name.casefold() == "vcruntime140.dll":
                expected_path = ntpath.join(system_root, "System32", "VCRUNTIME140.dll")
                if ntpath.normcase(item["path"]) != ntpath.normcase(expected_path) or item["sha256"] != runtime_pin:
                    raise RuntimeError("system prerequisite differs from retained mapping")
            modules.append({"name": name, "origin": origin, "sha256": item["sha256"]})
    if not {"rt.exe", "openconsole.exe", "cmd.exe"}.issubset(executables):
        raise RuntimeError("missing daemon, ConPTY helper or synthetic shell")
    for required in ("conpty.dll", "vcruntime140.dll"):
        if not any(item["name"].casefold() == required for item in modules):
            raise RuntimeError("missing required runtime module")
    return modules


def run(arguments, environment, output_root, name, cwd, input_text=None):
    # Files avoid inherited pipe EOF waits from detached descendants.
    output_path = output_root / f"{name}.stdout.txt"
    error_path = output_root / f"{name}.stderr.txt"
    with output_path.open("xb") as output, error_path.open("xb") as errors:
        with tempfile.TemporaryFile() as source:
            if input_text is not None:
                source.write(input_text.encode("utf-8"))
                source.seek(0)
            completed = subprocess.run(
                [str(arg) for arg in arguments], env=environment, cwd=cwd,
                stdin=source if input_text is not None else subprocess.DEVNULL,
                stdout=output, stderr=errors, timeout=COMMAND_LIMIT, check=False,
            )
    content = output_path.read_bytes()
    if len(content) > OUTPUT_LIMIT:
        raise RuntimeError("command output exceeds test bound")
    if completed.returncode != 0:
        raise RuntimeError("bounded native command failed")
    return content.decode("utf-8-sig")


def audit(candidate, inventory_path, output_root):
    if os.name != "nt":
        raise RuntimeError("native Windows execution is required")
    if sys.maxsize <= 2**32:
        raise RuntimeError("the external Python driver must be 64-bit")
    inventory = json.loads(inventory_path.read_text(encoding="utf-8-sig"))
    candidate = candidate.resolve(strict=True)
    verify_package(candidate, inventory)
    repository = Path(__file__).resolve().parents[1]
    if output_root == candidate or candidate in output_root.parents or repository == output_root or repository in output_root.parents:
        raise RuntimeError("private output must be outside candidate and repository")
    system_root = Path(os.environ.get("SystemRoot", ""))
    if not system_root.is_absolute() or not (system_root / "System32" / "cmd.exe").is_file():
        raise RuntimeError("system Windows directory is unavailable")
    output_root.mkdir(parents=False, exist_ok=False)
    scratch = output_root / "fixture"
    scratch.mkdir()
    for folder in ("temp", "profile", "profile/local", "profile/roaming", "project", "candidate"):
        (scratch / folder).mkdir(parents=True, exist_ok=True)
    copied = scratch / "candidate"
    for path in candidate.iterdir():
        shutil.copyfile(path, copied / path.name)
    verify_package(copied, inventory)
    environment = clean_environment(system_root, scratch)
    binary = copied / "rt.exe"
    project = scratch / "project"
    home = scratch / "private"
    collector = output_root / "collect.ps1"
    collector.write_text(COLLECTOR, encoding="utf-8")
    powershell = system_root / "System32" / "WindowsPowerShell" / "v1.0" / "powershell.exe"
    result = {"status": "failed", "product_source": EXPECTED_SOURCE, "candidate_sha256": EXPECTED_BINARY,
              "archive_sha256_mapping": inventory["artifact"]["archive"]["sha256"],
              "archive_locally_rehashed": False, "automatic_only": True,
              "filesystem_sandbox": False, "global_M12_acceptance": False}
    stage = "preflight"
    initialized = False
    counter = 0

    def command(args, input_text=None):
        nonlocal counter
        counter += 1
        output = run([binary, "--workspace", project, "--home", home, "--format", "json", "--timeout", "5", *args],
                     environment, output_root, f"command-{counter}", project, input_text)
        response = json.loads(output)
        if response.get("ok") is not True:
            raise RuntimeError("native administrative command rejected")
        return response["result"]

    def collect(name, recorded=None):
        # The collector remains outside the product environment and uses only system tools.
        arguments = [powershell, "-NoLogo", "-NoProfile", "-ExecutionPolicy", "RemoteSigned", "-File", collector,
                     "-CandidateRoot", copied, "-FixtureRoot", scratch]
        if recorded is not None:
            arguments.extend(["-RecordedPath", recorded])
        output = run(arguments,
                     os.environ.copy(), output_root, name, project)
        return json.loads(output)

    try:
        preflight = collect("preflight")
        if preflight["elevated"]:
            raise RuntimeError("use a non-elevated test driver")
        version = tuple(int(part) for part in preflight["os_version"].split("."))
        if version[:2] != (10, 0) or version[2] < 19045:
            raise RuntimeError("environment is below the declared Windows floor")
        result.update(os_version=preflight["os_version"], standard_account=preflight["standard_account"], elevated=False,
                      windows_10_floor_execution=version[2] == 19045)
        stage = "help_version"
        for index, arg in enumerate(("--help", "--version")):
            run([binary, arg], environment, output_root, f"probe-{index}", project)
        stage = "workspace"
        # Cleanup is attempted even if initialization fails after starting a daemon.
        initialized = True
        if not command(["workspace", "init", "--name", "M12 runtime independence"]).get("started"):
            raise RuntimeError("fresh detached daemon did not start")
        status = command(["workspace", "status"])
        definition = scratch / "definition.json"
        definition.write_text(json.dumps({"display_name": "M12 runtime independence", "command": str(system_root / "System32" / "cmd.exe"),
            "arguments": ["/D", "/Q"], "environment_allowlist": [], "capabilities": ["interactive_terminal"], "enabled": True}), encoding="utf-8")
        registered = command(["agent", "register", "--expected-revision", status["revision"], "--file", definition])
        stage = "synthetic_pty"
        session = command(["session", "create", "--definition-id", registered["entity_ids"][0]])
        stage = "module_inventory"
        snapshot = collect("modules")
        result["modules"] = classify_modules(snapshot, str(copied), str(system_root), inventory["installation"]["inventory"],
            inventory["pe"]["system_prerequisites"][0]["sha256"])
        identities = {(p["pid"], p["created_utc"]) for p in snapshot["processes"]}
        recorded = output_root / "recorded-processes.json"
        recorded.write_text(json.dumps([{"pid": pid, "created_utc": created} for pid, created in sorted(identities)]), encoding="utf-8")
        stage = "synthetic_pty_exit"
        command(["session", "input", session["session_id"], "--stdin"], "echo M12-INDEPENDENCE-OK\r\nexit\r\n")
        deadline = time.monotonic() + 15
        while True:
            sessions = command(["session", "list"])
            observed = next((row for row in sessions["items"] if row["session_id"] == session["session_id"]), None)
            if observed and observed["status"] == "exited" and observed["exit_code"] == 0:
                break
            if time.monotonic() >= deadline:
                raise RuntimeError("synthetic PTY did not exit cleanly")
            time.sleep(0.2)
        stage = "shutdown"
        if command(["daemon", "stop"])["lifecycle"] != "stopped":
            raise RuntimeError("daemon did not stop")
        initialized = False
        after = collect("after", recorded)
        live = {(p["pid"], p["created_utc"]) for p in after["processes"]}
        if live or identities.intersection(live):
            raise RuntimeError("owned fixture processes remain")
        stage = "return_integrity"
        verify_package(candidate, inventory)
        verify_package(copied, inventory)
        result.update(status="passed", recorded_processes=len(identities), remaining_processes=0,
            clean_environment=True, audited_module_origins=True, package_integrity=True,
            help_version=True, initialization_ipc=True, synthetic_pty_exit=True, orderly_shutdown=True)
    except Exception as error:
        result.update(failure_stage=stage, error_type=type(error).__name__)
        raise
    finally:
        if initialized:
            try:
                command(["daemon", "stop", "--terminate-sessions"])
                result["failure_cleanup"] = "owned daemon stop requested"
            except Exception:
                result["failure_cleanup"] = "unverified; retain fixture and investigate"
        (output_root / "result.json").write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--inventory", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    try:
        result = audit(args.candidate, args.inventory, args.output.resolve())
    except (OSError, ValueError, RuntimeError, KeyError, subprocess.SubprocessError) as error:
        print(f"Windows runtime audit did not pass: {type(error).__name__}", file=sys.stderr)
        return 1
    print(json.dumps({"status": result["status"], "global_M12_acceptance": False}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
