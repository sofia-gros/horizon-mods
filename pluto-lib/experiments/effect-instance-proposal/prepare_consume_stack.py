"""Generate an UNAPPLIED argument-only proposal; never edit/apply/build a host."""
from __future__ import annotations

import argparse
import difflib
import hashlib
from pathlib import Path
import subprocess

REVISION = "624c4f6265c02296fbe8a80f029dd0d987fb94c0"
HASHES = {
    "crates/horizon-client/src/game.rs": "cdfc9247cce139c00e5a36266812c7f7f42ccbb565d67705a896520b4b62b22c",
    "crates/horizon-client/src/game/consume.rs": "6e24f541bc2d6c7a2eccf15d7711950be3eba3ccdfd603dd4e915047306222c6",
    "crates/horizon-client/src/game/item_use.rs": "9d3de0dc51dd26ad8c035131321dfc880795b1a195a4e880fcb1b1d04adf569e",
    "crates/horizon-client/src/game/native_hooks.rs": "195b19d3647e21caf33757aa690b9df8cc5b562e2943a36404f7a26c6e4a60e1",
    "crates/horizon-client-render/src/effects.rs": "5269c3c35f38c4c658d4409b0f7badae4f0ddca0b68df36c5197a5dd51f735dd",
    "crates/horizon-world/src/consumable.rs": "ba3575cb85866cb940f33a1ac86455d71cf0cc6395b848588a8ee1fe33f5c326",
    "crates/horizon-world/src/consume_effect.rs": "03f2aac6879ce133347d42f6f6667d62c06d956ff9faac5dc14182a788b6a236",
}
GAME = "crates/horizon-client/src/game.rs"
CONSUME = "crates/horizon-client/src/game/consume.rs"
OLD_SIGNATURE = "    fn apply_consume_effect(&mut self, effect: &horizon_client::items::ConsumeEffect) {\n"
NEW_SIGNATURE = """    fn apply_consume_effect(
        &mut self,
        effect: &horizon_client::items::ConsumeEffect,
        consumed_stack: &ItemStack,
    ) {
"""
OLD_CLEAR = """            ConsumeEffect::ClearAllEffects => {
                self.potions.effects.clear();
                self.apply_effect_status();
            }
"""
NEW_CLEAR = """            ConsumeEffect::ClearAllEffects => {
                self.clear_effects_from_consumed_item(consumed_stack);
            }
"""
ANCHOR = "    fn consume_held(&mut self) {\n"
HELPER = """    // UNAPPLIED proposal: carry the actual consumed snapshot, but do not
    // interpret it as current-slot identity or effect-instance cure authority.
    // No filtering is enabled until the instance/owner contracts are supplied.
    fn clear_effects_from_consumed_item(&mut self, _consumed_stack: &ItemStack) {
        self.potions.effects.clear();
        self.apply_effect_status();
    }
"""
OLD_CALL = "            self.apply_consume_effect(effect);\n"
NEW_CALL = "            self.apply_consume_effect(effect, stack);\n"


def replace_once(text: str, old: str, new: str) -> str:
    if text.count(old) != 1:
        raise ValueError("Expected exactly one pinned source anchor")
    return text.replace(old, new, 1)


def read_sources(official: Path) -> dict[str, str]:
    official = official.resolve(strict=True)
    actual = subprocess.check_output(
        ["git", "-C", str(official), "rev-parse", "HEAD"], text=True
    ).strip()
    if actual != REVISION:
        raise ValueError("Not the selected official revision")
    sources = {}
    for relative, expected in HASHES.items():
        path = official / relative
        if path.is_symlink() or not path.resolve().is_relative_to(official):
            raise ValueError("Pinned source must be a regular in-tree file")
        data = path.read_bytes()
        if hashlib.sha256(data).hexdigest() != expected:
            raise ValueError("Pinned source changed: " + relative)
        sources[relative] = data.decode("utf-8")
    return sources


def transform(sources: dict[str, str]) -> dict[str, str]:
    result = dict(sources)
    game = replace_once(sources[GAME], OLD_SIGNATURE, NEW_SIGNATURE)
    game = replace_once(game, OLD_CLEAR, NEW_CLEAR)
    result[GAME] = replace_once(game, ANCHOR, HELPER + ANCHOR)
    result[CONSUME] = replace_once(sources[CONSUME], OLD_CALL, NEW_CALL)
    return result


def patch(sources: dict[str, str]) -> str:
    changed = transform(sources)
    return "".join(
        "".join(difflib.unified_diff(
            sources[path].splitlines(keepends=True),
            changed[path].splitlines(keepends=True),
            fromfile="a/" + path,
            tofile="b/" + path,
        )) for path in [GAME, CONSUME]
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--official", required=True, type=Path)
    parser.add_argument("--write", action="store_true", help="Write only the sibling UNAPPLIED patch artifact")
    args = parser.parse_args()
    proposed = patch(read_sources(args.official))
    destination = Path(__file__).resolve().with_name("UNAPPLIED-consumed-stack-cure-boundary.patch")
    if args.write:
        if destination.is_symlink():
            raise ValueError("Refusing symlink output")
        destination.write_text(proposed)
    else:
        print(proposed, end="")


if __name__ == "__main__":
    main()
