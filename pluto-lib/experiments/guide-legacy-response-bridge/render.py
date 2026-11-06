"""Render an EXCLUDED proposal from exact inputs; never patch a host or mod tree.

This intentionally does not update the installed payload guard metadata. A
later actual full plan must establish the new 41f node/bindings/fingerprints.
"""
import argparse
import hashlib
import json
from pathlib import Path
import tomllib

HERE = Path(__file__).resolve().parent
NATIVE = "pluto-lib/mixins/pluto_native_creative/"
CONSUMER = "tconstruct/mixins/tconstruct/"
OPEN_BEFORE = "    if remote || game.lan_acting.is_none() {\n        let path = game.options.resources.clone();"
OPEN_AFTER = "    if remote || game.lan_acting.is_none() {\n        if !super::tconstruct_materials_book_response::ensure_registered(game) { return false; }\n        let path = game.options.resources.clone();"
OLD_BINDING = '{ name = "game", ty = "&Game<\'_>", value = "&*self" }'
NEW_BINDING = '{ name = "game", ty = "&mut Game<\'_>", value = "&mut *self" }'
PLUTO_MODULE = '''\n[[module]]
id = "client-payload-updates"
file = "crates/horizon-client/src/game.rs"
name = "pluto_client_payload_updates"
source = "client-payload-updates.rs"
'''
CONSUMER_MODULE = '''\n[[module]]
id = "materials-book-response-update"
file = "crates/horizon-client/src/game/books.rs"
name = "tconstruct_materials_book_response"
source = "source-materials-book-response-update.rs"
'''


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def once(text, old, new):
    if text.count(old) != 1:
        raise ValueError("exact replacement count changed")
    return text.replace(old, new, 1)


def table(text, kind, name):
    start = text.index(f'[[{kind}]]\nid = "{name}"\n')
    end = text.find("\n[[", start + 2)
    return text[start:] if end == -1 else text[start:end]


def inputs(root, manifest):
    meta = json.loads((HERE / "inputs.json").read_text())
    result = {}
    for path, expected in meta["input_sha256"].items():
        raw = (root / path).read_bytes()
        if sha(raw) != expected:
            raise ValueError(f"input drift: {path}")
        result[path] = raw.decode()
    raw = manifest.read_bytes()
    if sha(raw) != meta["integration_manifest_sha256"]:
        raise ValueError("Integration consumer manifest drift")
    result[CONSUMER + "horizon-mixin.toml"] = raw.decode()
    for path, expected in meta["candidate_sha256"].items():
        if sha((HERE / path).read_bytes()) != expected:
            raise ValueError(f"candidate source drift: {path}")
    return result


def render(original):
    pmanifest = original[NATIVE + "horizon-mixin.toml"]
    old = table(pmanifest, "injection", "client-payload-observed")
    new = once(old, OLD_BINDING, NEW_BINDING)
    pmanifest = once(pmanifest, old, new) + PLUTO_MODULE
    tmanifest = original[CONSUMER + "horizon-mixin.toml"]
    response = table(tmanifest, "injection", "materials-book-response")
    bootstrap = once(response, "cancellable = true", "cancellable = false")
    tmanifest = once(tmanifest, response, bootstrap) + CONSUMER_MODULE
    book = once(original[CONSUMER + "source-materials-book.rs"], OPEN_BEFORE, OPEN_AFTER)
    result = {
        NATIVE + "horizon-mixin.toml": pmanifest,
        NATIVE + "client-payload-observed.rs": (HERE / "after-native-packet.rs").read_text(),
        NATIVE + "client-payload-updates.rs": (HERE / "native-bridge.rs").read_text(),
        CONSUMER + "horizon-mixin.toml": tmanifest,
        CONSUMER + "source-materials-book-response.rs": (HERE / "bootstrap.rs").read_text(),
        CONSUMER + "source-materials-book-response-update.rs": (HERE / "consumer.rs").read_text(),
        CONSUMER + "source-materials-book.rs": book,
    }
    # Exact inverse checks, not just accepted TOML or selected-line comparison.
    assert once(pmanifest.removesuffix(PLUTO_MODULE), new, old) == original[NATIVE + "horizon-mixin.toml"]
    assert once(tmanifest.removesuffix(CONSUMER_MODULE), bootstrap, response) == original[CONSUMER + "horizon-mixin.toml"]
    assert once(book, OPEN_AFTER, OPEN_BEFORE) == original[CONSUMER + "source-materials-book.rs"]
    for path in (NATIVE + "horizon-mixin.toml", CONSUMER + "horizon-mixin.toml"):
        before = tomllib.loads(original[path]); after = tomllib.loads(result[path])
        assert before.get("state", []) == after.get("state", [])
        assert before.get("crate", []) == after.get("crate", [])
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--inputs", type=Path, required=True)
    parser.add_argument("--tc-manifest", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    rendered = render(inputs(args.inputs, args.tc_manifest))
    args.out.mkdir(exist_ok=False)
    for path, text in rendered.items():
        target = args.out / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)
    (args.out / "EXCLUDED-NOT-AN-ACCEPTED-PACKAGE.txt").write_text(
        "Source proposal only. Installed metadata is unchanged and rejects this helper.\n")
    print(json.dumps({"excluded": True, "outputs": {p: sha(t.encode()) for p, t in rendered.items()}}, indent=2))
