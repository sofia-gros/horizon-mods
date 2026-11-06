#!/usr/bin/env python3
"""Generate/check an UNAPPLIED patch; never writes to the official workspace.

Only this experiment's patch/receipt are written with --write. There is no
apply/build/fetch mode. Source SHA and replacement counts fail closed.
"""
import argparse
import difflib
import hashlib
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
PREFIX = "crates/horizon-client/src/"
HOST_MODULE = PREFIX + "game/retained_book.rs"

# Exact reviewed source substitutions. Native fields, control-flow and packet
# routes remain; only slot storage/access and attachment instrumentation change.
RULES = {
    "game.rs": [
        ("mod books;\n", "mod books;\nmod retained_book;\n", 1),
        ("    book: Option<books::BookScreen>,", "    book: retained_book::RetainedBook<books::BookScreen>,", 1),
        ("            book: None,", "            book: retained_book::RetainedBook::default(),", 1),
    ],
    "game/books.rs": [
        ("self.book = Some(screen);", "self.book.set(Some(screen));", 1),
        ("self.book = None;", "self.book.set(None);", 2),
        ("match &self.book", "match self.book.as_ref()", 3),
        ("let Some(book) = &self.book", "let Some(book) = self.book.as_ref()", 1),
        ("matches!(self.book,", "matches!(self.book.as_ref(),", 1),
        ("self.book,\n", "self.book.as_ref(),\n", 3),
    ],
    "game/pad_menu.rs": [("matches!(self.book,", "matches!(self.book.as_ref(),", 2)],
    "game/ime.rs": [("matches!(self.book,", "matches!(self.book.as_ref(),", 1)],
    "game/native_hooks.rs": [("match &self.book", "match self.book.as_ref()", 1)],
    "game/native_machines.rs": [("self.book = book;", "self.book.set(book);", 1)],
    "game/remote_chat.rs": [("=> self.book = Some(*book),", "=> self.book.set(Some(*book)),", 1)],
    "game/lan_actions.rs": [("std::mem::replace(&mut self.book, host_book)", "self.book.replace(host_book)", 1)],
    "game/remote.rs": [
        ("self.book = None;", "self.book.set(None);", 1),
        ("        self.remote = Some(Remote {", "        self.book.invalidate_attachment();\n        self.remote = Some(Remote {", 1),
    ],
    "game/saving.rs": [],
}
PLAYER_WRITES = {"game.rs": 3, "game/remote.rs": 5}
REMOTE_TAKES = {"game.rs": 1, "game/remote.rs": 1, "game/saving.rs": 1}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def regular(root, relative):
    path = root / relative
    if not path.is_file() or path.is_symlink():
        raise ValueError(f"not a regular official source: {relative}")
    return path.read_bytes()


def read_inputs(official, pins):
    # Re-enumerate direct field paths so a newly added file/access cannot be
    # silently absent from the reviewed inventory. This remains a spelling
    # inventory, not a Rust alias analysis or an arbitrary-mod sandbox.
    files = [official / (PREFIX + "game.rs"), *sorted((official / (PREFIX + "game")).rglob("*.rs"))]
    book = []; transitions = []
    for path in files:
        relative = path.relative_to(official).as_posix()
        source = regular(official, relative).decode()
        accesses = [{"line": i, "text": line.strip()} for i, line in enumerate(source.splitlines(), 1)
                    if re.search(r"\bself\.book\b", line)]
        sites = [{"line": i, "text": line.strip()} for i, line in enumerate(source.splitlines(), 1)
                 if re.search(r"self\.player\s*=|self\.player\.(take|replace)\(|(?:replace|swap)\(&mut self\.player|self\.remote\s*=|self\.remote\.take\(", line)]
        if accesses: book.append({"file": relative, "accesses": accesses})
        if sites: transitions.append({"file": relative, "sites": sites})
    if book != pins["book_field_inventory"] or transitions != pins["attachment_inventory"]:
        raise ValueError("official direct-field inventory drift")
    result = {}
    for relative, expected in pins["source_sha256"].items():
        raw = regular(official, relative)
        if digest(raw) != expected:
            raise ValueError(f"official source drift: {relative}")
        result[relative] = raw.decode()
    return result


