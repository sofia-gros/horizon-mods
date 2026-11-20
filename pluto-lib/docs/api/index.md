# PlutoLib API

PlutoLib is a library mod for Horizon: events, player access, data, config and commands in one place, so a mod does not have to hook the game itself. Everything below is plain Rust; every example on these pages is compiled and run by the test suite.

## Start a mod on PlutoLib

Declare the dependency in the mod's `horizon-mixin.toml`, and give the mod an `[api]` package:

```toml
[mod]
id = "mymod"
version = "0.1.0"

[api]
path = "api"

[dependencies]
pluto_lib = "^0.1"
```

The package's `lib.rs` registers what the mod wants and returns the hooks entry point. A mod that registers everything through PlutoLib returns `no_hooks()`; the mod never names a Horizon type:

```rust,ignore
use pluto_lib::{ModHooks, ServerPlayConnectionEvents};

pub fn hooks() -> ModHooks {
    ServerPlayConnectionEvents::JOIN.register(|join| {
        join.player.send_message("Welcome!");
    });
    pluto_lib::no_hooks()
}
```

(This one block is not compiled by the page tests, because `ModHooks` and `hooks()` belong to the game-facing half of PlutoLib. Everything else on these pages is.)

## How these pages are written

Each page starts with a **Basic Example** and then walks through the feature one section at a time: what it is for, the code, and what to watch out for. The **Reference** at the end of a page lists every public item with its types, its arguments and an example for each form. All code on these pages is compiled and run by the test suite, so it does what it says.

## Pages

