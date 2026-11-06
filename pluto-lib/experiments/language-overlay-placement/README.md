# Separate language repair placement probe

Excluded source preparation, not applied or executed. Official targets are
Horizon f4dcf8a2 and Mixins 5a64af93. The previous direct-build candidate at
00131/9756 remains separate: lexical byte restoration there did not establish
an accepted Mixins hook.

The smallest current declaration proposed here has one module and two hooks:

- `Overlay<'_>::write` cancellable HEAD binds the real private receiver, relative
  path and borrowed bytes. Ineligible/no-op inputs return Continue into the exact
  native body. Eligible inputs use the exact 369-byte native method body with
  only `self`→`overlay` and the final write byte argument substituted. Its
  `Result` is passed unchanged to Break, including directory/detach/write errors.
  No recursive `write` call, extra callback, output-root read or broad overwrite.
- Native `stamp` uses MODIFY_VARIABLE on the single original `parts` STORE. Its
  helper adds one policy input to the original initialized Vec and returns it.
  `require=1`, ordinal 0 and actual private local identity are hypotheses until
  the latest planner resolves them. No guessed witness or automatic pin update.

The frozen policy string and exact helper hash are a reviewed pair. A future
semantic change to eligibility, key retention, bounds or serialization requires
an explicitly revised policy identity and cache-transition review; refreshing a
source hash alone must not leave old-policy roots reusable. Nonsemantic comment
changes do not imply different resource bytes. This candidate does not automate
semantic versioning or change the native hash's collision guarantee.

The language helper is copied from the **one** frozen source owner in sibling
`language-overlay-repair/merge.rs` only when a future controller prepares an
owned probe directory. This directory is not a discoverable production bundle:
its `horizon-mixin.toml` refers to `merge.rs`, which is intentionally not duplicated
here. No ordinary manifest or successful proof is changed.

The prototype reads `overlay.base`, the already prepared pack root, not current
overlay output. This relies on unmodified native-admitted Content and the native
namespace clash check, as pinned in the repair source contract. `Overlay::write`
has other internal callers, so the exact four-component language path check is
mandatory. Current native source has no other language writer; this is a
version-pinned contract, not a general promise about future callers or arbitrary
hand-built Content.

Unlike an interior build hook, HEAD preserves the original function's fallible
body. STORE has a narrower source contract than general interior hooks. This is
a reason to attempt a bounded plan, **not** evidence it will be accepted or type
check. A rejected target/helper/variable must be reported without fallback to a
large overwrite or silently omitting one hook. Never install the writer alone.

`prepare.py` checks the exact native small-copy inverse and returns four payloads
in memory; it does not run tools or apply them. `inspect_plan` rejects any extra
target/module and records actual node, fingerprints, binding and contract. It is
only a tiny probe inspector, not the mandatory all-installed exclusivity guard.
Any eventual activation must prove exclusive writer/stamp composition, and that
RETURN/interior hooks cannot change the copied small writer semantics.

## Separate helper14 test crate

`Cargo.toml.in` and `fixture.rs` describe a standalone serde/std fixture importing
the exact same merge.rs, with all fourteen prepared tests and no filter. It will
seed the selected official lock and admit only its fixture package plus exact
official dependency identities. serde/serde_json versions/features come from
the official client-lib manifest; no mock JSON parser is substituted.

This fixture does **not** compile the host writer, apply hooks, run actual build
or stamp/cache roots, or exercise Game/UI. Actual writer/build/cache cases are a
later, separately reviewed gate. Passing helper14 cannot establish their repair.
The latest Pack8 success remains the unchanged baseline of native whole-file
replacement, not evidence of this candidate's key retention.

No new workflow or native execution exists at this checkpoint. A runner will be
separately reviewed and serially released after the priority mining probe.

Twelve Python source/synthetic-plan inspector tests passed. Initial execution
from a shallow `/tmp` draft failed during import before tests because repository
parent depth was assumed; that source-preparation path was corrected. No native
planner was run, and the synthetic plan rows are not captured native witnesses.
