# M12 handoff

## Starting point

M11 implementation was merged through PR #22 at `dc5b2e6067d50345828ac175858029434ce3c081`. The reviewed source candidate is `d6efd7f`; the pre-merge evidence candidate is `dfdbf43`. See [the final review](m11-final-review.md) and [acceptance matrix](acceptance-matrix.md) for exact evidence and limitations. Post-merge [Quality](https://github.com/fpinero/relayterm/actions/runs/34833773372) and [Security](https://github.com/fpinero/relayterm/actions/runs/34833773375) passed on the merge commit. M11 is complete.

The sole maintainer approved delivery with assistant-assisted technical review. External human review is optional unless explicitly requested for a specific change. Do not impersonate an independent reviewer, require an alternate owner account, or add an external approval gate by inference. Preserve all technical acceptance and security controls.

## Implementation starting point

Follow [M12_details.md](M12_details.md) and the atomic pending queue in [TODO](../TODO.md), beginning at M12.00a. The owner has scheduled all five usability proposals at the beginning of M12: editable session names, predictable session order, visible form cursor, actionable concurrent-edit guidance, and inline competing-input guidance. They do not reopen M11 acceptance.

The detailed plan defines 37 atomic outcomes across M12.00 through M12.07, including persistence and protocol compatibility, bounded ordered paging, safe UI behavior, native artifacts, collision-safe installation, quick start, upgrade/rollback, evidence and local release handoff. Read repository instructions and inspect the actual source before implementing. Preserve unrelated local files and work on a feature or fix branch.

Reuse candidate-mapped M11 manual evidence after reviewing changes. Repeat affected behavior and the new installation/release scenarios. Do not replace required new evidence with older results. Keep automated checks distinct from manual observations, retain fixed budgets and failed observations, and do not claim a completed release before its gate passes.

## Retained limitations and delivery boundary

The two supplemental local Windows throughput misses remain documented. The reference macOS environment and all hosted native budgets passed. This does not establish universal performance on every machine.

M11 closure authorizes neither release publication nor tags. Follow explicit maintainer authorization for subsequent pushes, PRs, merges, and publication. No M12 implementation is included in this handoff.
