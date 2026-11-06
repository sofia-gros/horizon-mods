#!/usr/bin/env python3
"""Read-only, sealed-v15 composition research. Never applies or builds a host.

--write writes only this MOD experiment's patch and receipt. This is separate
from the immutable pristine prototype and from ordinary Pluto package guards.
"""
import argparse
import copy
import hashlib
import importlib.util
import json
from pathlib import Path, PurePosixPath
import re

HERE = Path(__file__).resolve().parent
SEAL_SHA = "e16a3781aae1dea6d1a627f4bd28641469e16e822dae9704d0507a75a8d02428"
GENERATOR_SHA = "971d2e32683e0ad5016e3c8d7fac9a7ff97fb68862715a580aa14695f7c42f97"
HELPER_SHA = "37a8a4520e3ee4442d915fdc886d57ed4dd4130f1730b423572eec222f106971"
V15_RUNTIME = "f5a83539525c26fd2142e26e43789a3dd0107033"


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def sealed_files(stage, seal_path):
    raw = seal_path.read_bytes()
    if digest(raw) != SEAL_SHA:
        raise ValueError("not the reviewed full v15 seal")
    seal = json.loads(raw)
    actual = set()
    for path in stage.rglob("*"):
        if path.is_symlink():
            raise ValueError("symlink in supplied stage")
        if path.is_file():
            actual.add(path.relative_to(stage).as_posix())
    if actual != set(seal):
        raise ValueError("stage file set changed")
    for relative, expected in seal.items():
        parsed = PurePosixPath(relative)
        if parsed.is_absolute() or ".." in parsed.parts:
            raise ValueError("invalid seal path")
        if digest((stage / relative).read_bytes()) != expected:
            raise ValueError(f"stage source changed: {relative}")
    return seal


def generator():
    if digest((HERE / "generate.py").read_bytes()) != GENERATOR_SHA:
        raise ValueError("pristine generator changed")
    if digest((HERE / "retained_book.rs").read_bytes()) != HELPER_SHA:
        raise ValueError("storage helper changed")
    spec = importlib.util.spec_from_file_location("sealed_book_generator", HERE / "generate.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def stage_rules(original):
    rules = copy.deepcopy(original)
    single = ("matches!(self.book,", "matches!(self.book.as_ref(),", 1)
    multi = ("self.book,\n", "self.book.as_ref(),\n", 3)
    assert rules["game/books.rs"].count(single) == 1
    assert rules["game/books.rs"].count(multi) == 1
    rules["game/books.rs"] = [
        (single[0], single[1], 4) if item == single else item
        for item in rules["game/books.rs"] if item != multi
    ]
    return rules


def appended_helpers(source):
    # Mixins appends these native support modules. Preserve the complete tail,
    # not just a substring of the current-book helper. This does not parse Rust.
    found = re.search(r"(?m)^pub\(crate\) mod pluto_[A-Za-z0-9_]+ \{", source)
    return None if found is None else source[found.start():]


def compose(stage, seal_path, official):
    before = sealed_files(stage, seal_path)
    g = generator()
    # Verify the pristine artifact first, using the original unmodified rules.
    pristine_patch, pristine_receipt = g.generate(official)
    if pristine_patch != (HERE / "prototype.patch").read_text():
        raise ValueError("pristine prototype changed")
    if pristine_receipt != json.loads((HERE / "generation.json").read_text()):
        raise ValueError("pristine receipt changed")
    inputs = {p: (stage / p).read_text() for p in pristine_receipt["source_sha256"]}
    g.RULES = stage_rules(g.RULES)  # Fresh private module instance; no file edit.
    output, rewrites = g.transform(inputs)
    preserved = {}
    for path, old in inputs.items():
        tail = appended_helpers(old)
        if tail is not None:
            if appended_helpers(output[path]) != tail:
                raise ValueError(f"existing generated helpers changed: {path}")
            preserved[path] = digest(tail.encode())
    if g.PREFIX + "game/books.rs" not in preserved or "pluto_current_book" not in inputs[g.PREFIX + "game/books.rs"]:
        raise ValueError("reviewed current-book module absent")
    patch = g.patch_text(inputs, output)
    if g.apply_in_memory(patch, inputs) != output:
        raise ValueError("independent stage delta application differs")
    if sealed_files(stage, seal_path) != before:
        raise ValueError("stage changed during inspection")
    receipt = {
        "status": "SOURCE ONLY: separate unapplied v15-stage delta; no metadata, host typecheck, package proof or UI execution",
        "official_horizon": pristine_receipt["official_horizon"],
        "official_mixins": pristine_receipt["official_mixins"],
        "ordinary_v15_runtime": V15_RUNTIME,
        "stage_seal_sha256": SEAL_SHA,
        "stage_file_count": len(before),
        "pristine_patch_sha256": pristine_receipt["patch_sha256"],
        "pristine_generator_sha256": GENERATOR_SHA,
        "helper_sha256": HELPER_SHA,
        "input_sha256": {p: digest(s.encode()) for p, s in inputs.items()},
        "output_sha256": {p: digest(s.encode()) for p, s in output.items()},
        "unchanged_appended_helper_tails": preserved,
        "patch_sha256": digest(patch.encode()),
        "rewrites": rewrites,
        "changed_files": [p for p in output if output[p] != inputs.get(p)],
        "stage_unchanged_before_after": True,
        "ordinary_guards_unchanged": True,
    }
    return patch, receipt


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--stage", required=True, type=Path)
    parser.add_argument("--seal", required=True, type=Path)
    parser.add_argument("--official", required=True, type=Path)
    parser.add_argument("--write", action="store_true", help="write only experiment patch/receipt")
    args = parser.parse_args()
    patch, receipt = compose(args.stage, args.seal, args.official)
    if args.write:
        (HERE / "v15-composition.patch").write_text(patch)
        (HERE / "v15-composition.json").write_text(json.dumps(receipt, indent=2) + "\n")
    elif patch != (HERE / "v15-composition.patch").read_text() or receipt != json.loads((HERE / "v15-composition.json").read_text()):
        raise SystemExit("stored stage-composition artifacts differ")
    print(json.dumps({"status": "source-only in-memory PASS", "stage_files_unchanged": receipt["stage_file_count"],
                      "changed_files": len(receipt["changed_files"]), "patch_sha256": receipt["patch_sha256"]}))
