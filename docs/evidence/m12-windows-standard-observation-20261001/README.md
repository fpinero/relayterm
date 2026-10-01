# Windows standard-account observation evidence

These files preserve the bounded 2026-10-01 operator observation and subsequent
return-time reconciliation for the unchanged f2b3f6f Windows package.
See the [scope and result](../../m12-windows-standard-observation.md).

- observation.json: unchanged sanitized successful per-run JSON payload, containing
  separate automatic checks and operator confirmations.
- failed-integrity-attempt.json: unchanged sanitized unsuccessful standard-account JSON payload,
  attempt, retained without inventing an unavailable root cause.
- attempt-index.json: all four shared attempts, including the two expected
  filtered-administrator refusal controls, classified with the hashes of their
  original shared result bytes.
- return-verification.json: exact package/fixture/archive identities and the
  return-time absence of the recorded fixture process identities.
- SHA256SUMS: hashes of the four evidence data files using portable LF lines.

Public JSON files use canonical LF without a BOM. Their payloads match the shared
originals; byte hashes differ when the originals use CRLF or a BOM. The manifest
hashes public committed bytes, while attempt-index.json preserves original-byte
provenance. Both working-file and committed-blob manifest checks passed.

There is no private observation path, raw state, terminal transcript, account name,
credential, screenshot or executable here. Raw fixtures and all original shared
attempts remain local. An open unrelated shell or desktop session is not a surviving
product fixture. This same-host account observation does not establish an independent
clean installation, another runtime floor or global M12 completion.
