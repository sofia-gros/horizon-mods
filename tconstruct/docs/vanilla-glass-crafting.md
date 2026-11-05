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
recipes or their discovery advancements. Remote clients rely on the server's recipe table. Existing vanilla
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
live reload and LAN behavior still need verification. Actual recipe-book discovery and persistence remain unverified.

Run the focused tests from the repository root:

```sh
cargo test --manifest-path tconstruct/tests/glass-crafting/Cargo.toml
python3 -m unittest discover -s tconstruct/tests -p 'test_glass_crafting.py'
```

## Config-controlled recipe discovery

Each contributed recipe now includes its pinned source hidden advancement.
Beacon discovery requires a Nether Star, Daylight Detector Quartz, and End
Crystal an Eye of Ender. Bottle discovery uses the current world's
`forge:glass/colorless` item tag. Each inventory criterion remains OR the
matching recipe-unlocked event, rewarding only that alternative recipe.
The event predicate is adapted to the host's string-valued `recipes` key.

The advancement is emitted only when this helper emits its recipe. Earlier
same-ID recipe contributors retain ownership and do not receive our discovery;
earlier advancement contributions are preserved independently. World datapacks
retain final precedence. No unconditional advancement bypasses config, and
saved progress is not forcibly revoked when the option is disabled.

Bottle discovery extends the bounded current-world tag bridge by Colorless
Glass. Indexing and final predicates share the resolved snapshot, fail closed
on missing/invalid/empty membership, and do not fall back to stale native tags.
Tag-only changes are observed on the next inventory event. This does not expand
native crafting ingredient parity or change the recipes themselves.

Exact source advancements and hashes are retained in
`tests/fixtures/configured_glass_discovery/`. The discovery addition received
brief static review only. Contributor, source-mapping and tag regressions were
extended but not run; no tests, compilation, CI inspection, version checks or
merge was performed. Actual events, reload, UI, crafting and LAN are unverified.
