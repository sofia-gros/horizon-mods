Task files for Codex, one JSON file per job, written by the owner's agent (see docs/codex.md for how they are used).
The order and the dependencies are in each file ("order", "depends_on"); `python tools/codex_next.py` says which one is next. `python tools/codex_guard.py --lint` checks them.
