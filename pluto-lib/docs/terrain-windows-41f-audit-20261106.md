# Terrain and Windows x64 source audit: official 41f

## Scope and result

Read-only comparison of official Horizon `f4dcf8a2` to
`41f822b659789780f58d32c63ddfb6887ea1b8bf`, with Mixins fixed at
`5a64af938ec86b00d53d16a408aabc0038fb3e57`. No checkout, compiler, planner,
Windows executable, installation, security change or GPU run was performed.
Only the official repositories were read. The companion JSON has exact source
links and the selected-file Git blob/SHA-256 evidence.

The [official comparison](https://github.com/horizon-rust/horizon/compare/f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd...41f822b659789780f58d32c63ddfb6887ea1b8bf)
contains 26 commits and 50 changed paths. Twelve of the thirteen selected terrain
source/lock files have the exact same Git blob, including Cargo.lock, mesher,
fluid tessellation, Assets, Fusion, models, sprites, streaming, game/mods and
packs. game.rs is the exception: its six complete diff hunks were applied in
memory, checking every old context line; the full result matches independently
read Git blob `ac695140626a8c542cb7c98e43979aa80e3d137b` (1,032,243 bytes,
SHA-256 `2488718332e589c8cbc15766c93857e83945b5f90b74f393ceaa1cd75c23891e`).
The entire PackResources impl remains byte-identical (2,583 bytes,
`db92017bff8481741595c0b687011a92af2b3f3caa3b451e460621925bca63e7`).

This does not establish new AST locations, whole-module fingerprints, type
compatibility or a complete plan. The separate observation contract
[8cfc1247](https://github.com/sofia-gros/horizon-mods/commit/8cfc1247a0c18d5c93bdd0c185fe1b2e797f2937)
changes only the Horizon revision and game.rs hash from the old contract. Its
collector code and eight Rust payloads remain unchanged. Twenty-two synthetic
plan/Python checks passed; compatibility remains null and production guards
remain unchanged until an actual complete plan is available.

## Screen and resource implications

- New official `Game::render_transparent_background` dispatches a cancelable
  native hook before the original dark fill. Inventory, creative, book, beacon,
  sign editor, stations and merchant now use it. A frame-local blur request is
  reset each frame and feeds the existing finite 1..64 menu-blur check. This is
  an official native API, not a newly implemented Pluto facade. A consumer that
  replaces a screen body must preserve its intended relationship to the new
  hook rather than assume old whole-body rendering is current.
- HudAtlas adds object-skin revision tracking; TabList uses a layout cache with
  skin/language/font/version inputs while keeping animated heart work separate.
  No measured rendering or performance conclusion follows from those changes.
- The selected terrain owner/load paths are unchanged from f4dc. In particular,
  `Assets::reload_with_models` still bypasses `load_with` and calls
  `load_with_parsed`; preparation bound only to `load_with` does not cover it.
  The Pluto owner-triple preparation seam remains PackResources::assemble.
  Synthetic reload/source tests are not actual Game reload or streaming proof.

## Three different Windows build paths

### Official game release build

The [official game-build workflow](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/.github/workflows/game-build.yml#L106-L185)
uses `windows-latest`, x64 output, `-C target-feature=+crt-static`, and:

```
cargo build --release --locked -p horizon-client -p horizon-client-lib \
  --bin horizon-client --bin horizon-ctl \
  -p horizon-prepare --bin horizon-prepare -p horizon-server --bin horizon-server
```

Audio/controller default features are retained. The reviewed workflow requests
Rust stable rather than a fixed compiler version; any new reproducible test must
record its actual compiler/target rather than silently equate it to our Linux
1.95 tests. It checks all four PE architectures, system-only DLL imports, baseline
x86-64 CPU instructions and help/version startup. Its real world smoke is on the
maintainer's self-hosted macOS resource runner, not proof of Windows GPU display.

The release workflow depends on original exporter classes built against the
maintainer's own Minecraft jars. It cannot simply be reused in the mods repo as
though those runners/resources were available. No release signing, new cache,
artifact, billing or permission arrangement is created by this audit.

### Official launcher native-mod build

The [managed native-build implementation](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/launcher/src/native_mods.rs#L1885-L1920)
is a different route. Its [toolchain source](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/launcher/src/rust_toolchain.rs#L16-L125)
declares Rust 1.99.1, `x86_64-pc-windows-gnu` and SHA-pinned WinLibs GCC16.2.0 /
MinGW-w64 14.0.0 MSVCRT. These are source facts, not verified downloads here.
It uses its own toolchain/Cargo home and scrubs inherited compiler overrides.
Its native mixins default is exactly 5a64. The default performance profile uses
workspace `fast`; release is separately selectable. Default client features are
retained; the old `mods` feature is added only when the staged source has it.

Therefore an MSVC release executable and a GNU launcher native build need
separate target evidence. Old Linux `--no-default-features` checks establish
neither route. Do not treat a Linux host type gate or std predicate test as a
Windows link/start or playable-build proof. Native trust prompts and system
security settings remain the user's decisions; this audit does not operate or
recommend automatically bypassing them.

### Existing TConstruct Play-TConstruct.cmd

At fixed mods main
[08afa992](https://github.com/sofia-gros/horizon-mods/blob/08afa992d5091e03360f3473ea93fd17aa793987/tconstruct/tools/play-windows.ps1),
the custom playtest script still reads compatibility pins 0c7b/ddd, uses the
user-installed stable MSVC/VS C++ tools, and builds `--no-default-features
--profile fast`. It requires an existing prepared Horizon26.3 resource folder
and an extracted matching Pluto bundle, then runs the content preparation flow.
The [Windows guide](https://github.com/sofia-gros/horizon-mods/blob/08afa992d5091e03360f3473ea93fd17aa793987/tconstruct/docs/windows-playtest.md)
explicitly records an unresolved startup error and no actual PC gameplay test.
This custom script is not the managed official launcher route above and is not
updated to 41f by a successful separate planner experiment.

## Delivery and resource boundary

The official [xtask packager](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/xtask/src/main.rs#L224-L245)
requires the game, ctl, prepare, server and `exporters/manifest.json`. It packages
original preparation code, not Minecraft assets. The user's launcher prepares
resources from their licensed Minecraft Java installation. The packaged manual
[PLAY instructions](https://github.com/horizon-rust/horizon/blob/41f822b659789780f58d32c63ddfb6887ea1b8bf/packaging/PLAY.txt)
name Minecraft Java26.3 and Java25. No Mojang assets, credentials or prepared
resource tree should be published in the mods repository or proof artifact.

Official `.hnmod` validation caps one archive at 512 entries and 64MiB unpacked;
a local folder build does not have that archive limit. The complete646 tree is a
planner composition, not automatically a deliverable single `.hnmod`. Validate
the exact TConstruct and Pluto archives individually, with matching Horizon /
Mixins pins, and keep the content `.hmod` enabled in the same intended instance.
Native embedded data does not automatically become runtime assets/lang/textures.
The existing TConstruct packager enforces the same per-archive budgets.

## Next acceptance gates, not completed work

1. Repair the exact current main hook blocker without dropping its behavior,
   obtain the latest complete plan and preserve whole-plan hash/bytes before
   terrain collection. The f4dc full646 run stopped at picked_stack; no terrain
   identities or old complete-plan guards were reached.
2. Review actual 41f hook/module identities, competitor inventory and lifecycle
   assumptions; then update a latest-only guard cutoff without weakening it.
3. Apply and type-check the exact combined candidate. Keep runtime, dependencies,
   source seals, bundled files and content revisions tied to that candidate.
4. Build and link the chosen Windows x64 route with exact compiler/target/flags,
   default gameplay features and all required packages. Check PE/imports/startup
   and independently verify the deliverable archives/resources contract.
5. Run the resulting version with authorized user-owned resources on a real
   Windows11x64 GPU, checking start, world load, Tinkers content, UI and reload.
   User machine availability and GPU behavior have not been observed here.

Performance/profile experiments remain excluded. The bounds CI proved only 28
std tests and limited microtiming; old CPU success remains on 624c/e8. No new
performance claim, production optimization or experiment activation is made.
