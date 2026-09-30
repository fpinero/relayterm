# Windows runtime distribution

## Candidate contract

M12 remains open. The ordinary x64 candidate bundles Microsoft.Windows.Console.ConPTY
1.24.260710001 with the reviewed portable-pty-psmux 0.9.7 adapter. The runtime is
loaded only beside the executable. There is no environment-selected backend,
system fallback, runtime download or automatic updater. Windows 10 22H2 remains
the support floor; Windows Server CI, other architectures and clean-machine proof
are separate evidence. See [the decision](decisions/0009-windows-packaged-conpty.md).

## Explicit build preparation

Prepare in a new private directory outside source. This verifies the exact NuGet
package hash and the Microsoft signatures of both x64 files. It installs nothing.

```powershell
python scripts/prepare_windows_runtime.py C:\validation\conpty-inputs
cargo build --workspace --locked
python scripts/windows_runtime.py C:\validation\conpty-inputs target/debug target/debug/deps
cargo test --workspace --locked -- --test-threads=1
```

Stage release and release/deps similarly after prebuilding release harnesses.
Do not use the experimental runner to certify ordinary distribution. Windows
harness executables create their own outer PTYs and also need the pinned files.
Test-only hooks are excluded from production builds and acceptance measurements.

For clean release builds, use the existing wrapper and supply the prepared root:

```powershell
python scripts/build_release.py build --target x86_64-pc-windows-msvc --output C:\validation\build-a --runtime-root C:\validation\conpty-inputs --offline
```

RELAYTERM_BUILD_RUNTIME_ROOT is an alternative build-tool input, never a product
runtime override. It also supplies native installer test fixtures. Keep source
clean and committed, and compare two builds from separate clean checkouts and
fresh output directories before packaging. The archive contains exactly 13 files.
CONPTY_PROVENANCE.json records package identity, source URL, license and file hashes.
CONPTY_LICENSE.txt carries the upstream Microsoft MIT text. Do not distribute
private preparation artifacts, development harnesses or runtime databases.

## Installation and failure policy

Extract the reviewed archive and use install_release.ps1 with a preexisting
version-specific destination. The installer validates the fixed file set and
hashes, refuses all owned collisions and installs rt.exe last. Reinstallation
does not overwrite an existing package. A failed copy rolls back files created
by that attempt. Unrelated files remain untouched. Keep all support files together.

The packaged ConPTY DLL loader excludes CWD and PATH from dependency lookup. Both runtime files
must be regular x64 PE files with the compiled pinned hashes. Read handles prevent
replacement during use. Operational commands and the internal daemon validate before side effects.
Private stderr retains runtime_unavailable, the validation stage and numeric
Windows loader errors. Help/version remain usable for diagnosis. Missing runtime
files never qualify acceptance through a fallback. An altered manifest cannot
override the compiled hash allowlist.

The Microsoft Visual C++ v14 x64 Redistributable is a separate system prerequisite.
Relayterm does not install it. Follow [Microsoft's official guidance](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist)
if the actual PE imports require it and System32 lacks a sufficient runtime.
Private DLLs in unrelated developer tools do not establish that prerequisite.

Stop the exact installation's daemons and clients before owned removal:

```powershell
& C:\validation\installed\install_release.ps1 -Destination C:\validation\installed -Remove
```

Removal verifies all owned hashes before deleting only the fixed inventory.
Private workspace state and foreign markers survive. A changed owned file causes
refusal for review. Maintainers update pins, rebuild and repeat affected native,
package and helper-resource tests for each runtime update. No independent helper
upgrade is supported.

## Operator observation

Use scripts/observe_windows_package.ps1 with the reviewed candidate directory and
its exact SHA-256. PrepareOnly verifies synthetic commands without claiming a real
console observation. The interactive mode uses a neutral cmd.exe prompt, input,
resize, normal exit and restoration. It retains synthetic private state.

Token inspection includes disabled administrative group SIDs from whoami CSV.
WindowsIdentity.Groups alone can omit those groups on a filtered token. A printed
standard_account=false does not qualify a standard-account journey. Use an
existing actual standard account, without creating accounts or changing policy.

Administrative JSON capture reads one complete line and waits for the directly
launched process with bounded deadlines. Windows PowerShell 5 pipeline capture
was observed waiting for EOF while a detached daemon was alive. The observer does
not wait for descendant pipe EOF. It requires no Python or system installation.
RemoteSigned, when needed, applies only to the dedicated PowerShell process.

Native results and exact artifact mappings are in [the Windows validation report](m12-windows-distribution-validation.md).
