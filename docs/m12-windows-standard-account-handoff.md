# M12 Windows standard-account observation

## Recommended workflow

Prepare a self-contained synthetic test kit from the existing development account.
The operator creates a dedicated local standard account through normal Windows
account settings, signs in once, and runs the prepared launcher in that account's
ordinary interactive desktop. Codex does not need to be installed or authenticated
in the new account. Do not use administrator elevation or an alternate-credential
process as a substitute for this bounded interactive desktop observation.

The operator has authorized creating a dedicated standard account for this trial.
Account creation and password entry remain operator actions. The account exists
before product testing starts; the test scripts must not create, promote, remove
or change accounts. Never put credentials in a launcher, prompt, file or report.
No corporate host is part of this procedure.

This closes only the mapped package's standard-account and real-console observation
if it passes. A new account on the same developer machine is not an independent
clean machine or proof of an older Windows floor. Native Linux regression is
complete within its mapped Ubuntu scope. Hosted checks, practical agent trial
and global acceptance are tracked in the [current closure audit](m12-closure-audit.md).

## Exact candidate and source

Read [Windows validation](m12-windows-distribution-validation.md),
[runtime distribution](windows-runtime-distribution.md),
[the shared inventory](evidence/m12-windows-distribution-20260930/candidate-inventory.json),
the closure audit, AGENTS.md, TODO.md and the latest avances.md.
Fetch `fix/m12-final-validation`, preserving dirty checkouts and private evidence.
Use a separate clean copy when needed. This handoff changes no product code.

Use the existing final package built from
`f2b3f6f446466f5b3657c3cd91746735cab59d78`, not a new build or an older candidate:

| Item | SHA-256 |
| --- | --- |
| rt.exe | `ce1ae04c16430bcd37fa91810fd613f38ef06feca83d5cea29c1056525c92efa` |
| Archive | `039676c0665363fed10d0ed746e636a175351d8506a4076a75820fd7e9ef3d50` |
| conpty.dll | `39fba2713e2495117b1591ae8c32a3b904bea7aa66069cf7815e2844c76d75d8` |
| OpenConsole.exe | `b7fd936c2668b87b9ecf7b3366dc6568afc1c6f981874cba3e955a1c35cf8160` |

Find the retained package through its existing private build/command records first.
Search only plausible prior validation directories if its location is absent.
If the exact package cannot be found, report that single blocker. Do not rebuild,
substitute another executable or reattribute old passes. Keep the complete 13-file
package together with its notices, helper files and provenance. Source/report
commits after f2b3f6f do not replace its binary identity.

## Preparation by Codex in the existing account

Create a new dedicated folder under `%PUBLIC%\Documents`, for example
`Relayterm-M12-Standard-Observation`. Include only synthetic test material and
reviewed candidate files. Never copy developer homes, provider credentials,
Git credentials, old workspace databases, private reports or raw terminal logs.
Preserve all previous test folders instead of overwriting them.

Prepare these local-only components:

- `candidate`: the complete copied/extracted final package, with exact inventory
  and every manifest/hash verified before and after copying.
- `observe_windows_package.ps1`: the existing checked-in observer script, copied
  unchanged and identified by its source revision and SHA-256.
- `run-observation.cmd`: a double-click launcher using system PowerShell, paths
  anchored at its own directory and correctly quoted arguments.
- A local PowerShell wrapper if needed for account preflight, bounded invocation
  and sanitized result collection. This is a fixture, not production product code.
- `OPERATOR.txt`: short Spanish steps and expected markers. This is local operator
  guidance, not public technical documentation.
- `results`: a dedicated synthetic return location accessible from both accounts.

Verify the folder's actual access from the standard account when it starts. A
small synthetic write/read control should establish result-folder access before
launching the product. Do not broaden system permissions, take ownership of
other profiles or change global ACLs to make the kit work. If the return folder
is unavailable, identify only that path/access issue and provide a concrete fix
for the dedicated synthetic folder, not a profile-permission workaround.

Use only the system PowerShell and packaged runtime in the new account. Rust,
Cargo, Python, Git and Codex are not product/runtime prerequisites. Inspect the
actual System32 Visual C++ runtime prerequisite already recorded for this machine;
no helper DLL from a developer tool qualifies it. If a system prerequisite is
missing, stop with its exact diagnostic instead of elevating or installing it
silently. Do not change persistent PATH, execution policy or security settings.
RemoteSigned may be used only for the dedicated PowerShell process if needed;
respect any controlling policy and do not use Bypass or disable protections.

Make the launcher refuse both an elevated token and any token containing the
Administrators SID `S-1-5-32-544`, including disabled membership. The existing
observer prints `standard_account=false` for a filtered administrator but does
not itself reject that non-elevated case. The wrapper must reject it before
product execution, so an accidental run from the development account cannot be
reported as a standard-account pass. Use the existing whoami CSV SID inspection;
WindowsIdentity.Groups alone can omit disabled groups.

Validate syntax, quoting, exact package integrity, bounded administrative capture
and existing observer PrepareOnly in the development account where appropriate.
The launcher refusal there is expected evidence for its preflight, not a test
failure to bypass. Verify no credential is embedded. Do not claim the future
standard-account observation has passed during this preparation.

