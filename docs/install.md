# Install a portable Relayterm candidate

Relayterm's local candidate is a portable archive for one declared native target. It is unsigned and has not been published as a release. Obtain the archive, its matching manifest, and `SHA256SUMS` from the same reviewed candidate handoff. A SHA-256 match detects corruption, but does not independently authenticate the publisher.

## Verify before extraction

Confirm the target in the manifest matches the machine. Linux uses `x86_64-unknown-linux-gnu`, Apple silicon uses `aarch64-apple-darwin`, and 64-bit Windows uses `x86_64-pc-windows-msvc`.

On Linux:

```sh
sha256sum --check SHA256SUMS
```

On macOS:

```sh
shasum -a 256 --check SHA256SUMS
```

In PowerShell, compare both reported hashes with `SHA256SUMS`:

```powershell
Get-FileHash -Algorithm SHA256 -LiteralPath .\relayterm-0.1.0-x86_64-pc-windows-msvc.zip
Get-FileHash -Algorithm SHA256 -LiteralPath .\relayterm-0.1.0-x86_64-pc-windows-msvc.manifest.json
```

From `cmd.exe`:

```batch
certutil -hashfile relayterm-0.1.0-x86_64-pc-windows-msvc.zip SHA256
certutil -hashfile relayterm-0.1.0-x86_64-pc-windows-msvc.manifest.json SHA256
```

Stop on any mismatch. Preserve the files for inspection and obtain a verified candidate again. Do not run or install a mismatched archive.

Extract into a new directory. The archive inspector used by CI rejects absolute paths, parent traversal, links, extra files, missing notices, and an unexpected executable name. Do not merge a release archive into an existing directory.

## Discover naming conflicts

Before adding a directory to `PATH`, inspect all current `rt` resolutions and the intended destination file. A successful version response from another command is not evidence that Relayterm was installed.

For POSIX shells:

```sh
type -a rt 2>/dev/null || true
command -V rt 2>/dev/null || true
test ! -e "$HOME/.local/bin/rt" && test ! -L "$HOME/.local/bin/rt"
```

For PowerShell:

```powershell
Get-Command rt -All -ErrorAction SilentlyContinue
$Destination = Join-Path $env:LOCALAPPDATA "Relayterm\bin\0.1.0"
Test-Path -LiteralPath (Join-Path $Destination "rt.exe")
```

For `cmd.exe`:

```batch
where rt
if exist "%LOCALAPPDATA%\Relayterm\bin\0.1.0\rt.exe" exit /b 1
```

Shell functions and aliases can take precedence over executable files. If any unrelated command, function, alias, or destination file exists, do not overwrite or delete it. Keep using Relayterm by absolute path, or select a dedicated directory and make a deliberate `PATH` precedence decision.

## Install for one user

No administrator access or shell-profile modification is required. A versioned directory keeps upgrades and rollback explicit.

On Linux or macOS, create a destination and use the helper from the extracted archive:

```sh
install_root="$HOME/.local/opt/relayterm/0.1.0"
mkdir -p "$install_root"
./relayterm-0.1.0-aarch64-apple-darwin/install_release.sh ./relayterm-0.1.0-aarch64-apple-darwin "$install_root"
"$install_root/rt" --version
```

Use the archive root matching the current target. The helper refuses links, a missing executable, a missing destination directory, and any existing destination `rt`. It stages a private file and renames it only after a complete copy.

In PowerShell:

```powershell
$InstallRoot = Join-Path $env:LOCALAPPDATA "Relayterm\bin\0.1.0"
New-Item -ItemType Directory -Force -Path $InstallRoot | Out-Null
.\relayterm-0.1.0-x86_64-pc-windows-msvc\install_release.ps1 -ReleaseRoot .\relayterm-0.1.0-x86_64-pc-windows-msvc -Destination $InstallRoot
& (Join-Path $InstallRoot "rt.exe") --version
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
```

From `cmd.exe`, invoke the installed binary by its exact path:

```batch
"%LOCALAPPDATA%\Relayterm\bin\0.1.0\rt.exe" --version
if errorlevel 1 exit /b %errorlevel%
```

Open a fresh shell after a deliberate `PATH` change and inspect resolution again. Verify both the absolute path and version. Relayterm never edits a shell profile automatically.

Paths containing spaces and ordinary non-ASCII characters are supported. Always quote them. A read-only destination or partial copy must report failure, and an existing sentinel must remain byte-for-byte unchanged.

## Platform warnings

The initial local candidate is unsigned. macOS Gatekeeper and Windows reputation controls may warn or block it depending on how it was obtained and local policy. Inspect the candidate hash and source mapping. Do not disable global platform protections. Signing and notarization require a later maintainer publication decision.

## Remove only the installed executable

Stop each selected workspace daemon deliberately before removing the binary. Do not kill processes by name.

On POSIX systems:

```sh
"$install_root/rt" --workspace /path/to/project daemon stop
rm -- "$install_root/rt"
rmdir -- "$install_root"
```

In PowerShell:

```powershell
& (Join-Path $InstallRoot "rt.exe") --workspace C:\path\to\project daemon stop
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Remove-Item -LiteralPath (Join-Path $InstallRoot "rt.exe")
Remove-Item -LiteralPath $InstallRoot
```

Removal does not delete private workspace state, backups, project files, Git repositories, worktrees, or unrelated `PATH` entries. A state-purge operation is outside the MVP.
