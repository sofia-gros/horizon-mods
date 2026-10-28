# Validation

On Linux, with Horizon `b6c936f82b08798057161bdfa577e3aac53fb91e` and
Mixins `0500ff24aa5933fca95a877ef41125353c4b4fe3`:

- Three HEAD injections applied to an exported Horizon source tree. The
  `horizon-client` binary passed `cargo check` and a locked fast build.
- `python3 -m unittest discover -s tests -v` passed the overlay registration
  and input preservation test.
- `tools/game_smoke.py` passed in isolated integrated-server and in-frame worlds: flint and sticks
  became the three named part IDs; the parts became a flint pickaxe; mining
  stone dropped cobblestone and increased damage to 1. The screenshot is
  [here](game-smoke.png).
- An independent save/reopen retained the pickaxe and damage 1. With an
  item set to damage 84 through `/item`, mining stone raised damage to 85
  while keeping the item. After another save/reopen, a further stone break
  timed out after 160 ticks, leaving both stone and the damaged tool intact.
  This is a deliberately limited prototype, not original broken-tool parity.

LAN, dedicated server, repair, traits,
proper part rendering and the original station UI have not been tested or
implemented. The game screenshot shows the current vanilla placeholder.
