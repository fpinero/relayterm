# M04: Versioned local protocol and client synchronization

## 1. Delivery and execution contract

This document is the detailed implementation plan for M04 in `TODO.md`. It is intended to give the implementing engineer or agent explicit behavioral contracts, implementation order, and verification gates.

The document was prepared in Plan mode and saved after leaving that mode. The user explicitly requested that this delivery create only this file. For this documentation-only delivery, do not modify `TODO.md` or `avances.md`; this is an explicit exception to the normal documentation-task logbook workflow. Saving this document does not complete M04.01 or any implementation task.

### 1.1 Implementation workflow

When M04 implementation is requested:

1. Inspect the current branch, working tree, repository instructions, and relevant project documents.
2. Work on `feature/m04-protocol`, based on the branch containing this accepted document.
3. Preserve all existing user changes, including this document if it has not yet been committed.
4. Add a localized link to this document from M04 in `TODO.md` and expand the queue using the atomic tasks below before implementation.
5. Complete contracts first, then transport security, framing, dispatch, synchronization, client behavior, and integration tests.
6. Remove each pending task only after its implementation and relevant checks pass.
7. Append evidence to `avances.md` without modifying earlier entries.
8. Do not infer authorization to publish or merge M04 from the earlier authorization for M03.
9. Require native Linux, macOS, and Windows evidence before closing M04.09.

## 2. Objective, starting point, and fixed decisions

### 2.1 Required outcome

Two independent local clients must communicate with one workspace server through current-user-restricted IPC, obtain coherent state, submit authorized operations, and consume ordered durable events.

The server implementation must be reusable by M05. M04 proves it through a test harness using real local IPC and real SQLite files.

### 2.2 Existing implementation

The implementer must reread the actual code rather than assume names or signatures from this document remain unchanged.

Current foundations:

- `relayterm-protocol` provides `PROTOCOL_VERSION = 1` and a pure compatibility check.
- [ADR 0002](decisions/0002-local-ipc.md) selects framed JSON control messages and a separate binary terminal frame kind.
- [ADR 0001](decisions/0001-daemon-scope.md) assigns one daemon to each workspace.
- Application services provide domain mutations, definition imports, internal instance observations, and durable reads.
- `DurableReadStore` provides a consistent snapshot, event pages, and task-history pages.
- M03 commits state, history, workspace revision, and event sequences atomically.
- Events carry typed change information, not complete entity content.
- Domain snapshots are explicitly private state, not wire DTOs.
- The current architecture test forbids Tokio and concrete adapters in domain, application, and protocol.
- CLI, daemon lifecycle, and TUI entry points remain unavailable placeholders.

Read the [M03 storage contract](M03_storage_contract.md), [M02 storage contract](M02_storage_contract.md), [technical specification](../MVP_TECHNICAL_SPEC.md), [project vision](../PROJECT_VISION.md), and [architecture index](architecture/README.md) before substantial implementation.

### 2.3 Decisions confirmed with the user

**Mutation recovery**

- Never automatically resend a mutation after an uncertain outcome.
- Report uncertainty explicitly.
- Recover authoritative state before the caller decides what to do next.
- Do not introduce durable request receipts, deduplication tables, or a migration for request idempotency in M04.
- A new request ID does not make a repeated business operation safe.

**Client attribution**

- Every authenticated M04 client acts as `LocalUser`.
- A user may claim a task on behalf of a supplied instance ID.
- That operation retains user attribution.
- Clients cannot select `System` or claim to be an authenticated instance.
- Instance credential issuance, authentication, and revocation are outside M04.
- Internal lifecycle services remain inaccessible through public operation dispatch.

### 2.4 Scope boundary

M04 implements protocol, local transport, dispatch, client library, synchronization, and test composition.

It does not implement:

- Detached daemon startup, discovery through CLI, or shutdown commands.
- Workspace initialization through IPC.
- Real or simulated production process launch.
- PTY allocation, terminal reconstruction, or terminal recording.
- Git operations or worktree persistence.
- TUI widgets.
- TCP listeners or remote endpoints.
- Automatic configuration import.
- Event pruning.
- Automatic mutation retries.

Synthetic instances may be introduced only through internal services in test setup.

## 3. Architecture and dependency ownership

### 3.1 Crate responsibilities

| Component | Responsibility |
| --- | --- |
| `relayterm-protocol` | Pure wire DTOs, operation identifiers, protocol errors, framing state machine, bounds, compatibility |
| New `relayterm-ipc` | Async local streams/listeners, endpoint ownership, peer checks, framed I/O, bounded writer |
| New `relayterm-client` | Request correlation, typed calls, synchronization state, reconnect and uncertainty handling |
| `relayterm-daemon` library | Reusable workspace server, DTO/domain conversion, application dispatch, subscription workers |
| `relayterm-application` | Domain orchestration and revision preconditions independent of transport |
| `relayterm-platform` | Existing private paths, native identity, locking, and narrow reusable OS helpers |
| SQLite adapter | Existing durable storage and read contracts |
| Integration test harness | Composes server, services, SQLite, and synthetic lifecycle setup |