def transform(inputs):
    output = dict(inputs)
    counts = []
    for relative, rules in RULES.items():
        path = PREFIX + relative
        source = output[path]
        for before, after, expected in rules:
            actual = source.count(before)
            if actual != expected:
                raise ValueError(f"rewrite count drift: {path}: {before!r}: {actual} != {expected}")
            source = source.replace(before, after)
            counts.append({"file": path, "before": before, "after": after, "count": actual})
        if relative in PLAYER_WRITES:
            pattern = r"(?m)^( +)(self\.player = (?:None|Some\(player\));)$"
            source, count = re.subn(pattern, lambda m: m[1] + "self.book.invalidate_attachment();\n" + m[1] + m[2], source)
            if count != PLAYER_WRITES[relative]:
                raise ValueError(f"player transition count drift: {path}: {count}")
            counts.append({"file": path, "kind": "actual-player-assignment", "count": count})
        if relative in REMOTE_TAKES:
            pattern = r"(?m)^( +)(if let Some\(mut (?:remote|r)\) = self\.remote\.take\(\) \{)$"
            source, count = re.subn(pattern, lambda m: m[0] + "\n" + m[1] + "    self.book.invalidate_attachment();", source)
            if count != REMOTE_TAKES[relative]:
                raise ValueError(f"connection detach count drift: {path}: {count}")
            counts.append({"file": path, "kind": "actual-remote-take", "count": count})
        output[path] = source
    for path, source in output.items():
        if re.search(r"self\.book\s*=|&mut self\.book\b|&self\.book\b|self\.book,", source):
            raise ValueError(f"unconverted raw slot access: {path}")
    output[HOST_MODULE] = (HERE / "retained_book.rs").read_text()
    return output, counts


def patch_text(inputs, output):
    parts = []
    for path in sorted(output):
        old = inputs.get(path, "")
        new = output[path]
        if old != new:
            parts.extend(difflib.unified_diff(old.splitlines(keepends=True), new.splitlines(keepends=True),
                                            fromfile="a/" + path if path in inputs else "/dev/null",
                                            tofile="b/" + path, n=3))
    return "".join(parts)


def apply_in_memory(patch, inputs):
    """Independent bounded unified-hunk reader; no filesystem apply."""
    lines = patch.splitlines(keepends=True)
    result = dict(inputs)
    at = 0
    while at < len(lines):
        if not lines[at].startswith("--- ") or not lines[at + 1].startswith("+++ b/"):
            raise ValueError("invalid unified file header")
        old_name = lines[at][4:].strip()
        path = lines[at + 1][6:].strip()
        old = [] if old_name == "/dev/null" else inputs[path].splitlines(keepends=True)
        new = []
        cursor = 0
        at += 2
        while at < len(lines) and lines[at].startswith("@@ "):
            match = re.fullmatch(r"@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@\n", lines[at])
            if not match:
                raise ValueError("invalid hunk header")
            start = int(match[1]); expected_old = int(match[2] or 1); expected_new = int(match[4] or 1)
            begin = 0 if start == 0 else start - 1
            if begin < cursor: raise ValueError("overlapping hunk")
            new.extend(old[cursor:begin]); cursor = begin; seen_old = seen_new = 0; at += 1
            while at < len(lines) and not lines[at].startswith(("@@ ", "--- ")):
                line = lines[at]; marker, value = line[0], line[1:]
                if marker in " -":
                    if cursor >= len(old) or old[cursor] != value: raise ValueError("hunk context mismatch")
                    cursor += 1; seen_old += 1
                if marker in " +": new.append(value); seen_new += 1
                if marker not in " +-": raise ValueError("unsupported hunk marker")
                at += 1
            if (seen_old, seen_new) != (expected_old, expected_new): raise ValueError("hunk length mismatch")
        new.extend(old[cursor:]); result[path] = "".join(new)
    return result


def generate(official):
    pins = json.loads((HERE / "official-sources.json").read_text())
    inputs = read_inputs(official, pins)
    output, rewrites = transform(inputs)
    patch = patch_text(inputs, output)
    if apply_in_memory(patch, inputs) != output:
        raise ValueError("independent in-memory patch application differs")
    if read_inputs(official, pins) != inputs:
        raise ValueError("official sources changed during read-only generation")
    receipt = {"official_horizon": pins["official_horizon"], "official_mixins": pins["official_mixins"],
               "status": "Unapplied; exact hunk application verified only in memory. No Game/BookScreen typecheck or UI proof.",
               "source_sha256": pins["source_sha256"], "output_sha256": {p: digest(s.encode()) for p, s in output.items()},
               "patch_sha256": digest(patch.encode()), "rewrites": rewrites,
               "changed_files": [p for p in output if output[p] != inputs.get(p)],
               "official_source_unchanged": True}
    return patch, receipt


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--official", required=True, type=Path)
    parser.add_argument("--write", action="store_true", help="write only this MOD experiment's patch and receipt")
    args = parser.parse_args()
    patch, receipt = generate(args.official)
    if args.write:
        (HERE / "prototype.patch").write_text(patch)
        (HERE / "generation.json").write_text(json.dumps(receipt, indent=2) + "\n")
    else:
        if (HERE / "prototype.patch").read_text() != patch or json.loads((HERE / "generation.json").read_text()) != receipt:
            raise SystemExit("stored unapplied prototype differs")
    print(json.dumps({"changed_files": len(receipt["changed_files"]), "patch_sha256": receipt["patch_sha256"],
                      "in_memory_application": "PASS", "official_source_unchanged": True}))
