# Phase 5 log: re-absorbing open PRs onto the new main

Each batch is a branch cut from `main`, one cherry-picked commit per original PR (`-x` keeps the original hash),
gated by: forbidden-API lint, the unit tests the PRs add, `tools/smoke-local/smoke.py` and CI. Original PRs are
closed only after the batch merges, with a pointer to it.

## Findings that changed the plan

- **Class A was optimistic.** Of 42 first-pass A PRs:
  - 10 were independent recipe-book unlock advancements (batch A-1).
  - 10 (world fluids, #716-#757) declare data for `registered_fluid`, a host feature that official Horizon and
    PlutoLib do not have; the PR docs say it needs the owner's fork. They are class B now (PlutoLib world-fluid
    API or upstream support needed first).
  - 6 conflicted with, and 3 more tested against helper sources from, earlier B PRs in their stack
    (#781, #864, #868, #919, #921, #926, and #775, #778, #821). They follow their B predecessors.
  - 4 were independent (batch A-2: #717, #771, #809, #858).
- `tools/classify_prs.py` now demotes `registered_fluid` data and PRs whose text references the personal fork.

## Batches

| Batch | Original PRs | Result |
|---|---|---|
| A-1 | #721, #727, #729, #735, #739, #741, #744, #748, #751, #754 | PR #948 |
| A-2 | #717, #771, #809, #858 | see its PR |

## Lessons for the next batches

- A cherry-pick that aborts on a conflict can leave untracked files that block later picks; clean them before retrying.
- "Applies cleanly" is not enough: also run the tests the PR adds, since they reveal dependencies on helper sources.
- B-class PRs need the missing PlutoLib API first; list that API in the batch PR instead of hooking the host.