The daemon library may gain usable server APIs while its user-facing runtime entry remains unavailable until M05.

### 3.2 Allowed dependency changes

- Protocol keeps no dependency on other Relayterm crates.
- Protocol may depend on Serde, JSON, and pure scalar validation libraries.
- IPC may depend on protocol and platform.
- Client may depend on protocol and IPC.
- Daemon may depend on application, domain, protocol, and IPC.
- Daemon integration tests may depend on SQLite and platform.
- Domain and application must not depend on IPC or client.
- Client must not depend on daemon, SQLite, or domain state.
- Do not connect CLI/TUI to the new client as part of M04.

Update architecture documentation and negative controls with these explicit edges. Do not permit arbitrary new Relayterm dependencies.

### 3.3 Async and platform choices

- Keep the frame codec synchronous and incremental.
- Put Tokio I/O integration in IPC.
- Enable the existing Tokio dependency's required `net` and `io-util` features only in runtime components.
- Use Tokio Unix sockets on Linux and macOS.
- Use Interprocess's Tokio Windows named-pipe API with an explicit creation-time security descriptor.
- Use the reviewed Interprocess 2.4 release family, starting with version 2.4.3; resolve and record the exact compatible version in the lockfile during the platform proof.
- Retain repository-owned `unsafe_code = "forbid"`.
- Do not add an unsafe exception to work around a missing wrapper.
- Verify MSRV, licenses, feature closure, and security APIs before integrating the Windows dependency.

