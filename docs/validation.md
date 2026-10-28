# Validation

On Linux, with Horizon `b6c936f82b08798057161bdfa577e3aac53fb91e` and
Mixins `0500ff24aa5933fca95a877ef41125353c4b4fe3`:

- Four HEAD injections applied to an exported Horizon source tree. The
  `horizon-client` binary passed `cargo check` and a locked fast build.
- `python3 -m unittest discover -s tests -v` passed the overlay registration
  and input preservation test.
- `tools/game_smoke.py` passed in isolated integrated-server and in-frame worlds: flint and sticks
  became the three named part IDs; the parts became a flint pickaxe; mining
  stone dropped cobblestone and increased damage to 1. The screenshot is
  [here](game-smoke.png).
- An independent save/reopen retained the pickaxe and damage 1. With an
  item set to damage 84 through `/item`, mining stone raised damage to 85
  while keeping the item. After another save/reopen and the follow-up speed
  hooks, the damage-85 tool mined dirt in 51 ticks and stayed at damage 85
  in both in-frame and integrated-server modes. Full broken-tool parity
  remains unverified.

LAN, dedicated server, repair, traits,
material-variable part rendering and the original station UI have not been tested or
implemented. The screenshot shows the fixed flint/wood composition using upstream textures.

An independent fixture bundle and this mod planned together (five injections)
and the combined staged client passed `cargo check`. Combined gameplay was not
run.

To reproduce combined planning/compilation with a fresh output path (after
`tools/stage.py`):

```sh
mkdir -p target/fixture-mods
cp -R mixins/tconstruct target/fixture-mods/
cp -R tests/fixtures/independent target/fixture-mods/
../horizon-mixins/target/debug/horizon-mixins apply \
  --workspace target/original --mods target/fixture-mods --out target/fixture-game
cargo check --locked --offline --manifest-path target/fixture-game/Cargo.toml \
  -p horizon-client --bin horizon-client --no-default-features
```

Use a fresh `fixture-mods`/`fixture-game` path for each independent run.
