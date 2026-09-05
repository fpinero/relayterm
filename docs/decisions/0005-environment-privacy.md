# 0005: Environment inheritance and privacy

Status: Accepted architecture; future runtime verification remains assigned below.
Task: M01.05.
Requirements: Sections 5.2, 9, 11; NFR-5.

## Context

The MVP needs an explicit contract before adding persistence, process, or UI complexity. This decision implements the corresponding bootstrap planning requirement while preserving the existing MVP exclusions.

## Decision

Build child environments explicitly. Persist approved variable names only; resolve values at launch. Authentication stays in provider tools or OS facilities. Relayterm is not a security sandbox.

## Invariants and behavior

| Platform | Baseline names/categories | Treatment |
| --- | --- | --- |
| Linux/macOS | PATH, HOME, SHELL, LANG, LC_ALL, LC_CTYPE, TMPDIR, XDG_CONFIG_HOME, XDG_DATA_HOME, XDG_CACHE_HOME, XDG_RUNTIME_DIR | Inherit existing values only for normal tool behavior; never serialize values |
| Windows | PATH, SystemRoot, WINDIR, COMSPEC, PATHEXT, USERPROFILE, APPDATA, LOCALAPPDATA, TEMP, TMP | Case-insensitive name handling; never serialize values |
| All | TERM, terminal dimensions and session metadata | Supervisor derives supported terminal settings rather than trusting arbitrary parent settings |
| Explicit additions | Names in environment_allowlist, including optional agent/socket/proxy variables | Resolve at spawn; no values in definitions or diagnostics |

Clear inherited environment before assembling the approved map. Unknown configuration fields warn; invalid security-sensitive fields fail closed. Do not read credential stores or introduce provider token fields. Child processes can independently access the user's files and network; filtering launch inheritance does not prevent this.

Configuration values and child bytes may exist transiently in launch/supervision memory. Diagnostic serializers accept only safe fields: stable event name, severity, opaque entity IDs, typed error category, and path aliases. Do not Debug-log full definitions, requests, argument arrays, text bodies, or environments. Escape untrusted content outside terminal panes. Reject recognized credential-shaped input at coordination boundaries and explain that heuristic detection cannot identify every arbitrary secret.

## Alternatives

Inheriting everything silently propagates unrelated credentials. Serializing everything and redacting later is fragile. A complete credential detector is not a credible guarantee.

## Consequences

Some CLIs need explicitly allowed additional variable names. Terminal content is sensitive even when no provider credentials were requested. Ordinary diagnostics sacrifice raw detail for privacy.

## Verification and ownership

M03 configuration and serialization tests inject obvious fake secret/path markers. M07 verifies shell startup under the constructed environment on all OS targets. M08 checks escaped non-terminal views; M11 reviews launch-to-log data flow and negative cases. Fixtures use synthetic names/values only.
