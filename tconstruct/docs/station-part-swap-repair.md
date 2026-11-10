# Station part-swap repair

Comparison: Tinkers' Construct `f4e77d43`, `MaterialSwappingRecipe.java:112–148`,
`MaterialRepairModule.getDurability`, `ToolStack.java:379–418,440–447`, and
`ToolDamageUtil.java:276–298`. This change is limited to the existing Station
single-part replacement path, not fluid casting or new part types.

The replacement material's repairable stat supplies durability. Supported
Station head, bow limb and four armor plating parts now repair; handles,
grips, bindings, maille and other nonrepairable stats do not. The source integer
part-cost × integer durability product is divided by `3.0f`. Overflow or invalid
stats fail closed rather than emulate Java signed overflow.

The Station rebuilds the replacement first, then applies the existing known
repair hooks in modifier order using the replacement's traits: Emerald,
Cultivated, Preserved and Dense, including their existing upgrade/trait merge.
Truncation to Java's integer range happens after the hooks. Unknown order or
malformed supported levels rejects the plan before consumption. Same-material
repair eligibility uses the positive raw repair amount, even if the final amount
truncates to zero or a modifier reduces it to zero.

Damage is repaired after rebuilding has applied the new maximum. An originally
broken tool uses the new maximum as its effective damage for positive repair.
A zero integer repair preserves its broken flag, including flag-only saved
broken tools. Unrelated custom data is retained. The operation still consumes
one original tool and one replacement part through the existing transaction.

## Boundaries

This reuses the Station's existing bounded repair-hook evaluator, not a general
modifier interpreter. Arbitrary changed/unknown modifier definitions, custom
repair hooks and external durability-changed callbacks remain unsupported.
It does not add modifier-removal hooks, change the existing validation order,
or expand part recognition to shell/ribcage/laces, skull, slime or shield plating.
The source has additional repairable stat types, but these are not claimed as
newly supported Station replacement paths. Fluid-casting replacement retains
its separate arithmetic and compatibility checks.

## Lightweight validation

From repository root, with Rust 1.95.0 and the existing dependency cache:

```sh
D=/workspace/shared/haste-wire-target/debug/deps
rustc --edition=2021 --test tconstruct/tests/rust/station_swap_repair.rs \
  -L dependency=$D \
  --extern serde_json=$D/libserde_json-620fe6b3f56c4283.rlib \
  --extern horizon_items=$D/libhorizon_items-dc50f9345dd775f4.rlib \
  --extern horizon_world=$D/libhorizon_world-16286478434269d0.rlib \
  -o /tmp/station-swap-repair-tests
/tmp/station-swap-repair-tests
python -m unittest discover -s tconstruct/tests -p 'test_*repair*.py'
```

18 Rust tests passed (including existing included stat/effect tests), and 3
Python wiring tests passed. New tests execute the real stat/ordered-hook/final
ItemStack helpers; the Python test checks the actual Station call order and
consumption wiring. They do not execute the complete world/GUI transaction.
New-max shrink, broken-new-max growth, flag-only broken, zero/fractional repair,
nonrepairable parts, changed traits, malformed order, and overflow are covered.
Full generated-host build, actual Station interactions, save/reload and LAN
remain unverified. The temporary linked test binary was removed after testing
to conserve shared disk space.
