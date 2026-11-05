# Clear Glass in vanilla crafting

The pinned source's four alternative recipes accept `forge:glass/colorless`
instead of only vanilla Glass. With the default native tag, vanilla Glass and
TConstruct Clear Glass work for Glass Bottles, a Beacon, a Daylight Detector,
and an End Crystal. The original shapes, other ingredients, and output counts
are retained: three bottles or one of the other outputs. This reuses the native
colorless-glass tag supplied by the machine-crafting content.

These recipes are contributed to the host's ordinary recipe tables when
`[recipes] glassRecipeFix = true` in `config/tconstruct-common.toml` (the source
default). The authoritative world-load/reload hook rereads that world's config;
false, an invalid setting, or unreadable configuration adds none of these four
recipes. Remote clients rely on the server's recipe table. Existing vanilla
recipes are not removed. Explicit earlier contributions of the same recipe ID
are retained, and world datapacks have the host's normal higher precedence.
Recipe IDs are `tconstruct:common/glass/vanilla/<output>`.

There are no unconditional native recipe files that could bypass the setting.
This helper shares the native data-file hook with other contributors, including
optional Gravel crafting, but uses distinct IDs and does not edit their files.

Tests execute the production contribution helper with the real config parser,
cover all four emitted native recipes, invalid/disabled configs, repeated calls,
separate worlds, and setting changes between fresh reload inputs. Python checks
compare every source recipe and verify tag availability and hook wiring.
Full host checking is separate from these unit tests; actual crafting clicks,
live reload and LAN behavior still need verification. Recipe-book unlocks are
not added by this change.

Run the focused tests from the repository root:

```sh
cargo test --manifest-path tconstruct/tests/glass-crafting/Cargo.toml
python3 -m unittest discover -s tconstruct/tests -p 'test_glass_crafting.py'
```
