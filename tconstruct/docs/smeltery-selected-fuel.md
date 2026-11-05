# Selected Smeltery fuel temperature

Pinned TConstruct `f4e77d43` selects fuel before asking whether it can perform the current work. `SmelteryBlockEntity.heat` calls `findFuel(false)`, tests `canHeat(possibleTemp)` / `canAlloy()`, and only then calls `findFuel(true)`. `MultitankFuelModule.findFuel` accepts a candidate with positive fuel temperature; it does not look for a hotter alternative to suit the current items.

Foundry likewise calls `canHeat(findFuel(false))` before consuming fuel.

The current adapter's fuel loop incorrectly continued past valid but too-cold fuel, spending a later hotter tank. It now stops without consuming either tank when its selected valid fuel cannot heat or alloy. A zero-temperature recipe is skipped as in the source module. This applies to the shared Smeltery/Foundry fuel selection path. Recipe quantities, duration, metadata-preserving fuel patches and the existing atomic `write_many` boundary remain unchanged.

Source references:
- [SmelteryBlockEntity](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity/controller/SmelteryBlockEntity.java)
- [MultitankFuelModule](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/smeltery/block/entity/module/MultitankFuelModule.java)

## Limits deliberately left open

This fixes the temperature-check ordering only. The port still discovers candidates in coordinate order instead of the source's structure traversal, and does not implement source cached-handler/last-fuel priority or GUI handler promotion. Those differences can change which tank is selected in a multi-tank structure. Do not interpret this patch as complete multi-tank parity. Using a sole suitable fuel tank avoids ambiguity in the tested scenario.

The test runner executes the verbatim production selection block and its tank-kind/capacity helpers with real source fuel recipes, parsers and Tank NBT; Game, ownership, registry, recipe snapshot access and heating eligibility are bounded adapters. Alloy eligibility is stubbed false. The fixture verifies cold-first/hot-second nonconsumption for both machine flags, missing/insufficient/zero-temperature fallback, no re-selection outside phase zero or while already fueled, changed recipe snapshots, and preserved extension fields in staged patches, and refusal to consume tagged/malformed fuels (retaining existing fallback). It does not execute actual reload, world persistence, structure discovery, the final write transaction, GUI or LAN. The preexisting final atomic save is unchanged; rejected-save behavior is not claimed from this fixture.

Run `HORIZON_HOST=/path/to/clean/pinned/horizon tconstruct/tests/run_selected_fuel_rust.sh`. CI runs all six named cases with clean-host revision and dependency-lock checks. Manual gameplay remains unverified and outside v0.5 completion conditions.

The selected-fuel fixture retains the extracted production selection block
unchanged. After extraction it supplies a type-only `Option<()>` anchor for the
unused deferred alloy RNG local, whose production use is beyond this fixture's
boundary. This neither executes nor claims coverage of the later alloy/RNG stage.
