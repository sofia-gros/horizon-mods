# Slimeball alternatives for Leads and Sticky Pistons

Two source recipes let the existing Forge slimeball family substitute for
vanilla Slimeballs:

- Four String and one slimeball in the source Lead pattern produce two Leads.
- One slimeball above one Piston produces one Sticky Piston.

The default tag includes vanilla Slimeballs (Earth), Sky Slime Balls, Ichor
Slime Balls and Ender Slime Balls. Existing vanilla recipes remain untouched.
These recipes provide uses for already acquired slimeballs; they do not add
new mobs, natural drops or slime island generation.

The pinned TConstruct revision `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`
conditions these recipes on `slime_recipe_fix`, which its implementation maps
to `[recipes] glassRecipeFix`, **not** `slimeRecipeFix`. This port deliberately
keeps that source mapping. Missing settings use the source default, true.
False, invalid supported values, malformed TOML and config read failures do
not contribute the recipes or their discovery advancements.

The native per-world data-file hook reads the existing config parser on each
world load and `/reload`. Remote clients do not apply their own settings.
Files are not written to resources; world datapacks retain higher priority,
and an existing same-ID native contribution is preserved. Dedicated servers
use their working directory for config, single player the game root above
`saves`.

## Recipe discovery

The configured contributor now adds both source recipe-book advancements:
`recipes/tools/common/slime/lead` and
`recipes/redstone/common/slime/sticky_piston`. Each uses the source
`forge:slimeballs` inventory predicate OR its own recipe-unlocked event.
The legacy tag syntax becomes native `items: "#forge:slimeballs"`; source
`recipe` becomes the host's `recipes` string. Criteria, OR requirements and
reward IDs are unchanged, with no display or achievement toast.

Advancements are supplied only when this contributor supplies the matching
recipe. An earlier same-ID recipe prevents attaching our advancement to a
replacement; an earlier same-ID advancement is independently preserved.
World datapacks retain final precedence. Rebuilding with the config disabled
removes these contributions; it does not revoke previously earned progress.
No unconditional advancement files are added to the content package.

Current-world tag replacement and cache invalidation additionally require
`feat/slimeball-discovery-tags-379`, the companion extension of the bridge
stacked on PR #790. This recipe branch deliberately does not duplicate that
bridge. Without the companion, the host's existing tag handling applies.
The source fixtures and SHA-256 manifest live under
`tests/fixtures/configured_slime_discovery/`.

## Validation

```
cargo test --locked --manifest-path tconstruct/tests/slime-crafting/Cargo.toml
python3 -m unittest discover -s tconstruct/tests -p test_slime_crafting.py -v
HORIZON_HOST=/path/to/pinned/horizon python3 tconstruct/tests/run_slime_crafting_overlay_rust.py
```

The production contributor/config fixture covers both recipes, source flag
mapping, disabled/invalid/read-error states, repeated calls modeling reload,
separate roots and other contributions. The additional fixture executes the
unchanged pinned host overlay merge function with stubbed datapack discovery:
higher-priority pack recipes survive and disabled contributions disappear on
rebuild. It does not execute a full Game reload. Source patterns, counts, tags
and packaged slimeball declarations are checked separately.

Actual crafting, in-game reload, recipe-book UI, tag replacement and LAN round
trips remain unverified. Existing Forge namespace and tag removal limitations
remain. Host integration is checked separately by full Mixins/client CI.

The Mixins validation workflow explicitly runs both Rust fixtures. The host
overlay runner uses the checked-in fixture lockfile and runs after the pinned
Horizon checkout is fetched; its dependency set is fetched by the first fixture.

## Discovery review status

This addition updates the production-contributor Rust fixture, overlay
precedence/rebuild fixture and source translation regressions for paired
recipe/advancement contributions. Tests were added or updated but not run.
Only brief static review was performed. No builds, CI inspection, version
checks or merges were performed. Gameplay, recipe-book UI, actual reload,
advancement events and LAN remain unverified.
