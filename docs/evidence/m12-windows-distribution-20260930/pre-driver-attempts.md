# Preserved preparation and invocation failures

All original checkout material remains untouched. Its tracked delta was also saved
as original-preserved-tracked.patch, outside source. Existing untracked evidence is
still in that checkout and its original private artifact directories.

Before the bounded driver was introduced:

- The first dependency/package read used a cp1252 output stream. Printing the NuGet
  UTF-8 BOM failed with UnicodeEncodeError. A UTF-8 read succeeded without changing
  the pinned inputs.
- A read requested .github/workflows/quality.yml, which does not exist. The actual
  file is ci.yml. This was a read-only invocation error.
- The first installer apply_patch request used delete and add for the same path.
  The tool rejected it before modification. The installer was then written normally.
- initial-check.log preserves the first compile failure, a missing TryInto import
  in the vendored edition-2018 module. check-2.log preserves its successful fix and
  the subsequently corrected function-field naming warnings.

The timestamped driver directories preserve subsequent failures individually:
Clippy item ordering (default and all features), installer hashing availability,
an omitted installer-fixture input variable, documentation whitespace/ADR-count
checks, vendor test invocation outside workspace, absent upstream test files,
the backup fixture's old executable-only copy, and the PTY test's old experimental
alternate-screen expectation. No threshold was relaxed for these corrections.

The initial standalone vendor lock picked crypto-common 0.1.7 and generic-array
0.14.7 from the cache. An explicit comparison rejected that test identity. Both
were repinned to the application's existing 0.1.6 and 0.14.9 identities, respectively.
The application lock was not upgraded. The rejected standalone test identity and
its successful six-test result remain separate from the final repinned result.

A source-edit helper attempted to locate a pre-formatting substring in the resource
test and failed with ValueError before writing. The correction then located the
formatted block. The first resource executable had already been built and its
attempt remains preserved independently from the revised abrupt-child selector.

The earliest failed workspace attempt overlapped the end of release precompilation
before any sustained scenario ran. It stopped at backup/restore and carries no
performance acceptance claim. The later workspace sustained measurements ran with
no overlapping compiler or verification job.

Additional inspection-only errors retained in the session: the resource gate was first looked up under resource_gate.rs instead of resource_runtime_gate.rs; an ADR glob used docs/adr rather than docs/decisions; a progress read selected the production output directory instead of its timestamped attempt log. None changed product source or evidence.

The first observer preparation timed out after 180 seconds while Windows PowerShell captured native JSON until pipe EOF. Only its directly owned PowerShell child was killed by the driver. The positively identified synthetic daemon was then stopped with the exact candidate and that fixture workspace/home, returning exit 0. WindowsIdentity.Groups omitted the disabled Administrators SID, whereas whoami CSV included it. The corrected bounded observer returned standard_account=false and completed synthetic preparation in 1.515 seconds. Neither the initial printed true nor preparation alone is standard-account or physical-console evidence.

The first final installed sequence did not launch a test: an over-broad private driver string replacement selected nonexistent installed3-review3 instead of installed-review3. Parent attempt stderr preserves the traceback; the empty child attempt directory is retained. The path was corrected before repeating that sequence. Two progress reads consequently found no log files in the empty child attempt.

The first numeric-only export failed its equality assertion because a prefix regex omitted lifecycle/module suffixes. Its partial derived directory remains preserved. An accidental placeholder path in the following verifier call failed without changing output. The corrected export is independently compared against all original parsed records before inclusion.

A second numeric export comparison exposed Python helper tuples versus their JSON array representation. Canonical JSON comparison reproduces equality for the original resource record. The third export uses canonical JSON equality and the corrected measurement-prefix allowlist; prior partial exports remain preserved.
