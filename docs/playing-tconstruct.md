# Try the TConstruct development port

The port is a work in progress against TConstruct 1.20.1, fixed upstream revision `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. Its eventual goal is a complete port. On this repository's current `main`, the playable subset is wood/flint pickaxe parts, Part Builder selection, basic assembly, single-material Station repair, and the connected mining, melee and durability behavior. The tool currently uses a vanilla wooden-pickaxe carrier; it does not yet render all original material-dependent parts.

![Flint pick head selected in the Part Builder](screenshots/part-builder-interactive/selected-material.png)

The pattern and material are placed on the left, with three supported part buttons and a bounded material-information panel. The image is a real game screenshot, not a claim that all original parts, materials or tooltips work. Original GUI assets retain their [source notice](<../Tinkers' onstruct/assets/tconstruct/source_notice.txt>) and [MIT license](<../Tinkers' onstruct/assets/tconstruct/license.txt>).

## Build both packages

Use the matching Horizon fork, Git, Rust and `wasm32-unknown-unknown`. The example assumes both repositories are siblings and a Unix-like shell; adjust paths to your environment. Rust 1.95 or newer is needed by the current mod runtime.

```sh
git clone https://github.com/sofia-gros/horizon.git
git clone https://github.com/sofia-gros/horizon-mods.git
cd horizon
git checkout fc4848f4d7920390ea3aa8e6a5f804111f93a45d
rustup target add wasm32-unknown-unknown
cargo build -p cargo-horizon-mod
cargo build --profile fast -p horizon-client --no-default-features --features mods
HORIZON_REPO="$PWD"
cd "../horizon-mods/Tinkers' onstruct"
git checkout ce3a5ab5e8a51ea01d1da502ce5f11d0584ef0ff
cargo test
"$HORIZON_REPO/target/debug/cargo-horizon-mod" horizon-mod build
cd server-tools
"$HORIZON_REPO/target/debug/cargo-horizon-mod" horizon-mod test
"$HORIZON_REPO/target/debug/cargo-horizon-mod" horizon-mod check target/horizon-mod/tconstruct_tools.hmod
```

`test` builds the server package and runs its scenario; its artifact is `server-tools/target/horizon-mod/tconstruct_tools.hmod`. The content/client package is `target/horizon-mod/tconstruct.hmod` relative to `Tinkers' onstruct/`. Package files are build outputs, not committed game resources.

## Run a prepared game

Prepare Horizon's game resources from your own Minecraft installation using [Horizon’s source-build instructions](https://github.com/sofia-gros/horizon/blob/fc4848f4d7920390ea3aa8e6a5f804111f93a45d/README.md#build-from-source). From the `server-tools` directory used above, for example:

```sh
"$HORIZON_REPO/target/fast/horizon-client" \
  --resources "$HORIZON_REPO/.local/horizon-client/26.3" \
  --dev-mod "../target/horizon-mod/tconstruct.hmod" \
  --dev-mod "target/horizon-mod/tconstruct_tools.hmod"
```

In a test world, create a pattern and Part Builder, insert a pattern and supported material, then select a head, handle or binding. Use the three parts in the Tinker Station to make the supported pickaxe; the same Station also accepts a damaged supported pickaxe and matching repair material. The GUI's actual placement and behavior are still a subset. The [isolated Linux probes](linux-tconstruct-game-verification.md) provide reproducible test steps; their results apply to the revisions they name.

Dynamic material textures, the smeltery, the rest of the tools/materials and many modifiers are not implemented. The name-entry, alternate slot layouts and multi-slot repairs in open development PRs are not part of this `main` baseline. The port's [roadmap](tconstruct-roadmap.md) and [historical README](<../Tinkers' onstruct/README-history.md>) retain earlier work and test evidence.
