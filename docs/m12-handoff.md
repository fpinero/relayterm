# M12 handoff

## Starting point

M11 implementation was merged through PR #22 at `dc5b2e6067d50345828ac175858029434ce3c081`. The reviewed source candidate is `d6efd7f`; the pre-merge evidence candidate is `dfdbf43`. See [the final review](m11-final-review.md) and [acceptance matrix](acceptance-matrix.md) for exact evidence and limitations. Post-merge [Quality](https://github.com/fpinero/relayterm/actions/runs/34833773372) and [Security](https://github.com/fpinero/relayterm/actions/runs/34833773375) passed on the merge commit. M11 is complete.

The sole maintainer approved delivery with assistant-assisted technical review. External human review is optional unless explicitly requested for a specific change. Do not impersonate an independent reviewer, require an alternate owner account, or add an external approval gate by inference. Preserve all technical acceptance and security controls.

## Authorized next planning scope

Start with M12.01 in [the pending queue](../TODO.md). Read project instructions, the vision, specification, supported platforms, backup and restore guide, and current CLI behavior. Check the branch and preserve local changes. Create a feature branch before implementation.

Plan and verify the seven M12 tasks in order:

1. Define supported release targets and reproducible builds for `rt` or `rt.exe`, with license notices and checksums. Verify execution without an undeclared build toolchain requirement.
2. Document and test installation and PATH, including an existing unrelated `rt` command without overwriting it.
3. Execute the published quick start from clean environments using synthetic agents and no hosted account.
4. Verify upgrade, compatibility, backup, restore, shutdown, and recovery guidance.
5. Reconcile public documentation and obtain maintainer decisions for release governance and publication.
6. Reconcile all applicable acceptance evidence, including new clean-installation and quick-start results on Linux, macOS, and Windows.
7. Prepare local release notes and an artifact inventory for the maintainer's publishing decision.

Reuse candidate-mapped M11 manual evidence after reviewing changes. Repeat only affected behavior and the new installation/release scenarios; do not replace required new evidence with older results. Keep automated checks distinct from manual observations. Do not weaken budgets, silently relabel failures, or claim a completed release before its gate passes.

## Retained follow-ups and limitations

The TODO queue retains UX-FORM-CURSOR, UX-CONFLICT-MESSAGE, UX-INPUT-ACQUIRE-MESSAGE, UX-SESSION-NAMES, and UX-SESSION-ORDER. They are non-blocking proposals, not silently added M12 requirements. If scheduled, define scope and verify affected behavior.

The two supplemental local Windows throughput misses remain documented. The reference macOS environment and all hosted native budgets passed. This does not establish universal performance on every machine.

M11 closure authorizes neither release publication nor tags. Follow explicit maintainer authorization for subsequent pushes, PRs, merges, and publication. No M12 implementation is included in this handoff.