| Page | What it covers |
|---|---|
| [base](base.md) | `Event`, `Identifier`, `ActionResult`, the support matrix |
| [lifecycle](lifecycle.md) | server start, stopping and successful stopped events, ticks, chunk load and unload |
| [networking](networking.md) | player connections and bounded directional play payloads (native hooks source-wired; routing/caps pure-tested; LAN delivery unverified) |
| [interaction](interaction.md) | using items and blocks, finishing eating or drinking, attacking, break vetoes and cancellation notifications, digging speed |
| [entities](entities.md) | per-type jump/fall/contact class queries and transient class attributes and native armor-base mirroring (source-wired dimension/owner-checked load-time/in-place-NBT class-base restoration; later metal/size mutations unconnected, native-unvalidated), split mob-spawn preparation/completion with once-only Navigation-owned identity (runtime common-entry yaw/UUID trace checked; natural/spawner/event/marked-structure/conversion and offhand gear/save source wiring; full review/native validation pending), derived entity-type declarations and symbolic type/default-attribute-supplier/network/command output (native-unvalidated; state/tracking policy incomplete), entities entering and leaving worlds, a player shearing a mob (cancel it, source-wired, native-unvalidated), damage (allow it or change its amount, with the attacker's stack), death |
| [loot](loot.md) | change loot tables as they load; change the items a block, mob or use is about to drop (`MODIFY_DROPS`, source-wired, native-unvalidated); ordered borrowed living-death drop callbacks and once-only head claims (pure-tested helpers; item-owned drop RNG and optional player context adapted to the fixed host pin; diagnostic Wither negative-age star and 50-XP collection checked; callback ordering and combined contracts unvalidated) |
| [trades](trades.md) | common/rare candidate factories, ordered weighted snapshots and caller RNG; native pool/tag/offer transport included in v13 plan and host/API/game-entry typechecks; combined review, linking and gameplay pending |
| [resources](resources.md) | pack-aware client assets/PNG pixels, reload notifications, JSON-folder callbacks, same-pass multi-folder ordered JSON layers and owned entity/item registry-tag snapshots (server v13 plan/typechecked; production snapshot/top-layer helpers tested; linking/gameplay pending; client JSON inert) |
| [items](items.md) | owned reload item prototypes with same-loader custom-data capture and separately validated item-level crafting remainders (source-wired, native-unvalidated; reduced StackData view), edit existing tooltip entries without flattening rich text, append simple/translated lines, use items over time; item ender-mask behavior with explicit native fallback (source-wired, native-unvalidated); standing/wall item binding declarations (owned-table overlay/cache policy tested and symbolic export/provider/cache-identity hooks source-wired; native validation remains an explicit stub) |
| [block](block.md) | static `PistonBehavior` overrides (source-wired per-state projection/cache identity; native-unvalidated), shape-update tick requests and supplied-state tags (source-wired, native-unvalidated), typed path-node classification (static query source/plan pass; build/game pending), slime/sticky identity and pair predicates (native piston stubs), swept contact velocity (`entity_inside_motion`, falling-side/suppression checked; native sound requests checked; audible playback/LAN unverified; cow/item four-face falling-side/sound gates pass; other species/flat contact unverified), player landing damage/motion (`fall_on` / `update_entity_after_fall_on`, pinned build and player damage/bounce/suppression checked; mob adapter and four-kind cow immunity/bounce with stone damage control checked; other species unverified; item post-fall native-support routing fixture passes; changed binding build/game pending, historical four-kind evidence does not certify current inputs), block behaviour (including generation-marker plant survival; natural End grass/fern checked), replaceability/generator-depth queries (native build and Blood generation/reopen checked; live depth unavailable), explicit write flags, native break-block (build/live planted fungus checked; loot/fluid effects unverified), live native features; native tree + extra decorators (live oak fixture checked), bound roots (five live cases checked); ordered native/custom plans (three live cases checked) |
| [worldgen](worldgen.md) | feature/decorator codecs, live trees, bound roots and ordered decoration; explicit structure codecs/plans with height/biome/RNG queries; explicit native plain/registered templates, clipped DATA markers, per-piece state and shared world/RNG feature placement; natural plain-template starts/cache/save-reload checked in game; natural cross-chunk markers/save-reload and original-settings End-island trees/vines checked; End saved-start adapter/full build and four-island reopen fixture checked (Nether/LAN unverified); avoid-biome sets and automatic native feature-type dispatch unconnected |
| [color](color.md) | block/item tint registration and direct lookup, owned item-stack context; verified world-chunk base-face dispatch/native fallback; native item/hand baked-layer and common terrain-particle adapters source-declared (unvalidated); independent particle/other rendering paths unconnected |
| [block-entity](block-entity.md) | NBT, block entities for a mod's blocks (saved data, server ticking, client ticker source wiring and transient snapshots (native-unvalidated, including transition resets), sync to clients and menu data slots; registered-custom factory policy tested; native head/skull factory wired in source but blocked pending native attachment plan review and lifecycle verification) |
| [screen](screen.md) | engine-scoped slot-action callbacks with staged hotbar/offhand access, per-slot pick-all exclusions and local/guest data connections (bridge/API regressions, screen examples and v10 attachment guards pass); native component comparison, stack-limit queries, crafting-remainder lookup and player-inventory return are source-wired (native types/gameplay and remote recipe-state sync pending), source-stack snapshot transport across slots/cursor, empty-slot normalization and complete quick-move remainder write-back (production conversion/rollback fixture only; R2 native/game validation pending), windows for a block's menu: static and dynamic slot positions, drawing, clicks, tooltips, menu buttons, what each slot takes, shift-click routing, what a hopper may put in |
| [renderer](renderer.md) | custom item/worn skull registry and file/shared-document special-renderer source paths (actual helpers trace-tested; updated document adapter host/API typechecked; baking/reload/pixels unvalidated); per-stack worn armor layers (`ArmorRenderer`; pure-tested, host edits source-declared and unbuilt); drawing a mod block's block entity in the world; static keep-hint policy (client tickers uncached, registry-poison cache rejection pure-tested; native generation/token invalidation source-wired, unvalidated); exact submitted-state properties and optional frame clocks; custom model-layer definitions/registration, owned model-part trees, pose reset, native layer baking/part drawing and item display-context selection including Head (pure behavior tested; startup forwarding adapted to the pinned layer event, adapted-host API typecheck passed; game pending) |
| [fluid-storage](fluid-storage.md) | the fluid storage of block entities followed through forwarding blocks, with the `fluid_forward`, `fluid_reader` and `fluid_drainer` hooks of block-entity.md (pure rules and test world; host adapter not written) |
| [neighbours](neighbours.md) | reading and taking items and fluid from the containers and tanks beside a ticking block entity (pure trait, scope and test world; host adapter written over the host's container, fluid and block state calls, not yet compiled or game-verified) |
| [fluid](fluid.md) | fluids a mod defines: id, temperature, light, density, viscosity, colour; owned native-plus-defined reload membership and fluid tags through `reload_registry("minecraft:fluid")` (source-wired, native-unvalidated) |
| [i18n](i18n.md) | translations: text per language, the language in use, switching |
| [world](world.md) | owned item-fragment particle factories and exact-type overrides (native client request/cube emission/shared host-remote event adapters source-wired, unvalidated; other emitters/network routes incomplete), reading/changing blocks, block tags, authoritative level-stream random floats (native build/projectile sound callbacks checked; exact RNG traces unverified) |
| [projectile](projectile.md) | a projectile hits something: act on the world, cancel the hit; a projectile launches, a bow shoots |
| [creative](creative.md) | creative inventory tabs, data-carrying stacks, entries for existing tabs; eleven-tab paging/cursor preservation/named-stack extraction fixture verified on the fixed host; separate vanilla-stack extraction/full quit-reopen verified (named/paged persistence, search and LAN unverified) |
| [player](player.md) | messages, stack data, effects, reading hunger and feeding players with `add_food`, testing with `MemoryPlayerOps` |
| [attachments](attachments.md) | typed player/entity data; explicit sync policy and native entity forwarding (source-wired; entity save/LAN unverified) |
| [config](config.md) | config files |
| [commands](commands.md) | commands |

## What a call does when the host cannot

PlutoLib always offers the whole API. A call the pinned Horizon cannot honour yet is accepted and does nothing (or returns its documented default); `base.md` describes the support matrix, and each such call says "Stub" on its page.

## Test a mod's logic

Code that uses a `ServerPlayer` runs against an in-memory game in a plain `cargo test`:

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayConnectionEvents, PlayerConnection, ServerPlayer};

ServerPlayConnectionEvents::JOIN.register(|join| {
    join.player.give_item("minecraft:bread", 1);
});

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || {
    ServerPlayConnectionEvents::JOIN.invoke(&PlayerConnection { player: player.clone(), name: player.name.clone() });
});
assert_eq!(game.items[&(1, "minecraft:bread".to_owned())], 1);
```
