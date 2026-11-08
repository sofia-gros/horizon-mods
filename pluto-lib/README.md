# PlutoLib

**The Fabric API of Horizon.** An independent mod that other mods call. It follows Fabric API's names, features and behaviour (Horizon Mixins is expected to
offer the NeoForge-like API), is released once per Horizon version, and is general purpose: any mod can use it, not only Tinkers' Construct.
Where Fabric API has nothing but a mod needs it (for example translations, guide-book screens), PlutoLib may add its own API.

Principles and what is decided: [docs/design-principles.md](docs/design-principles.md). The specification (modules, naming, packaging): [docs/fabric-api-design.md](docs/fabric-api-design.md).
Which of today's Tinkers hooks each module will replace: [docs/fabric-api-hook-map.md](docs/fabric-api-hook-map.md) (generated).

## Using it

A mod declares PlutoLib in its `horizon-mixin.toml` and calls the `[api]` crate:

```toml
[dependencies]
pluto_lib = "^0.1"
```

```rust
use pluto_lib::ServerPlayConnectionEvents;

ServerPlayConnectionEvents::JOIN.register(|player| println!("{} joined", player.name));
```

## What exists today (0.1.0, unreleased)

In `mod/` (built with Horizon's official launcher):

- `base`: `Event` (phases, ordering, combiners), `Identifier`, `ActionResult`, and the support matrix: the list of calls that exist but do nothing yet because the host cannot do them.
- `lifecycle_events_v1`: `ServerLifecycleEvents` (`SERVER_STARTING`, `SERVER_STOPPING`), `ServerTickEvents` (server and world tick start and end), `ServerChunkEvents` (`CHUNK_LOAD`, `CHUNK_UNLOAD`).
- `networking_api_v1`: `ServerPlayConnectionEvents` (`JOIN`, `DISCONNECT`); callbacks receive a `ServerPlayer`.
- `player_api`: `ServerPlayer` with `send_message` and `give_item`. The handle is plain data; its methods reach the game only while a callback that carries a player runs (outside, they do nothing and say so).
- `data_attachment_api_v1`: `AttachmentRegistry::create(id, default)` and `has_attached` / `get_attached` / `get_attached_existing` / `set_attached` on `ServerPlayer`; values are serde types saved with the player. Players only so far; worlds, chunks, block entities and other entities follow.
- `config_api` (not in Fabric API; NeoForge's `ModConfigEvent`): `ConfigRegistry::register` asks for `config/<mod id>-<kind>.toml` with default text; `ConfigEvents::{LOADING, RELOADING, UNLOADING}` hand over the file's text.
- `command_api_v2`: `CommandRegistrationCallback::EVENT` hands a registrar; `register(usage, handler)` adds a command by one usage line (`pay <player:player> <amount:integer>`), the handler gets a `CommandContext` (the player and the argument words). `ServerPlayer` also gained `count_item`, `take_item` and `empty_slots`.
- `events_interaction_v0`: `UseItemCallback`, `UseBlockCallback`, `AttackEntityCallback`, `PlayerBlockBreakEvents::BEFORE` (a veto cancels the break).
- `entity_events_v1`: `ServerEntityEvents` (`ENTITY_LOAD`, `ENTITY_UNLOAD`), `ServerLivingEntityEvents::ALLOW_DAMAGE`.
- `loot_api_v2`: `LootTableEvents::MODIFY` (edit a loading loot table's JSON).
- `resource_loader_v0`: `ResourceManagerHelper` reload listeners for client resources.
- `item_api_v1`: `ItemTooltipCallback` (add tooltip lines).

Each is fed from Horizon's stable native hook points (`hooks.rs`). Callbacks receive reduced, owned data. Some Fabric members exist as stubs (`SERVER_STARTED`, `SERVER_STOPPED`, `PlayerBlockBreakEvents::AFTER` and `CANCELED`, `AFTER_DEATH`); the list is `base/support.rs`.

**Verified in a running game:** a mod that depends on PlutoLib and uses only its API (`tests/probe`, run by `tools/smoke-local/probe_pluto_lib.py`) saw `SERVER_STARTING`, `CHUNK_LOAD`, `JOIN`, `END_SERVER_TICK` and `LootTableEvents::MODIFY` in the game log, and from the `JOIN` callback it sent the player a message and gave them an item (`send_message=true give_left=0`). The other events compile and follow the same path but have not been seen firing yet (the client-resources reload did not fire in the offscreen test run).
The old unified bundle in `mixins/` (creative tabs, inventory, fluids, protection, tool rules and more) still ships and is migrated into `mod/` module by module.
Of its eleven composition guards only `protection` passes on the current Horizon; the rest need re-review ([docs/migration/](../docs/migration/)).

## Building and testing

```sh
cargo test --manifest-path pluto-lib/tests/pure/Cargo.toml        # the host-independent part and the compiled examples of docs/api, seconds
python tools/check_api_docs.py                                     # from the repository root: every public item is documented
python -m unittest discover -s pluto-lib/tests                     # packaging and guard tests
python tools/package_bundle.py pluto-lib/mod dist/pluto-lib.hnmod  # from the repository root
```

See [../docs/development.md](../docs/development.md) for the launcher build. Third-party notices: [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
