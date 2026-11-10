# Soup fluid from ordinary bowls

The shared container planner now accepts ordinary Beetroot Soup, Mushroom Stew
and Rabbit Stew. Each bowl supplies 250 mB of the corresponding stock TConstruct
fluid and returns one empty Bowl. These foods can be crafted using vanilla
recipes, providing a fluid source without first acquiring that fluid elsewhere.
Existing filling consumes 250 mB to make the same food, so the stock round trip
preserves food, bowls and fluid quantity. Existing Meat Soup behavior is retained.

The standard input slot accepts one soup bowl, matching its stack limit. Each
transfer processes one item, even if a command supplies an oversized stack.
Missing item/fluid registration, insufficient capacity, occupied output,
fill-only mode, damaged items, item metadata and incompatible single-fluid tank
contents are rejected without consuming the input. Suspicious Stew is excluded:
its effect-bearing contents are not inferred from the item name.

The Melter's direct held-container path from #655 uses the same planner. It
stages a tank-only change and returns the Bowl only after the machine save
succeeds; existing opaque-data guards are unchanged. Other existing callers of
the planner retain their own transaction and multiplayer restrictions.

## Source and limits

Pinned Mantle [`17c287ad`](https://github.com/SlimeKnights/Mantle/tree/17c287ad52f7b10154e07fa7570dc7bccd74ca2b/src/generated/resources/data/mantle/mantle/fluid_transfer)
provides `beetroot_soup_empty.json`, `mushroom_stew_empty.json` and
`rabbit_stew_empty.json`. Each requires a populated `forge` fluid tag and returns
250 mB plus a Bowl. The checked-in fixtures retain those definitions; see
[attribution](../third_party/soup-fluid-transfer-NOTICE.md).

This bounded implementation chooses the bundled `tconstruct` source fluid,
matching the stock tag pairs and existing filling path. Dynamic fluid-tag
replacement and other-mod fluid preference are not added. Source hunger-effect
definitions exist for all three fluids; no new Spilling/Slurping effects or tool
acquisition are implemented or asserted to be game-tested here.

The native-dependency fixture exercises all three routes, round trips, slot
limits, one-item processing, rejection without mutation, tagged-fluid separation
and Melter metadata preservation. Python verifies the pinned definitions and
stock tags. Actual world interaction, consuming the resulting fluid through a
tool, save/reload and LAN remain unverified. Run:

```
HORIZON_HOST=/path/to/pinned/Horizon bash tconstruct/tests/run_seared_reinforcement_rust.sh
python3 -m unittest discover -s tconstruct/tests -p 'test_soup_fluid_transfer.py'
```