Interprocess documents creation-time descriptors, remote-client control, and Tokio listeners. Its security descriptor supports safe SDDL deserialization. These capabilities must still receive native tests. See [pipe listener options](https://docs.rs/interprocess/latest/x86_64-pc-windows-msvc/interprocess/os/windows/named_pipe/struct.PipeListenerOptions.html) and the [security descriptor API](https://docs.rs/interprocess/latest/x86_64-pc-windows-msvc/interprocess/os/windows/security_descriptor/struct.SecurityDescriptor.html).

## 4. Wire contract

### 4.1 Frame layout

All frames use this seven-byte header:

| Offset | Size | Meaning |
| --- | --- | --- |
| 0 | 4 bytes | Payload length, unsigned big-endian |
| 4 | 2 bytes | Protocol version, unsigned big-endian |
| 6 | 1 byte | Frame kind |

Payload length excludes the header.

Frame kinds:

- `0x01`: UTF-8 JSON control message.
- `0x02`: opaque terminal data.
- Every other kind is invalid in version 1.

Rules:

1. Read the fixed header before allocating a payload buffer.
2. Validate version, kind, and kind-specific size.
3. Use checked arithmetic.
4. Reject zero-length control payloads.
5. Preserve partial header and partial payload state across reads.
6. Decode every complete frame from coalesced input.
7. Bound buffered bytes even when one transport read contains many frames.
8. Reject unsupported kinds rather than skipping them.
9. EOF between frames is a normal disconnect.
10. EOF inside a header or payload is a truncated frame.
11. Do not use compression or newline framing.
12. Serialize through a bounded writer so outbound serialization cannot silently exceed the limit.

The server closes only the offending connection after a framing violation.

### 4.2 Initial bounds

Use binary units.

| Resource | Limit |
| --- | --- |
| JSON payload | 8 MiB |
| Encoded collection content per response page | 6 MiB |
| Handshake payload | 4 KiB |
| Operation identifier | 64 ASCII bytes |
| JSON nesting | 32 levels |
| Error response | 2 KiB |
| Page size | Default 50, maximum 200 |
| Terminal data per frame | 16 KiB |
| Connections per workspace server | 16 |
| Outstanding requests per connection | 16 |
| Outstanding requests across the server | 64 |
| Request/response buffered bytes per connection | 16 MiB |
| Event queue per subscription | 256 events and 1 MiB |
| Subscriptions per connection | 1 |
| Aggregate IPC buffered payload budget | 128 MiB |
| Client snapshot staging | 64 MiB of encoded entity data |
| Handshake timeout | 5 seconds |
| Partial-frame timeout | 10 seconds after its first byte |
| Blocked writer timeout | 10 seconds |
| Default client request deadline | 35 seconds |
| Durable event fallback poll | 250 milliseconds |
| Snapshot restart attempts per refresh | 3 |

Enforce both item and byte limits. Queue accounting includes frames currently being written until their buffers are released.

These limits bound protocol buffers. They do not claim a total process-memory ceiling: M03 currently reconstructs complete domain state, and that inherited storage behavior must be documented separately.

### 4.3 Handshake and connection binding

The first frame must contain:

```json
{
  "type": "request",
  "protocol_version": 1,
  "request_id": "1",
  "operation": "protocol.hello",
  "workspace_id": "00000000-0000-0000-0000-000000000001",
  "params": {}
}
```

Successful handshake returns:

- Protocol version.
- Connection ID.
- Bound workspace ID.
- Supported operation identifiers.
- Frame and page limits.
- Terminal capability set to unavailable.

Rules:

- Authenticate the OS connection before accepting application requests.
- Match the requested workspace against the server's fixed workspace.
- Never select a database from an arbitrary client-supplied path.
- Reject a second handshake on the same connection.
- Reject other requests before the handshake.
- Require header and JSON versions to match.
- Version mismatch closes the connection without starting or replacing a daemon.

### 4.4 Request and response envelopes

Requests contain exactly:

- `type`.
- `protocol_version`.
- `request_id`.
- `workspace_id`.
- `operation`.
- `params`.

Responses contain:

- `type = "response"`.
- `protocol_version`.
- `request_id`.
- `workspace_id`.
- Exactly one of `result` or `error`.

Subscription messages contain:

- `type = "event"`.
- `protocol_version`.
- `workspace_id`.
- `subscription_id`.
- The originating subscribe `request_id`.
- A typed event or synchronization-control payload.

Do not serialize Rust `Result` using its default representation.

Request IDs are decimal strings representing nonzero `u64` values. They must increase strictly within one connection, including requests that receive validation errors. A repeated or decreasing ID is rejected without redispatch. The server stores only the highest accepted ID, not an unbounded cache.

This protects against duplicate dispatch on one connection. It provides no cross-connection idempotency.

### 4.5 JSON validation

- Reject unknown envelope and operation-parameter fields.
- Reject duplicate object keys, including nested objects.
- Reject trailing non-whitespace data.
- Reject malformed UTF-8 and nonconforming scalar types.
- Reject unsupported enum variants.
- Map unknown operations to a stable error without echoing their supplied name.
- Implement bounded duplicate-key detection explicitly; parsing into a generic JSON map alone is insufficient.
- Keep payload-bearing DTOs free of automatically exposing `Debug` implementations.

A malformed JSON/envelope closes the connection. A well-formed request with invalid operation parameters receives a correlated error and leaves the connection usable.

### 4.6 Scalar and entity representations

| Value | Representation |
| --- | --- |
| Entity/connection/subscription ID | Canonical lowercase hyphenated UUID string |
| Revision, event sequence, request ID | Canonical decimal string |
| Timestamp | Object with decimal-string Unix seconds and integer nanoseconds |
| State, priority, reason | Explicit stable `snake_case` enum |
| Optional value | Explicit `null` unless a request default is documented |
| Native path | Encoding tag and Base64 payload |
| Relative scope/change path | Validated UTF-8 string |
| Terminal content | Binary frame only |

Native paths reuse the M03 encoding tags and decoded 8 KiB bound. Decode losslessly; do not use lossy display strings as identity. Reject encodings for a different native platform at an operation that needs a local path.

Create explicit response DTOs for every persisted entity:

- Workspace.
- Agent definition.
- Instance/session and launch-definition snapshot.
- Task and task content.
- Claim and closure attribution.
- Progress entry.
- Handover.
- Workspace event.

Map every field from the validated domain record inventory. Do not derive wire deserialization on private domain entities.

Authorized entity responses may include their required private content. Events, errors, and logs must not copy that content. Never expose database paths, lock paths, registry internals, or raw security descriptors.

Event wire conversion preserves payload version 1 and known event variants. Convert IDs and counters to the wire scalar representation without changing the stored event format.

## 5. Public operations and application mapping

### 5.1 Implemented operations

| Operation | Parameters and behavior |
| --- | --- |
| `protocol.hello` | Bind the authenticated connection to its workspace |
| `protocol.ping` | Empty parameters, bounded liveness response |
| `workspace.get_snapshot` | Start or continue a coherent paged snapshot |
| `agent.list_definitions` | Revision-checked definition page |
| `agent.register_definition` | Existing definition creation fields |
| `agent.update_definition` | Definition ID, complete editable fields, expected revision |
| `task.list` | Revision-checked task page |
| `task.get` | Task ID and optional expected revision |
| `task.create` | Complete task content |
| `task.update` | Task ID, complete editable content, expected revision |
| `task.transition` | Task ID, target state, expected revision |
| `task.claim` | Task ID and target instance ID |
| `task.release` | Task ID and expected revision |
| `progress.append` | Task ID, summary, verification |
| `handover.create` | Task ID, complete handover content, expected revision |
| `handover.get` | Handover ID |
| `task.get_history` | Task ID, exclusive sequence cursor, limit, expected revision |
| `task.get_claim_history` | Task ID, revision-checked claim page |
| `session.list` | Revision-checked persisted instance/session metadata |
| `event.list` | Exclusive event sequence cursor and limit |
| `event.subscribe` | Exclusive event sequence cursor |
| `event.unsubscribe` | Active subscription ID |

### 5.2 Mutation rules

- Route domain mutations through existing application services.
- Supply `Actor::LocalUser` in server code.
- Never accept caller-selected actor or author timestamps.
- Generate entity IDs and times through existing injected dependencies.
- `task.update` replaces editable task content only.
- Use dedicated transition, claim, and handover commands.
- Do not bypass claims by updating task status directly.
- Preserve existing final-state and ownership guards.
- Preserve informational dependency behavior.
- Keep progress and handovers append-only.
- Do not turn `session.list` into a lifecycle observation.

Add a revision-checked application execution entry point for operations requiring an expected revision. The comparison must occur against the same transaction snapshot used by the mutation, not as a preceding independent read.

Definition update should reuse revision-aware import behavior for one existing definition:

1. Require the definition to exist.
2. Preserve its stable ID and workspace.
3. Validate all editable fields.
4. Apply the update atomically.
5. Preserve an identical update as a no-op.
6. Never rewrite historical launch snapshots.

### 5.3 Mutation results

Successful results contain:

- IDs needed to identify the created or changed entities.
- Committed workspace revision.
- `changed`.
- First and last generated event sequences, or `null` for a no-op.

Do not return a complete application snapshot as every mutation response.

A post-commit notifier failure remains success. Subscription recovery must not depend exclusively on notifier delivery.

### 5.4 Pagination

Ordinary entity lists:

- Default limit 50, maximum 200.
- Sort by canonical ID bytes.
- Use an exclusive `after_id`, never an offset.
- First page captures revision.
- Following pages require that revision.
- Return conflict when the revision changed.
- Stop early at the encoded byte budget and return a continuation cursor.
- Never silently omit records or produce a non-advancing cursor.

Task history uses the existing durable append-sequence query. Do not substitute the older offset-based application history API.

### 5.5 Reserved operations

The following are known but unavailable in M04:

| Operation | Reserved request content |
| --- | --- |
| `session.create` | Launch kind, optional definition/task IDs, optional native working directory, rows, columns |
| `session.attach` | Session ID |
| `session.input` | Session ID and stream ID; actual bytes belong to binary frames |
| `session.resize` | Session ID, rows, columns |
| `session.terminate` | Session ID |
| `worktree.create` | Task ID, base ref, branch name, relative destination |
| `worktree.list` | Bounded page request |

`session.create` distinguishes a configured definition from a default shell. Do not accept raw shell command strings.

Reserved operations validate framing and request shape, then return `operation_unavailable` before any mutation or process action. They must not:

- Create fake `running` instances.
- Return fabricated process IDs.
- Return empty worktree success responses.
- Change task associations.
- Create directories or branches.

Their identifiers are reserved; future successful result schemas remain feature-gated until M07/M10.

## 6. Consistent snapshots and synchronization

### 6.1 Snapshot paging contract

A complete domain snapshot may exceed one frame. Implement a stateless, revision-checked multi-page protocol.

First call:

- No continuation cursor.
- Captures a `WatermarkedSnapshot`.
- Returns workspace metadata, revision `R`, watermark `W`, retention floor `F`, and the first entity page.

Page order:

1. Definitions.
2. Tasks.
3. Instances.
4. Claims.
5. Progress.
6. Handovers.

Within a collection, sort by ID bytes.

Continuation includes:

- Snapshot revision.
- Snapshot watermark.
- Collection.
- Exclusive last entity ID.
- Requested limit.

For every continuation:

1. Read a new consistent snapshot.
2. Require its revision and watermark to match the original.
3. Validate cursor collection and bounds.
4. Return the next bounded page.
5. Return `next_cursor = null` only after all collections finish.

If state changes between pages, return conflict. Do not hold a SQLite transaction open while waiting for client requests.

The client builds state in a staging buffer and publishes it atomically only after the final page. On conflict it discards staging and restarts, at most three times per refresh. Repeated activity produces an explicit retryable synchronization state.

If the client staging limit is reached, report `resource_limit`; never publish a truncated snapshot.

### 6.2 Subscribe after the snapshot

After installing snapshot `(R, W)`:

1. Request `event.subscribe(after_sequence = W)`.
2. Server creates a coalescing wakeup receiver before its initial durable read.
3. Server validates the requested cursor using durable event storage.
4. Server queues the successful subscription response before subscription events.
5. Server replays durable events after `W`.
6. Server continues reading pages after its last queued sequence.
7. Server rechecks storage on notifications and every 250 milliseconds.

Notifications are wakeups, not authoritative event payloads.

A mutation between snapshot completion and subscription registration is recovered from the durable log. A lost notifier wakeup is recovered by polling.

### 6.3 Replay and live delivery

- Use one event pump per subscription.
- Preserve ascending sequence order.
- Require the next event to equal the previous sequence plus one.
- Do not concatenate an independent replay producer and live producer.
- Fetch at most 200 events per durable page.
- Respect both queue limits.
- Coalesce notifications rather than buffering one wakeup per commit.
- Use checked sequence arithmetic at `u64` boundaries.
- Never infer order from timestamps.
- Never silently skip an unknown event version.

A `caught_up` control message identifies the durable watermark reached during that read cycle. It does not promise that no newer commit exists.

### 6.4 Cursor outcomes

Let `W` be the current watermark and `F` the first retained sequence.

- `after_sequence > W`: `invalid_cursor`.
- `after_sequence + 1 < F`: `resnapshot_required`, using overflow-safe comparison.
- `after_sequence = W`: valid subscription waiting for later events.
- Empty workspace: `W = 0`, normal `F = 1`, cursor `0` is valid.
- Missing sequence inside supposedly retained coverage: integrity failure, never silent recovery.

M03 does not prune events. Test expired cursors through a controlled storage fixture without adding production pruning.

### 6.5 Content refresh

An event is an invalidation signal for content, not a replacement entity.

The client must expose:

- Last complete snapshot and its watermark.
- Last contiguously received event sequence.
- Whether the visible snapshot is stale.
- Connection and synchronization status.
- Explicit refresh.

For automatic refresh:

1. Coalesce invalidations for 50 milliseconds.
2. Run only one snapshot refresh at a time.
3. Keep the existing snapshot visible but marked stale.
4. Install the new snapshot atomically.
5. Preserve later events already received.
6. Do not label a snapshot current if its watermark trails known events.

A client may receive all events while its visible content is still stale. Keep those facts separate.

### 6.6 Slow clients

When an event queue exceeds its byte or item bound:

- End that subscription.
- Drop its pending event frames.
- Send `resnapshot_required` with reason `slow_subscriber` through the control queue if possible.
- Close the connection if its control writer is also stalled.
- Continue serving other clients.

After unsubscribe acknowledgement, no additional frames for that subscription may be written. Remove pending frames before queuing the acknowledgement.

## 7. Local transport security and endpoint ownership

### 7.1 Shared endpoint contract

An endpoint is derived from:

- Native user identity.
- Opaque workspace ID.
- Selected private runtime root.

It is not derived from display names, project names, or unsanitized client text.

- Require absolute local endpoint inputs.
- Reject remote host syntax.
- Do not fall back to TCP or the current directory.
- Keep endpoint identities and paths out of routine diagnostics.
- Require an owned workspace-lock guard before binding or cleaning an endpoint.
- Hold that guard for the server endpoint lifetime.

M04 provides this ownership primitive. M05 will compose it with daemon startup and registry lookup.

### 7.2 Unix sockets

Use a private runtime parent with mode `0700` and a socket with mode `0600`.

Binding sequence:

1. Validate the private directory and ownership.
2. Acquire the workspace endpoint lock.
3. Inspect an existing endpoint with link-aware metadata.
4. Reject symlinks and non-socket objects.
5. Reject foreign ownership.
6. If a live endpoint responds, report endpoint in use.
7. Only the lock holder may remove a verified stale socket.
8. Bind, set socket mode, verify it, and begin accepting.

Do not change process-wide `umask` in an async server.

Reject overlong socket paths with a safe error directing the caller to an explicit shorter runtime root. Do not silently invent a second endpoint location.

Validate both server-side client credentials and client-side server credentials against the local UID. Tokio exposes peer credentials on Linux and macOS. Failure to obtain required credentials fails closed. See [Tokio peer credentials](https://docs.rs/tokio/latest/tokio/net/unix/struct.UCred.html).

Cleanup must verify that the endpoint is still the socket owned by that listener before unlinking it.

### 7.3 Windows named pipes

Use local byte-mode duplex named pipes.

Required creation policy:

- Explicit protected DACL.
- Current user SID as owner.
- Access granted only to that current SID.
- No Everyone, Anonymous, Users, or broad administrator ACE copied from file-storage permissions.
- `accept_remote = false`.
- Non-inheritable handles.
- Bounded instance count allowing active clients plus the accepting instance.
- First-instance collision protection, verified in the selected library implementation.
- No interval where the pipe exists with a default permissive descriptor.

The filesystem ACL helper from M03 is not the named-pipe authorization policy.

Windows checks the pipe DACL during client connection. Its default descriptor can grant read access to broad principals, and some generic write rights also permit creating pipe instances. Configure and test exact rights deliberately. See [Microsoft named-pipe access control](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights).

Verify the connected pipe object's owner and DACL through its handle where exposed by safe APIs. Do not treat a PID or a caller-supplied SID as authentication.

M04 relies on OS-enforced pipe access for the user identity. It must not claim an additional peer-token check that it does not implement.

### 7.4 Security verification requirements

- Same-user connection succeeds.
- Unix UID mismatch is rejected before dispatch through a controlled peer-policy fixture.
- Unix endpoint mode and private parent are inspected natively.
- Windows DACL is inspected from the actual created pipe.
- A controlled deny-current-user pipe rejects a real connection.
- Synthetic foreign principals receive no data-access rights in ACL checks.
- Remote acceptance is disabled in the actual creation configuration.
- A second server cannot attach as another listener to an existing live endpoint.
- Repeated client connections preserve the same ACL policy.
- Tests do not require creating real user accounts or modifying machine-wide security settings.

If safe APIs cannot satisfy a required behavior, leave the platform task open. Do not replace a failed proof with a weaker success claim.

## 8. Errors, uncertainty, cancellation, and resource ownership

### 8.1 Error representation

Use:

```text
code
field: optional enumerated field
effect: not_applied | unknown
recovery: none | refresh | reconnect | resnapshot | inspect_state | use_matching_version
```

Messages, if supplied, come from fixed server text. Do not echo arbitrary operations, parameters, database diagnostics, paths, SIDs, or parser excerpts.

Required codes include:

- `invalid_request`.
- `invalid_params`.
- `unknown_operation`.
- `unsupported_version`.
- `unauthorized`.
- `workspace_mismatch`.
- `invalid_state`.
- `invalid_reference`.
- `conflict`.
- `invalid_time`.
- `storage_busy`.
- `read_only`.
- `storage_error`.
- `integrity_error`.
- `invalid_cursor`.
- `resnapshot_required`.
- `operation_unavailable`.
- `resource_limit`.
- `result_unknown`.

Map known application errors explicitly. Never map `Uncertain` to `not_applied`.

### 8.2 Mutation uncertainty

Client outcomes distinguish:

1. **Not sent:** failure before the request enters the writer.
2. **Confirmed success:** valid correlated success received.
3. **Confirmed rejection:** valid correlated error with `not_applied`.
4. **Unknown:** sending began and no definitive response arrived, or the backend reports uncertainty.

Once writing begins, conservatively classify disconnect, timeout, or partial-frame failure as unknown.

- Do not retry the mutation.
- Do not infer failure from absence of an event.
- Do not infer identity from matching narrative text.
- Reconnect and refresh state.
- Surface uncertainty to the caller for an explicit decision.

### 8.3 Cancellation

M04 cancellation is local waiting cancellation, not a remote rollback command.

- Before writer ownership: remove the queued request and report not sent.
- After writer ownership: classify a mutation as unknown.
- Once the server dispatches a mutation, let it reach a transaction outcome even if the client disconnects.
- Do not abort the commit future merely because a response receiver disappears.
- Read operations may be abandoned safely.
- Reserve bounded correlation entries for late responses until completion or disconnect.
- Do not allow cancellation tombstones to grow without limit.

Future M05 shutdown must retain ownership of dispatched mutation tasks until they finish or reach an explicitly uncertain outcome.

### 8.4 Concurrency and writers

- One reader and one writer per connection.
- One ordered operation worker per connection.
- Separate connections may execute concurrently.
- A server-wide semaphore bounds operations.
- Subscription delivery must not block request parsing or other connections.
- Acquire byte-budget permits before allocating/queuing large payloads.
- Never hold a domain, registry, or store lock across a stalled network write.

For future terminal support, maintain separate control/event and terminal queues. When both are ready, write at most one terminal frame before checking control again. M04 tests this scheduler with synthetic frames only.

## 9. Binary terminal reservation

Terminal frame payload layout:

| Field | Size |
| --- | --- |
| Session ID | 16 bytes |
| Stream ID | 8-byte big-endian unsigned integer |
| Byte offset | 8-byte big-endian unsigned integer |
| Data | 1 through 16 KiB |

The 32-byte metadata prefix is included in the terminal payload length.

- Terminal offsets are distinct from workspace event sequences.
- No UTF-8 decoding, Base64 conversion, credential-pattern scanning, or transcript logging.
- No actual stream can be established in M04.
- A well-formed unsolicited terminal frame is a protocol violation because no attachment exists.
- Reserved JSON terminal operations return unavailable.
- Synthetic writer tests verify FIFO ordering, partial writes, and control fairness without constructing real sessions.

Reattachment, gap markers, ownership of input, resize authority, and terminal-history policy remain M07 decisions.

## 10. Atomic task breakdown

### M04.01: Finalize contracts

1. **M04.01a:** Reconcile ADR 0002, specification section 6.2, and M03 handoff.
2. **M04.01b:** Publish the operation table and separate user commands from internal observations.
3. **M04.01c:** Specify frames, scalar encodings, DTO fields, and limits.
4. **M04.01d:** Specify request IDs, error effects, cancellation, and no automatic mutation retry.
5. **M04.01e:** Specify snapshot paging, event replay, cursor expiry, and stale content.
6. **M04.01f:** Specify endpoint ownership and native security proofs.
7. **M04.01g:** Record crate boundaries and downstream M05/M07 handoffs.

Verification: cross-review tables and examples for contradictions. Every implemented operation must have request, result, authorization, mutation effect, and tests.

### M04.02: Implement the pure protocol

1. **M04.02a:** Add scalar newtypes and checked encodings.
2. **M04.02b:** Add explicit request/response/event DTOs.
3. **M04.02c:** Add bounded JSON decoding and duplicate-key rejection.
4. **M04.02d:** Implement incremental frame encode/decode.
5. **M04.02e:** Add handshake and version rules.
6. **M04.02f:** Add safe protocol error categories.
7. **M04.02g:** Add byte-level golden fixtures and round trips.

Verification: partial/coalesced frames, every header boundary, every kind, version mismatch, exact limits, invalid JSON, duplicate fields, non-UTF-8, unknown variants, and EOF.

### M04.03: Implement Unix IPC

1. **M04.03a:** Add IPC interfaces and Tokio adapters.
2. **M04.03b:** Implement endpoint resolution and lock ownership.
3. **M04.03c:** Implement private bind and socket validation.
4. **M04.03d:** Implement symmetric UID checks.
5. **M04.03e:** Implement safe stale cleanup and listener shutdown.
6. **M04.03f:** Add native Linux/macOS connection and rejection tests.

Verification: real streams, permissions, collision, stale socket, malicious replacement fixture, long paths, disconnect, and a valid client after an invalid peer.

### M04.04: Implement Windows IPC

1. **M04.04a:** Prove safe descriptor and pipe APIs against the locked dependency.
2. **M04.04b:** Implement local-only endpoint naming and byte-mode streams.
3. **M04.04c:** Apply explicit owner and protected DACL at creation.
4. **M04.04d:** Validate actual pipe security and collision handling.
5. **M04.04e:** Handle repeated accepts, busy connection attempts, partial I/O, and disconnect.
6. **M04.04f:** Add native allow/deny and actual descriptor tests.

Verification: Windows execution is mandatory. Cross-compilation alone cannot close this task.

### M04.05: Implement service dispatch

1. **M04.05a:** Add explicit DTO/domain conversions and field coverage tests.
2. **M04.05b:** Add transaction-bound revision preconditions.
3. **M04.05c:** Implement definition registration/update and task operations.
4. **M04.05d:** Implement progress, handover, entity reads, and history reads.
5. **M04.05e:** Add compact mutation receipts.
6. **M04.05f:** Enforce connection workspace and user attribution.
7. **M04.05g:** Verify error mapping and post-commit notification failure.

Verification: compare each IPC operation with the corresponding direct application result and persisted state.

### M04.06: Reserve future operations

1. **M04.06a:** Define session and worktree request DTOs.
2. **M04.06b:** Return unavailable before side effects.
3. **M04.06c:** Implement binary terminal envelope encoding.
4. **M04.06d:** Add synthetic queue fairness tests.
5. **M04.06e:** Verify no public lifecycle injection or fake process success.

Verification: unsupported requests leave SQLite revision, events, tasks, instances, and filesystem unchanged.

### M04.07: Implement synchronization

1. **M04.07a:** Implement bounded revision-checked snapshot pages.
2. **M04.07b:** Add ordered durable event pump.
3. **M04.07c:** Register wakeups before reads and add fallback polling.
4. **M04.07d:** Implement cursor and unknown-payload recovery.
5. **M04.07e:** Implement byte/item queue bounds and slow-subscriber isolation.
6. **M04.07f:** Implement unsubscribe ordering and resource cleanup.
7. **M04.07g:** Add deterministic snapshot/replay/live-race tests.

Verification: no missed committed event, no mixed-revision snapshot, and no blocked fast client beside a slow client.

### M04.08: Implement the client library

1. **M04.08a:** Add handshake and typed operation methods.
2. **M04.08b:** Add monotonic IDs and bounded response correlation.
3. **M04.08c:** Add local cancellation and late-response handling.
4. **M04.08d:** Add uncertainty classification without automatic mutation replay.
5. **M04.08e:** Add staged snapshot installation and stale-state tracking.
6. **M04.08f:** Add bounded read reconnect and event resubscription.
7. **M04.08g:** Add explicit recovery and resource-limit outcomes.

Read reconnect uses at most three attempts with delays of 100, 250, and 500 milliseconds. Never reconnect automatically after authorization, workspace, or version failure. Inject time in tests.

### M04.09: Verify the protocol gate

1. **M04.09a:** Build a real server process harness with temporary private storage.
2. **M04.09b:** Run two independent client processes through the full coordination journey.
3. **M04.09c:** Inject response loss after commit and verify no repeated mutation.
4. **M04.09d:** Test malformed peers, queue exhaustion, cursor expiry, and service survival.
5. **M04.09e:** Verify architecture and privacy negative controls.
6. **M04.09f:** Run native platform and security CI.
7. **M04.09g:** Update platform evidence, pending acceptance coverage, and logbook.

The process harness must reuse production server/client libraries. It must not implement a second simplified protocol.

## 11. Required acceptance scenarios

### 11.1 Two-client continuity

1. Initialize a private SQLite workspace through M03 setup.
2. Register two synthetic instances internally.
3. Start the workspace IPC server in a separate process.
4. Connect clients A and B.
5. Obtain matching snapshots.
6. Subscribe after their watermarks.
7. Create and ready a task through A.
8. Claim it for the first instance.
9. Reject a competing claim.
10. Append progress.
11. Create a handover.
12. Verify claim closure and `handover_ready` atomically.
13. Read history through B.
14. Claim for the second instance.
15. Complete the task.
16. Verify both clients observe consecutive events and converge to the same state.
17. Verify attribution remains `LocalUser`.
18. Reopen storage independently and verify durable history.

### 11.2 Uncertain mutation

1. Send progress append.
2. Pause the server after commit and before response write.
3. Disconnect the client.
4. Confirm the client reports unknown.
5. Reconnect and refresh.
6. Verify exactly one progress entry and one corresponding mutation.
7. Verify the client sent no automatic replacement request.

Also test disconnect before write, partial write, explicit rejection, and notifier failure.

### 11.3 Snapshot and subscription races

Use barriers, channels, and injected clocks, not timing guesses:

- Commit after snapshot capture but before subscribe.
- Commit between replay pages.
- Commit while the event pump switches from backlog to waiting.
- Suppress notifier delivery.
- Mutate between snapshot pages.
- Change revision during history paging.
- Force retention expiry in a controlled store.
- Inject an impossible internal event gap.

Each test asserts exact state, watermark, and recovery outcome.

### 11.4 Resource and malformed-peer tests

- Split every header position and representative payload positions.
- Feed many frames in one read.
- Advertise an oversized payload without sending it.
- Stall inside a frame.
- Send invalid JSON, duplicate keys, deep nesting, unknown fields, or wrong scalar types.
- Exceed request and byte budgets.
- Stop reading subscription output.
- Cancel many calls and deliver late responses.
- Repeatedly connect and disconnect.
- Verify permits, tasks, queues, and handles are released.
- Verify an unaffected client can still ping, mutate, and synchronize.

### 11.5 Privacy

Inject synthetic markers into legitimate private content and malformed inputs.

Assert:

- Authorized entity reads preserve required content.
- Events contain only approved event fields.
- Errors contain no injected value or backend context.
- Logs contain no complete frame, request, response, SID, endpoint path, or narrative.
- Debug formatting cannot expose payload-bearing DTO contents.
- Invalid terminal bytes are never logged as text.
- Runtime files remain outside the repository's tracked content.

## 12. Verification, compatibility, and closure

### 12.1 Iteration checks

```text
cargo test -p relayterm-protocol --locked
cargo test -p relayterm-ipc --locked
cargo test -p relayterm-client --locked
cargo test -p relayterm-daemon --locked
cargo test -p relayterm-application --locked
cargo test -p relayterm-domain -p relayterm-application -p relayterm-protocol --locked
```

### 12.2 Full checks

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
cargo deny check advisories licenses bans sources
python3 scripts/check_repository.py
python3 scripts/check_audit_controls.py
python3 scripts/check_secrets.py
gitleaks git --redact --no-banner .
git diff --check
```

Ensure audit tools are available through the execution environment. Do not interpret a missing executable or permission failure as a successful negative control.

### 12.3 Compatibility

- Keep IPC protocol version 1 because no stable wire format has yet been published.
- Treat the M04 contract as the first complete version-1 format.
- Keep event payload version independent.
- Do not alter existing stored event serialization to match wire scalar choices.
- Introduce no SQLite migration for idempotency.
- Preserve configuration import semantics.
- Preserve existing domain state machines.
- Update specification section 6.2 for additive operations and explicit actor restrictions.
- Document expected revision requirements and snapshot page behavior for future clients.
- Keep native-path transport lossless and platform-specific.

### 12.4 Native gate

Require successful execution on:

- Linux stable.
- Linux pinned Rust 1.98.1.
- macOS stable.
- Windows stable.

Record the tested commit, workflow run IDs, native targets, commands, and concrete limitations.

Before closing:

1. Reconcile every M04 task with implementation and tests.
2. Verify every specification operation is either implemented or explicitly unavailable.
3. Verify no missing platform behavior was replaced by a permissive fallback.
4. Remove completed M04 tasks from `TODO.md`.
5. Append exact evidence to `avances.md`.
6. Update platform documentation and the remaining acceptance index.
7. Keep M04 open if any required native or recovery test remains unverified.

### 12.5 Handoff

M05 receives:

- A reusable workspace server.
- An endpoint ownership guard.
- Authenticated local transport.
- A shared client library.
- Explicit mutation uncertainty semantics.
- Coherent paged snapshots and durable event replay.
- Bounded subscription behavior.
- Internal-only lifecycle entry points.

M07 receives reserved terminal operations and a tested binary framing/scheduling foundation. It must still implement and verify real process supervision, terminal stream semantics, and reattachment.

M04 completion demonstrates local protocol correctness and client synchronization. It does not establish detached daemon lifetime or interactive terminal behavior.
