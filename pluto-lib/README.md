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

In `mod/` (built with Horizon's official launcher, checked locally):

- `base`: `Event` (phases, ordering, combiners), `Identifier`, `ActionResult`, and the support matrix: the list of calls that exist but do nothing yet because the host cannot do them.
- `networking_api_v1`: `ServerPlayConnectionEvents::JOIN`, fed from Horizon's `player_logged_in` hook. It has been compiled and linked, not yet seen firing in game.

The old unified bundle in `mixins/` (creative tabs, inventory, fluids, protection, tool rules and more) still ships and is migrated into `mod/` module by module.
Of its eleven composition guards only `protection` passes on the current Horizon; the rest need re-review ([docs/migration/](../docs/migration/)).

## Building and testing

```sh
cargo test --manifest-path pluto-lib/tests/base/Cargo.toml        # the host-independent core, seconds
python -m unittest discover -s pluto-lib/tests                     # packaging and guard tests
python tools/package_bundle.py pluto-lib/mod dist/pluto-lib.hnmod  # from the repository root
```

See [../docs/development.md](../docs/development.md) for the launcher build. Third-party notices: [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
