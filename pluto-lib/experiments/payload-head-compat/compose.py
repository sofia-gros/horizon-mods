"""Excluded, reversible source composition; no official tree or plan is changed."""
import hashlib
import json
from pathlib import Path
import tomllib

HERE = Path(__file__).resolve().parent
TC = "tconstruct/mixins/tconstruct/"
NATIVE = "pluto-lib/mixins/pluto_native_creative/"
IDS = ("broad", "haste")
RETURN = " -> std::ops::ControlFlow<Result<()>>"
IMPORT = "    use std::ops::ControlFlow::{Break, Continue};\n"


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def once(text, old, new):
    if text.count(old) != 1:
        raise ValueError("replacement is not unique")
    return text.replace(old, new, 1)


def table(text, kind, name):
    marker = f'[[{kind}]]\nid = "{name}"\n'
    if text.count(marker) != 1:
        raise ValueError("manifest table is not unique")
    start = text.index(marker)
    end = text.find("\n[[", start + len(marker))
    return text[start:] if end == -1 else text[start:end]


def helper(old):
    if old.count("return Continue(());") != 3:
        raise ValueError("old early-return structure differs")
    new = once(old, RETURN + " {\n", " {\n")
    new = once(new, IMPORT, "")
    new = new.replace("return Continue(());", "return;")
    new = once(new, "    Break(Ok(()))\n", "")
    header = new.split("\n", 1)[0] + "\n"
    if not header.startswith("fn tconstruct_") or not header.endswith(" {\n"):
        raise ValueError("function header differs")
    restored = once(new, header, header[:-3] + RETURN + " {\n" + IMPORT)
    if restored.count("return;") != 3:
        raise ValueError("new early-return structure differs")
    restored = restored.replace("return;", "return Continue(());")
    if not restored.endswith("}\n"):
        raise ValueError("function ending differs")
    restored = restored[:-2] + "    Break(Ok(()))\n}\n"
    if restored != old:
        raise ValueError("helper inverse differs")
    return new


def manifest(text):
    before = text
    edits = []
    for name in IDS:
        old = table(text, "injection", name + "-wire-receive")
        new = once(old, "cancellable = true", "cancellable = false")
        text = once(text, old, new)
        edits.append((old, new))
    restored = text
    for old, new in reversed(edits):
        restored = once(restored, new, old)
    if restored != before:
        raise ValueError("manifest inverse differs")
    a = tomllib.loads(before); b = tomllib.loads(text)
    for section in ("state", "crate", "module"):
        if a.get(section) != b.get(section):
            raise ValueError("unrelated manifest section changed")
    for entry in b["injection"]:
        if entry["id"] in {n + "-wire-receive" for n in IDS}:
            entry["cancellable"] = True
    if a != b:
        raise ValueError("other manifest semantics changed")
    return text


def render(original, guide):
    """Compose two noncancel HEADs over already reviewed Guide postimages.

    The supplied original manifest is an exact pinned input. An unrelated owner
    must rebase explicitly; this function never installs a whole old manifest.
    """
    meta = json.loads((HERE / "inputs.json").read_text(encoding="utf-8"))
    for p, digest in meta["original_sha256"].items():
        if sha(original[p].encode()) != digest:
            raise ValueError("original source drift: " + p)
    if set(guide) != set(meta["guide_postimage_sha256"]):
        raise ValueError("Guide output set differs")
    for p, digest in meta["guide_postimage_sha256"].items():
        if sha(guide[p].encode()) != digest:
            raise ValueError("Guide output drift: " + p)
    result = dict(guide)
    for name in IDS:
        path = TC + "source-" + name + "-wire-receive.rs"
        result[path] = helper(original[path])
    result[TC + "horizon-mixin.toml"] = manifest(guide[TC + "horizon-mixin.toml"])
    return result