## Single interactive run in the standard account

The launcher should keep its console visible through the result and display:

- `standard_account=true` and `elevated=false` after verified preflight.
- The exact candidate SHA-256 above.
- The next action in Spanish, one checkpoint at a time.

Invoke the unchanged observer in interactive mode with the candidate directory
and expected hash. Do not use PrepareOnly as physical evidence. The observer
creates its own synthetic project, private state and neutral cmd.exe session
under the standard user's temporary directory. Its administrative command helper
is named Invoke-Admin because it uses the product administrative CLI; it does
not elevate Windows privileges.

The operator performs only the remaining console observations:

1. Press 3 and Enter to attach the prepared session. Type
   `echo M12-RUNTIME-OK` and Enter; confirm the marker appears correctly.
2. Resize the console narrower and wider; confirm redraw and fresh output remain
   usable. Do not repeat the full session-name/cursor/conflict/SSH matrix.
3. Type `exit` and Enter. Release with Ctrl-], detach with Esc and quit with q
   as guided. Confirm the outer console is restored and the launcher completes.

Guide the operator if terminal or keyboard behavior differs; do not relabel an
unobserved result. Capture only the synthetic session and restored console area.
No desktop/account name, private path, unrelated window or authentication content
belongs in a public screenshot. Automated readback handles identities, process
cleanup and command outcomes; screenshots are only for the physical boundary.

## Automatic result return

Keep raw state and the original operator-observation.json in the standard profile.
That file contains a native private-state path and must not be copied into Git.
The wrapper should read the exact observation file created by this invocation,
not whichever old file is newest, and write a sanitized result to the shared
results folder with only:

- Candidate/source and observer/launcher identities.
- Standard-account/elevation classification, real TUI exit code and command results.
- Explicit operator confirmations for input, resize and normal restoration.
- A distinction between automatically verified facts and physical confirmations.
- Any failure stage and numeric diagnostic, excluding private paths or usernames.

Bind collection to this run without altering the copied observer: identify its
specific spawned process and new fixture path, or use a scoped temporary root
owned by the standard account after validating native temporary-path semantics.
Preserve all failed attempts. Do not accept a stale JSON file, a timeout, a false
standard-account marker, missing confirmation or an incomplete cleanup as success.
If the observer fails before its normal cleanup, inspect and stop only positively
identified synthetic fixture processes, retaining the state. Never kill by name.

The operator can leave cropped screenshots in that synthetic results folder.
After completing the run, the operator returns once to the normal account and
asks the existing Codex chat to collect the results. No Notepad command transfer,
password handoff or repeated account switching is required.

## Reconciliation and delivery

Codex reads the shared results, verifies the candidate hash after use and confirms
that only the test fixture daemon/session was stopped. Retain the complete package
and private state. Record actual OS/account/terminal scope without exporting identities.
Update the Windows report, current closure table, TODO and append-only avances
only for passed proof. If a failure occurs, diagnose the exact new result rather
than restarting the whole platform battery. A new account on the same machine
must not be labelled an independent clean Windows installation.

Publish only sanitized evidence and documentation on a suitable Windows result
branch after repository, secret, whitespace and history checks pass. The copied
request below authorizes that branch push. Fetch/reconcile safely, never discard
work or force-push, and verify remote/local SHA equality. Do not merge main,
create a PR, tag, release or upload packages. Keep Linux's branch independent.

## Copy-paste request for Windows Codex

```text
Prepare the remaining Relayterm M12 standard-account and real-console
observation using docs/m12-windows-standard-account-handoff.md from the
fetched fix/m12-final-validation branch.

I will create a local standard account myself and enter its password.
Prepare everything from my existing development account so I change
accounts only once for the test and once to return. Do not create accounts,
request/store passwords, elevate or change system security settings.

Use the exact retained f2b3f6f Windows package, with rt.exe SHA-256
ce1ae04c16430bcd37fa91810fd613f38ef06feca83d5cea29c1056525c92efa.
Do not rebuild or copy rt.exe without its complete verified support files.

Prepare a self-contained synthetic kit in a dedicated Public Documents
folder: candidate, unchanged observer, double-click launcher, account/hash
preflight, Spanish operator steps and automatic sanitized result return.
The launcher must reject filtered administrator tokens as well as elevation.
Validate all automatable preparation before asking me to switch accounts.
The new account must not need Codex, Git, Rust, Cargo, Python or provider access.

Guide only input, narrow/wide redraw, normal exit and restoration. Automate
setup, integrity, readback, evidence and owned fixture cleanup. Collect results
when I return to this account; do not make me carry commands through Notepad.
Preserve every attempt and distinguish preparation, automated checks and
physical confirmations. Do not repeat completed performance/SSH/VM journeys.

After successful reconciliation, update public-safe reports, pending-only
TODO and append-only avances, commit and push the verified Windows result
branch, and verify its remote SHA. This authorizes that branch push only.
No main merge, PR, tag, release, binary upload or corporate-host access.
Communicate in Spanish; public documentation/code comments in English.
```
