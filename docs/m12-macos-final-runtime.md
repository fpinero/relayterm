# M12 macOS isolated runtime verification

## Result and evidence boundary

On 2026-09-23 the retained final macOS package passed the installed workflow in
a disposable native runtime with enforced filesystem, executable and TCP access
restrictions. This closes the macOS runtime portion of M12.NATIVE. It is stronger
than PATH filtering: positive/negative controls verified that the product and
its supervised shell could not read external files or use developer tools.

Environment: macOS 26.5.2, build 25F84, Apple arm64. This uses the host operating
system with an isolated writable root and explicit system dependency allowlist,
not a VM, separate OS installation or separate user account. It does not claim
new clean-runtime evidence on macOS 14. The existing macOS 14 hosted results and
three-platform physical observations retain their original identities.

This is test-environment isolation, not a Relayterm security feature or a claim
that ordinary agent sessions are sandboxed. The separate SSH presentation and
Windows runtime/performance requirements remain open.

## Exact artifact

| Item | Identity |
| --- | --- |
| Product source | `691a8fbb45980658b98d647a85ea8305b2325938` |
| Target/version | `aarch64-apple-darwin`, 0.1.0, unsigned production build |
| Archive SHA-256 | `4d8c1dda4d7fa9ea412928365e8dbc9b550f02d0f3766b3462c1cd7832955007` |
| Installed executable SHA-256 | `4b5ae7ef4e38382c82732cf785b2ae09af34d8cf614b6fe11d0c1b4efbb80060` |
| Private isolation profile SHA-256 | `1c9dabfbb2183b63dfc54c20ecf16bf2056044722d3307eefebac95b17135983` |

The archive was reused, not rebuilt. Its exact nine regular entries were checked
before extraction. The packaged installer installed the executable into the
fresh runtime. Installed and retained hashes matched before and after the trial.
Native dependencies remain libiconv.2.dylib and libSystem.B.dylib as recorded in
[the candidate report](m12-macos-correction-candidate.md).

## Isolation method and controls

A private temporary root contains a fresh project, HOME, TMPDIR, Relayterm home,
extracted package and install directory. The environment is explicitly supplied
with those paths, PATH, SHELL, LANG and TERM. Existing user configuration and
provider credentials are not imported.

Every product invocation uses the system sandbox-exec with a deny-by-default
profile. Its file-content allowlist contains the runtime root, system libraries,
system resources, devices and explicitly selected native shell/install commands.
Metadata lookup is permitted separately. Writes are limited to the runtime and
devices. Native Unix IPC, process operations needed for the owned daemon/children,
system queries and macOS service lookup are permitted; TCP access is not.
The named system commands include sh/bash/zsh, cp/chmod/mv/rm/mkdir/cat and the
nc control probe. Developer directories, Homebrew and the repository are absent
from the file-content/executable allowlist.

Python orchestration and a VT100 observer compiled from an already cached test
dependency ran outside the restricted runtime. They are test drivers, not runtime
dependencies, package members or synthetic production capabilities. The product
and daemon-owned shell stayed inside the inherited isolation profile.

| Control | Result |
| --- | --- |
| External synthetic file readable outside isolation | Passed positive control |
| Same external file read through restricted cat | Denied |
| Repository README read inside runtime | Denied |
| Homebrew Python, Cargo and system Python execution | Denied |
| Loopback TCP probe outside isolation | Connected to the owned temporary control socket |
| Same probe inside isolation | Denied |
| Daemon-owned shell reading the external synthetic file | Denied, corroborated by its marker in the permitted project |
| Installed command discovery | Bash without startup files and sh resolved the exact installed rt |

The control socket was closed after the probe. No SSH listener, firewall change,
OS account change or persistent service was introduced by this runtime test.

## Installed workflow results

- Help/version, private initialization, detached daemon startup and native IPC
  passed through the restricted installed executable.
- The packaged installer refused an unrelated existing rt and preserved its
  sentinel bytes. Actual shell discovery selected the intended installation.
- Three real sh PTYs launched, accepted input and retained names and creation
  order. No provider command, account, source checkout or compiler was required.
- Task readiness, first claim, rejected competing claim, progress, structured
  handover, successor claim and completion passed with authoritative readback.
- An 80-by-24 outer PTY observed CURRENT, named sessions and WRITER using a real
  VT100 parser. The shell inherited external-file denial. Normal TUI exit returned
  zero, emitted alternate-screen restoration, and left the ordered session
  snapshot unchanged. Existing physical observations are reused explicitly.
- Live backup captured the active workspace. After stopping only the owned
  original daemon/sessions, restore into a new short private home succeeded.
  Task data, complete history and claim entries matched. Names, order, launch
  snapshots and session/instance identities were preserved; former live instances
  were honestly marked lost. Both daemons were then stopped.
- A separate installed copy was removed after its hash was checked. Its unrelated
  sibling, the main tested installation, original/restored homes and backup were
  preserved. Final process inspection found zero processes using this runtime's
  executable. Its hash remained unchanged.

Optional Git worktree behavior remains covered by the existing exact-installed
native worktree gate. Git was not added to this restricted runtime.

## Retained unsuccessful attempts

The following attempts remain in private evidence and are not labelled passes:

1. The initial profile denied a root-directory read needed by the system shell.
   The shell aborted before product launch. A literal root-directory read was
   added without exposing repository, user or developer file contents; subsequent
   positive/negative controls established the corrected boundary.
2. The runner requested task status completed instead of the actual done value,
   and omitted the required revision on one claim-history read. Those requests
   were rejected. The same fixture continued with correct arguments and no
   duplicate prior mutations.
3. Two TUI observers searched raw diff bytes for complete strings. They timed
   out although the terminal drew the cells. A cached VT100 parser reconstructed
   the display before assertions. The passing observer is a corrected harness
   run, not a product correction or additional full native physical journey.
4. The first restored home produced a 106-byte endpoint, exceeding the existing
   100-byte IPC path bound. Startup was rejected. The restored data was preserved;
   another fresh shorter home restored the same backup and passed. This is the
   documented short-private-path requirement, not data loss or a new regression.
5. The runner initially looked for session identity in the wrong ordered-list
   object level. The assertion was corrected to inspect the nested instance and
   compare all preserved fields while allowing the documented loss timestamps.

The original state, both restored copies, backup, command journal, terminal
captures, scripts, isolation profile and evidence hash index remain private.
The corrected scripts were executed by checkpoint and syntax-checked; a second
fresh complete execution is not claimed. No artifact or production source changed.
