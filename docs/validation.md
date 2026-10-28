# Validation

Linux, Horizon `b6c936f82b08798057161bdfa577e3aac53fb91e`, Mixins
`0500ff24aa5933fca95a877ef41125353c4b4fe3`, with prepared 26.3 resources:

- Sixteen injections and three support modules applied to an exported Horizon
  source tree. The client passed `cargo check` and a locked `fast` build.
  With `--no-default-features`, the dependency tree contains no Wasmtime runtime.
- The resource overlay unit test and three transaction unit tests passed. The
  latter cover failed atomic takes, repeated takes, split repair inputs, excess
  material and preservation of custom data.
- In both integrated-server and in-frame single-player modes, normal crafting
  produced the Pattern and two tables, and both tables were placed as blocks.
  GUI operations made the three parts, assembled the pickaxe and repaired it;
  the named tool's custom data survived repair and save/reload. Reopening the
  Part Builder retained its Pattern. The game screenshots are
  [Part Builder](part-builder.png) and [Tinker Station](tinker-station.png).
- In both modes, breaking a Part Builder returned its stored Pattern, flint and
  the table item. Two-player LAN races for a part, assembly and repair each
  produced exactly one output and consumed the inputs once. Two separate worlds at the same coordinates kept independent inputs. The repaired race
  result retained the name and custom data. Integrated assembly contention was
  repeated twice after fixing a stale-view synchronization issue.
- The fixed pickaxe's damage-85 slow dirt mining and retained tool were checked
  in both game modes. Full original broken behavior is not established.

The client and server use a compact subset of the original interface: part
icons directly take a result, and assembly and repair have separate icons.
Shift-click, drag and pickup-all are blocked. Oak planks are the only supported
wood input, and the part/tool combination is fixed. Other materials, tool
components and the full Station interface are not implemented. Dedicated
server and arbitrary unrelated item components are not verified.

## Reproducing game checks

Build and prepare the resource root as in the README. Under X11/Vulkan (or
Xvfb with software Vulkan), run each script with a fresh world; add `--in-frame`
for the alternate single-player path:

```sh
for test in stations spill race race_station race_repair recipes isolation broken; do
  python3 "tests/game/$test.py" --game target/build/fast/horizon-client \
    --ctl target/build/fast/horizon-ctl --resources target/resources
  python3 "tests/game/$test.py" --game target/build/fast/horizon-client \
    --ctl target/build/fast/horizon-ctl --resources target/resources --in-frame
done
```

Each script leaves its isolated world and logs under `target/`. The unit test
is `python3 -m unittest discover -s tests -v`; the Rust transaction test is:

```sh
cargo test --locked --offline --manifest-path target/modded/Cargo.toml \
  -p horizon-client --lib --no-default-features --profile fast tconstruct_tx::tests
```

## Independent fixture

`tests/fixtures/independent` is a second bundle with a HEAD hook on the same
station-click function, ordered before TConstruct. To construct a combined
stage after `tools/stage.py`:

```sh
mkdir -p target/fixture-mods
cp -R mixins/tconstruct target/fixture-mods/
cp -R tests/fixtures/independent target/fixture-mods/
../horizon-mixins/target/debug/horizon-mixins apply \
  --workspace target/original --mods target/fixture-mods --out target/fixture-game
cargo build --locked --offline --manifest-path target/fixture-game/Cargo.toml \
  -p horizon-client --bin horizon-client --no-default-features --profile fast \
  --target-dir "$PWD/target/fixture-build"
```

The fixture logs `independent fixture station-click hook executed` the first
time its hook runs. Combined gameplay passed `tests/game/stations.py` in both
integrated-server and in-frame modes, and the fixture message was present in
each game log. Run the same test with
`--game target/fixture-build/fast/horizon-client`; inspect `Player/game.log`
in the printed run directory. Use a fresh output path for a new stage.

### GUI rendering regression

The 176×184 station panel is centered using its actual height. The Part Builder
buttons and Tinker Station slot borders are cut from the original atlases; the
station slot background uses the original 28% opacity. The three assembly
input positions match the original pickaxe layout. The prototype still uses
direct part output buttons and lacks original layout selection and repair UI.

At 960×540 (integrated server) and 640×360 (in-frame), visually inspected
empty and populated panels, hovered slot tooltips, and the reopened Part
Builder after save/reload. The station test exercised click targets, output
collection and repair. The LAN test captured both clients after allowing frames
to update and verified a single output under concurrent clicks.

Reviewer testing found that
a repaired tool's Lore remained in saved server data and in-frame display, but
the integrated client's display dropped Lore. Other arbitrary component
round-trips are not established; do not treat the custom-data synchronization
as general component support.
