# M12 Windows 11 ACL review and diagnostic contract

## Latest evidence on 2026-09-25

The [prepared Windows VM reconciliation](m12-windows11-vm-reconciliation.md)
records passing standard-account runtime, coordination, real-console TUI and
populated recovery gates, with original init response capture unavailable. Earlier
Windows-runtime pending statements below are historical checkpoints. Corporate
ACL failure, debug latency, sustained-load measurement and final acceptance remain
open. Reuse completed Mac/SSH and VM journeys; do not infer FortiClient causality.

## Decision and provenance

On 2026-09-24, reviewed both operator-supplied reports in full:
`m12-windows11-independent-runtime-report.md` and
`m12-windows11-continuation-for-mac.md`. Originals remain outside the repository,
as requested. Only these sanitized reports were supplied, not their private
JSON, scripts, packages or process traces. Reported Windows observations below
are imported evidence, not tests performed from this Mac.

The independent Windows 11 gate **failed**. Six initialization attempts returned
exit 1 and `invalid_location`; downstream daemon, three ConPTY sessions,
coordination, TUI continuity and backup/restore are blocked, not passed.
Installation, resolution, help/version, collision preservation and bounded
startup dependency checks passed. Windows 11 Pro 25H2 x64 build 26200.8655 used
a standard, non-elevated account without a checkout or newly installed toolchain.
This is neither a clean Windows 10 proof nor complete runtime independence.

| Retained Windows artifact | Identity from supplied report |
| --- | --- |
| Product | `691a8fbb45980658b98d647a85ea8305b2325938`, 0.1.0, x86_64-pc-windows-msvc, release, empty production features, unsigned |
| Executable | 11804160 bytes, SHA-256 `2bef4ebbfcb1a894fd8929da227b86339af8b8f9d239b7e2f112411d194a5708` |
| ZIP | 4497901 bytes, SHA-256 `875f2e06319d67d346d067aa1e37fbce0436b0591ab719228e5310ef7b618f94` |
| External manifest | 541 bytes, SHA-256 `c480ec0df02791d06877dbd4be5f4b680051ee709e22c74aa3d99bcc83e589b1` |
| SHA256SUMS | 230 bytes, SHA-256 `9d6561ea52eb02ce6fac36d9d8c048a845f44a0073870750d2595d1788f28cb5` |

The [macOS isolated runtime](m12-macos-final-runtime.md) and
[final SSH delta](m12-macos-ssh-final-delta.md) already passed on the same product
source. macOS executable SHA-256 remains
`4b5ae7ef4e38382c82732cf785b2ae09af34d8cf614b6fe11d0c1b4efbb80060`, archive
`4d8c1dda4d7fa9ea412928365e8dbc9b550f02d0f3766b3462c1cd7832955007`.
Local checkpoint d8d789e includes both results. Older pending Mac/SSH statements
are historical, not requests to repeat observations. Windows developer-host
physical results remain valid within their original scope.

M12 stays open. Preserve seven of eight failed Windows debug attempts against
250 ms, eight passing release attempts, the finite-burst versus sustained-load
harness discrepancy, all 16 ACs and the final three-platform inventory gate.
Current authorization excludes push, publication, merging, tags, releases and
host account, policy or dependency changes, regardless of older delivery requests.

## Source diagnosis on Mac

Compared the two implicated source files between 691a8fb and local HEAD;
`git diff 691a8fb HEAD -- crates/relayterm-platform/src/permissions.rs crates/relayterm-persistence-sqlite/src/registry.rs`
returns no differences. The current source therefore preserves the relevant
retained-product logic.

1. `initialize_workspace` resolves the project identity, then calls
   `create_private_dir` for Data before Runtime, workspaces, locks or SQLite.
2. On a new directory, `create_private_dir` creates it, secures it and validates
   it. Existing directories are validated without automatically repairing ACLs.
3. `private_descriptor` creates a protected DACL granting FullControl to the
   current process user, SYSTEM and Administrators, inheritable for directories.
4. `validate_windows_acl` opens a handle with READ_CONTROL, uses **GetSecurityInfo**,
   compares owner with the current process SID, checks protected DACL via SDDL,
   then asks for effective rights for Everyone and Builtin Users.
5. Cached locked `windows-permissions` 0.2.4 calls GetEffectiveRightsFromAclW.
   A nonzero return becomes `io::Error::from_raw_os_error`, not a rights mask.
   Relayterm discards that code into SecurityDescriptorUnavailable, registry
   converts it to InitializationError::Location, daemon maps InvalidLocation,
   and CLI emits `invalid_location`.

