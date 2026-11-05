# Successful Foundry menu opening progression

Adds source `tconstruct:foundry/structure`, preserving `open_foundry`,
`tconstruct:block_container_opened` and exact block-entity type
`tconstruct:foundry`. Only the icon item key becomes native `id`.
Source task display, translations, toast enabled/chat disabled, visibility,
requirements and no-rewards behavior are unchanged.

The source parent remains `tconstruct:foundry/alloyer`. That Alloyer
advancement and the Scorched Alloyer block are not registered by this port,
so this addition does not complete the visible Foundry ancestry. The host
can still track its criterion and may issue a toast. No substitute parent,
recipe or pretend Alloyer block is introduced. Foundry Blaze and Manyullyn
Lanterns conditions are not implemented by this change.

The bounded controller-open adapter now maps Foundry Controller to Foundry
type and Smeltery Controller to Smeltery type. One cannot satisfy the other's
criterion. It reuses the successful-open paths from PR #828: valid current
structure and nonempty saved state before local opening; the actual matching
network menu before guest attribution through `as_guest`. The local path
still excludes temporary guest contexts. Player predicates and progress use
the normal native advancement pipeline. Remote clients never award locally.
Other container types and existing menu/structure behavior are unchanged.

Exact source advancement, SHA-256 and attribution are under
`tests/fixtures/foundry_opening/`, from Tinkers' Construct
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`.
The shared production-helper Rust fixture now covers separate Foundry and
Smeltery types, actor predicates, repeat awards and remote suppression.
Python checks cover source fidelity, unchanged parent, existing wiring and
packaging. No tests were run. Brief static review only; no builds, CI
inspection, version checks or merges. Native typing/mixin application,
actual local/guest opens, dedicated server/LAN, toast/tree display and
saved advancement progress remain unverified.
