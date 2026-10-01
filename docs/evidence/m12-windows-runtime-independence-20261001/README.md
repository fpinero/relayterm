# Windows runtime independence evidence

Reviewed automatic Windows evidence from 2026-10-01, scoped to the unchanged
f2b3f6f package. See the [verification handover](../../m12-windows-runtime-independence.md).

- result.json: reviewed passing runner result with path-free loaded-module labels
  and separate scope flags, including standard_account=false and global acceptance=false.
- failed-attempt-01.json: retained module-collection failure and owned cleanup request.
- module-review.json: all 43 unique loaded file hashes, allowed origins, signature
  status/type and publisher disposition. Forty Windows files and two packaged
  Microsoft files are validly signed; the pinned rt.exe remains unsigned.
- cleanup-review.json: recorded PID/creation-time pairs, absent return identities
  and empty successful/failed fixture inventories, with no forced termination.
- fixture-identities.json: original/final runner and collector hashes, ZIP identity,
  native parse/test results, diagnosed fixture correction and scope boundaries.
- SHA256SUMS: canonical LF checksums for the five JSON evidence files.

JSON payloads are sanitized and use LF without BOM. Private paths, account names,
raw snapshots, shell transcripts, runtime databases and binaries remain outside
these files. This dependency/environment audit is not a filesystem sandbox or
global M12 sign-off. No archive rehash is claimed for this audit.