The external probe used **GetNamedSecurityInfoW**, not the product's handle-based
GetSecurityInfo. Its successful descriptor reads and 1355 for both trustees,
plus the protected product-created Data directory and absent registry, strongly
support this path. They do not prove the failing product call or the cause of
1355. The SYSTEM/Administrators-only control also failed, so a domain-user ACE
alone cannot explain the evidence. Full-environment failure excludes stripped
PATH as the sole cause; PATHEXT fixed only the separate command-discovery issue.
Security modules were observed, with no established causal role.

Microsoft documents group-member enumeration and an error when enumeration
fails. This makes lookup failure plausible even when the queried trustees use
literal well-known SIDs. It does not establish which group or system component
failed on this host. See [GetEffectiveRightsFromAclW](https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-geteffectiverightsfromaclw).

## Next bounded engineering step

Prepare a development-only Windows diagnostic test around the existing
handle-based validation path before claiming a fix. Build it on an existing
Windows development environment; do not install build dependencies on the
independent host or alter its retained executable. Identify source, diagnostic
patch, target/profile and binary SHA-256 separately. Diagnostic execution on the
independent host is still pending; this review did not transfer or run a binary.

The test must preserve the raw OS error before mapping and report only a fixed
stage enum, directory/file kind, trustee alias (Everyone or BuiltinUsers),
numeric OS status and result category. Do not emit paths, account/domain names,
user SIDs, SDDL, environment dumps or terminal content. No public CLI/protocol
change or automatic production logging is needed for this experiment.

Use a fresh owned fixture and the product's exact descriptor creation and
handle-based read sequence. First compare against a read-only check of a retained
partial fixture, if explicitly selected by the operator. Never repair that
fixture. Record owner-match and protected-DACL booleans separately from effective
rights. Run Everyone and BuiltinUsers probes independently in the diagnostic
so the first error does not hide the second. Treat a nonzero API return as
unavailable even if the output mask is zero. Bound each process to 30 seconds.
If the actual validator does not reproduce the mapped error, stop attributing
initialization to this call and capture the other bounded initialization stages.
No VPN, domain, account, elevation or security-software intervention is part of
this procedure.

## Proposed correction contract, not an implemented fix

Prefer investigating a strict structural DACL validator for Relayterm-managed
objects: compare binary SIDs against the current user, SYSTEM and Administrators
without group expansion or account-name lookup. Enumerate and validate ACEs with
supported native APIs, not string splitting of SDDL. This is a conservative
allowlist policy, not a general effective-access calculator.

Retain owner, protected-DACL, file type and reparse rejection. Require the known
usable grants appropriate to files and inheritable directories. Reject null or
absent DACLs, invalid descriptors, malformed bounds/SIDs, unexpected ACE types,
unsupported flags and grants to any other principal. Inherit-only or unknown
entries must not silently pass because they do not grant the current object
access. Do not infer safety from a deny entry or allow a broad grant masked by
a deny. Unreadable metadata remains a failure. Define masks, flags, duplicate
handling and inheritance rules explicitly before implementation.

This could reject custom ACLs previously accepted by the two-trustee heuristic;
review and document that compatibility consequence. Do not silently rewrite an
existing ACL. An Authz/AccessCheck result for the current user's token alone is
not proof that other users lack access. Never accept 1355 as zero rights, skip
validation, add Everyone permissions or require administrator elevation.

## Verification gates

| Environment | Required proof |
| --- | --- |
| Mac, performed in this review | Source/wrapper error-path inspection and unchanged-source comparison; three existing Unix permissions tests; documentation/privacy checks recorded in the logbook |
| Mac, future implementation | Pure policy tests if separated from native decoding, including malformed and unknown entries; these cannot validate Win32 decoding or ConPTY |
| Windows development, pending | Diagnostic reproduces actual stage/status; native files/directories and inheritance; current-user owner; rejection of Everyone, Users, Authenticated Users and unrelated SID grants, null/empty/unprotected DACLs, wrong owner, reparse points and unsupported ACEs; unknown/API errors fail closed; existing denied fixtures stay unchanged |
| Windows regression, pending | Workspace init and reopen, IPC, SQLite sidecars, private backup/restore, no-overwrite behavior; fmt, clippy and relevant platform/persistence/CLI tests plus broader repository checks |
| Independent Windows 11, pending | Non-elevated account, fresh state, separately identified reviewed candidate, installation and previously blocked automated journey; preserve original retained-package failure |

Native diagnostic reproduction must distinguish a real 1355 observation from
synthetic error injection. Injected failures can prove error handling, not the
host's domain condition. Do not change host policy to force reproduction.
Mac success, cross-compilation and historical hosted CI cannot close these gates.
A product correction needs a new artifact mapping and affected native evidence;
reuse unaffected physical and SSH observations with an explicit impact assessment.
