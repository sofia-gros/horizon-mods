#!/usr/bin/env python3
"""Prepare std-only checks of exact official index types/lookups and our adapter.

No native source is modified. RecipeIndex::load is deliberately not included:
this fixture proves record conversion/lookup/update, not parsing or world load.
The outer multi-field publication example is modeled, not a running Game.
"""
import argparse
import json
from pathlib import Path
import shutil
import tomllib
import prepare_advancement_projection_fixture as common

ROOT = Path(__file__).resolve().parents[1]
SOURCE = "crates/horizon-client/src/recipe_book.rs"
PARTS = [
    ("#[derive(Clone, Debug, Default)]\npub struct RecipeIndex", "/// `RecipeDisplay.craftingStation` of a recipe type."),
    ("    pub fn contains(&self, id: &str) -> bool {", "/// The recipe ids a knowledge book names:"),
]


def fragment(text, start, end):
    if text.count(start) != 1 or text.count(end) != 1:
        raise ValueError("Official fragment selectors are no longer unique")
    return text[text.index(start):text.index(end, text.index(start))]


def verify(output):
    evidence = json.loads((output / "fixture-sources.json").read_text())
    host = Path(evidence["official_source"])
    common.clean_official(host, evidence["official_revision"])
    original = (host / SOURCE).read_bytes()
    if common.digest(original) != evidence["official_source_sha256"]:
        raise ValueError("Official index source drifted")
    for path, sha in evidence["files_sha256"].items():
        if common.digest((output / path).read_bytes()) != sha:
            raise ValueError("Fixture source drifted: " + path)
    for part in evidence["fragments"]:
        exact = fragment(original.decode(), part["start"], part["end"])
        if common.digest(exact.encode()) != part["sha256"] or exact not in (output / "recipe_book.rs").read_text():
            raise ValueError("Official declaration/lookup fragment differs")
    return evidence


def prepare(host, output):
    host = host.resolve()
    native = ROOT / "mixins/pluto_native_creative"
    pin = tomllib.loads((native / "horizon-mixin.toml").read_text())["compatibility"]["horizon_revision"]
    common.clean_official(host, pin)
    if output.exists():
        raise ValueError("Output must be new")
    output.mkdir(parents=True)
    for src, dest in [(native / "inventory-api/src/recipe_book/index.rs", "index.rs"),
                      (native / "recipe-index-patch.rs", "adapter.rs"),
                      (ROOT / "tests/native/recipe_index_patch.rs", "tests.rs")]:
        shutil.copyfile(src, output / dest)
    original = (host / SOURCE).read_text()
    parts = [fragment(original, *selectors) for selectors in PARTS]
    (output / "recipe_book.rs").write_text(
        '#[path="index.rs"] pub mod index;\n' + parts[0] + 'impl RecipeIndex {\n' + parts[1]
        + '#[path="adapter.rs"] pub mod pluto_index_patch;\ninclude!("tests.rs");\n')
    (output / "lib.rs").write_text('extern crate self as pluto_inventory_api;\npub mod recipe_book;\n')
    evidence = {
        "official_source": str(host), "official_revision": pin,
        "official_source_sha256": common.digest((host / SOURCE).read_bytes()),
        "fragments": [dict(start=start, end=end, sha256=common.digest(part.encode()))
                      for (start, end), part in zip(PARTS, parts)],
        "files_sha256": {str(p.relative_to(output)): common.digest(p.read_bytes()) for p in output.iterdir()},
        "scope": "Exact native RecipeIndex/Known and lookup methods; production API/adapter; modeled outer publication fields. No loader, Game, networking or ownership proof.",
    }
    (output / "fixture-sources.json").write_text(json.dumps(evidence, indent=2) + "\n")
    verify(output)
    print(output.resolve())


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--horizon-source", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    if args.verify:
        verify(args.output)
        print("Exact official index fragments and adapter/API fixture hashes verified")
    elif args.horizon_source is None:
        parser.error("--horizon-source is required")
    else:
        prepare(args.horizon_source, args.output)
