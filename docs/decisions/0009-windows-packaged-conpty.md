# 0009: Windows package-local ConPTY

## Context

Accepted engineering decision for the M12 candidate. Global acceptance remains open.

## Decision

Keep portable-pty-psmux 0.9.7 vendored under third_party with a repository-relative
Cargo patch. The original crate SHA-256 is
db56953cd034f7147cbd4ff39797b5ed3aacbdbd93cca41d3a621aaa35be2b3c.
Unix implementation is preserved. Windows x64 uses Microsoft.Windows.Console.ConPTY
1.24.260710001, whose hashes and MIT declaration are recorded in provenance.json.
The package declares Windows 10 build 17763 or above; Relayterm retains its stricter
Windows 10 22H2 floor. ARM64 and x86 distribution are not supported.

Resolve both runtime files beside the process executable. Ignore PATH, CWD and
experimental environment selection. Reject nonregular files, wrong PE architecture
and differing pinned SHA-256. Hold read handles that deny write/delete replacement.
LoadLibraryExW uses DLL_LOAD_DIR and SYSTEM32 search flags, without persistent
search-policy changes. Resolve the three Conpty-prefixed WINAPI exports. Keep the
module and verified file handles alive for the process. Missing prerequisites and
loader errors retain native numeric diagnostics, without system-backend fallback.
Operational CLI commands and the internal daemon check before side effects. Help/version remain available.

The ABI reference is the pinned conpty.h. Flags are zero, since 0x8 means glyph
width rather than passthrough. Keep the input endpoint alive with the master and
answer DA1 once, within 4096 inspected bytes, without changing output bytes.

Distribute rt.exe, conpty.dll and OpenConsole.exe with runtime license/provenance
in the exact 13-file archive. The Windows installer verifies the fixed inventory,
checks pinned runtime hashes, rejects all owned collisions, installs the executable
last and rolls back files created by that attempt. Removal checks all owned files
before deleting them and preserves foreign files and private state. Install new
versions in separate directories. No runtime network download or automatic update.
Relayterm release maintainers own updates, which require pin changes and retesting.

## Invariants and behavior

The pinned NuGet manifest declares MIT and identifies Microsoft terminal upstream.
The MIT text from the official upstream is included verbatim as CONPTY_LICENSE.txt.
Preparation and build staging verify Microsoft Authenticode signatures plus package
and file hashes. Runtime trust is the compiled, reviewed signed-input hash allowlist,
not an ambient certificate store decision or mutable package manifest. Runtime
hashing does not promise protection from a same-user administrator or debugger.
The unsigned Relayterm executable remains a separate signing boundary.

## Alternatives

Keeping the experimental private Cargo override cannot reproduce ordinary packages.
Using the system backend has no qualified sustained proof here. Upgrading inputs
would require a new evaluation. Retain the reviewed pins and fail closed.

## Consequences

Windows distribution requires package-local support files and a supported x64
Visual C++ runtime. Development harnesses also need the pinned files beside their
executables. Preparation is explicit, isolated and separate from Cargo resolution.
Resource accounting charges daemon-owned helpers to the unchanged daemon memory
budget, while also reporting separate helper handles, count and lifecycle. The
outer test terminal helper is harness infrastructure and must be reported separately.
Native and extracted-package evidence is recorded in the Windows distribution
report. Mac/Linux need affected dependency/tooling regressions before global M12
closure; prior experimental and historical results remain distinct.

## Verification and ownership

M12.CONPTY-DISTRIBUTION owns native integration and package/resource verification.
M12.FINAL-SOURCE-GATES owns cross-platform reconciliation. Maintainers own pin
updates and publication. Operator account and real-console observations remain
explicitly separate from autonomous checks.
