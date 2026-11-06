"""Excluded latest-target source experiment. Never edits an official tree.

The full --official gate must enumerate every current game source before any
future typing runner may use this delta. A partial download cannot pass it.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import subprocess

HERE = Path(__file__).resolve().parent
MOD = HERE.parents[2]
PREFIX = "crates/horizon-client/src/"
HELPER = PREFIX + "game/retained_book.rs"
PIN = "f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd"


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def config():
    pins = json.loads((HERE / "inputs.json").read_text())
    if pins["official_horizon"] != PIN:
        raise ValueError("Latest official pin changed")
    for name, digest in pins["mod_source_sha256"].items():
        path = MOD / name
        if path.is_symlink() or not path.is_file() or sha(path.read_bytes()) != digest:
            raise ValueError("Frozen Mod source changed: " + name)
    return pins


def legacy():
    path = HERE.parent / "guide-response-phase/native-patch/generate.py"
    spec = importlib.util.spec_from_file_location("guide_latest_pinned_legacy", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def combined_helper():
    return (HERE.parent / "guide-response-phase/native-patch/retained_book.rs").read_text() + "\n" + (
        HERE.parent / "guide-owned-opening/owned.rs").read_text() + "\n" + (
        HERE.parent / "guide-owned-opening/book-view.rs").read_text()


def inventory(sources):
    """Spelling inventory, never alias analysis or a native-mod sandbox."""
    book, transitions = [], []
    order = sorted(sources, key=lambda p: (p != PREFIX + "game.rs", p))
    for path in order:
        source = sources[path]
        accesses = [{"line": i, "text": line.strip()} for i, line in enumerate(source.splitlines(), 1)
                    if re.search(r"\bself\.book\b", line)]
        sites = [{"line": i, "text": line.strip()} for i, line in enumerate(source.splitlines(), 1)
                 if re.search(r"self\.player\s*=|self\.player\.(take|replace)\(|(?:replace|swap)\(&mut self\.player|self\.remote\s*=|self\.remote\.take\(", line)]
        if accesses:
            book.append({"file": path, "accesses": accesses})
        if sites:
            transitions.append({"file": path, "sites": sites})
    return book, transitions


def check_inventory(sources, pins):
    book, transitions = inventory(sources)
    if book != pins["book_field_inventory"] or transitions != pins["attachment_inventory"]:
        expected = {x["file"]: x for x in pins["book_field_inventory"]}
        actual = {x["file"]: x for x in book}
        paths = {p for p in expected.keys() | actual.keys() if expected.get(p) != actual.get(p)}
        expected = {x["file"]: x for x in pins["attachment_inventory"]}
        actual = {x["file"]: x for x in transitions}
        paths |= {p for p in expected.keys() | actual.keys() if expected.get(p) != actual.get(p)}
        raise ValueError("Latest direct-site inventory drift; review, do not auto-convert: " + ", ".join(sorted(paths)))


def build_known_inputs(sources, pins):
    """Read-only transform of 19 known sources, NOT a complete-tree gate."""
    if set(sources) != set(pins["source_sha256"]):
        raise ValueError("Known source file set differs")
    for path, expected in pins["source_sha256"].items():
        if sha(sources[path].encode()) != expected:
            raise ValueError("Latest known source drift: " + path)
    check_inventory(sources, pins)
    old = legacy()
    output, rewrites = old.transform(sources)
    output[HELPER] = combined_helper()
    patch = old.patch_text(sources, output)
    if old.apply_in_memory(patch, sources) != output:
        raise ValueError("Forward patch differs from bounded transform")
    existing = {p: text for p, text in output.items() if p != HELPER}
    reverse = old.patch_text(existing, sources)
    restored = old.apply_in_memory(reverse, existing)
    if restored != sources:
        raise ValueError("Reverse patch does not restore exact latest sources")
    receipt = {
        "official_horizon": PIN,
        "official_mixins": pins["official_mixins"],
        "official_lock_sha256": pins["official_lock_sha256"],
        "status": "Excluded unapplied source experiment; known source transform only.",
        "full_inventory_verified": False,
        "latest_typing_executed": False,
        "ui_or_lan_executed": False,
        "source_sha256": pins["source_sha256"],
        "output_sha256": {p: sha(s.encode()) for p, s in output.items()},
        "restored_sha256": {p: sha(s.encode()) for p, s in restored.items()},
        "patch_sha256": sha(patch.encode()),
        "reverse_patch_sha256": sha(reverse.encode()),
        "rewrites": rewrites,
        "changed_files": [p for p in output if output[p] != sources.get(p)],
    }
    if len(rewrites) != 22 or len(receipt["changed_files"]) != 11:
        raise ValueError("Reviewed transformation shape changed")
    return patch, receipt


def generate(official):
    pins = config()
    def git(*args):
        return subprocess.run(["git", "-C", str(official), *args], check=True,
                              capture_output=True, text=True, timeout=30).stdout.strip()
    if git("config", "--get", "remote.origin.url") not in {
        "https://github.com/horizon-rust/horizon", "https://github.com/horizon-rust/horizon.git"
    } or git("rev-parse", "HEAD") != PIN:
        raise ValueError("Not the exact authorized official source")
    if git("status", "--porcelain", "--untracked-files=all", "--ignored=matching"):
        raise ValueError("Official source is not clean")
    if sha((official / "Cargo.lock").read_bytes()) != pins["official_lock_sha256"]:
        raise ValueError("Latest official lock differs")
    old = legacy()
    paths = [official / (PREFIX + "game.rs"), *sorted((official / (PREFIX + "game")).rglob("*.rs"))]
    all_sources = {path.relative_to(official).as_posix(): old.regular(official, path.relative_to(official).as_posix()).decode() for path in paths}
    if HELPER in all_sources:
        raise ValueError("Official helper path already exists")
    # In particular, every new/changed file outside the 19 known inputs is
    # scanned. A new site is an error with exact paths, never an exclusion.
    check_inventory(all_sources, pins)
    sources = {p: all_sources[p] for p in pins["source_sha256"]}
    patch, receipt = build_known_inputs(sources, pins)
    if patch != (HERE / "prototype.patch").read_text():
        raise ValueError("Frozen latest prototype differs")
    if receipt != json.loads((HERE / "generation.json").read_text()):
        raise ValueError("Frozen partial-source receipt differs")
    for path, text in all_sources.items():
        if old.regular(official, path).decode() != text:
            raise ValueError("Official source changed while reading: " + path)
    # A future outer runner must also seal the entire Git/export/source trees.
    receipt["full_inventory_verified"] = True
    receipt["status"] = "Full current game direct-site inventory and exact source transform checked; not applied or typed."
    return receipt


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--official", required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(generate(args.official.resolve()), sort_keys=True))
