"""In-memory exact source extraction for a future native Text differential.

No Cargo manifest, fake Game, metadata invocation, compiler or execution.
"""
from pathlib import Path
import hashlib
import json
import re

HERE = Path(__file__).resolve().parent
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def exact(raw, digest):
    if not isinstance(raw, bytes) or sha(raw) != digest:
        raise ValueError("source identity mismatch")
    return raw.decode()


def unique_slice(raw, begin, end):
    if raw.count(begin) != 1 or raw.count(end) != 1:
        raise ValueError("nonunique source boundary")
    start = raw.index(begin); finish = raw.index(end, start)
    return raw[start:finish]


def substitute(raw, replacements):
    result = raw
    for old, new in replacements:
        if result.count(old) != 1 or new in result:
            raise ValueError("nonunique fixture substitution")
        result = result.replace(old, new, 1)
    inverse = result
    for old, new in reversed(replacements):
        if inverse.count(new) != 1:
            raise ValueError("nonunique fixture inverse")
        inverse = inverse.replace(new, old, 1)
    if inverse != raw:
        raise ValueError("fixture inverse mismatch")
    return result


def compose(raw_inputs):
    config = json.loads((HERE / "wire-inputs.json").read_text())
    if set(raw_inputs) != set(config["source_sha256"]):
        raise ValueError("source set mismatch")
    s = {name: exact(raw, config["source_sha256"][name]) for name, raw in raw_inputs.items()}
    legacy = unique_slice(s["old_response"], "    let Some(decoded)", "    Break(Ok(()))\n}")
    old = substitute(legacy, [("super::books::tconstruct_materials_book_wire", "super::tconstruct_materials_book_wire"),
        ("&message.data", "wire_bytes"), ("return Break(Ok(()));", "return;")])
    update = unique_slice(s["consumer"], "fn update(", "pub(in crate::game) fn ensure_registered")
    update = substitute(update, [("fn update(", "pub(super) fn update(")])
    view = unique_slice(s["bridge"], "pub(super) struct RetainedView", "#[cfg(test)]")
    if any(x in view + update for x in ["Game<'", "game.book", "request_sequence"]):
        raise ValueError("Game adapter leaked into value fixture")
    files = {
        "lib.rs": '''extern crate self as horizon_client;
pub mod text { pub use horizon_client_core::text::*; }
use horizon_client_core::text::Text;
use pluto_inventory_api::client_payload::Payload;
mod tconstruct_materials_book_identity;
mod tconstruct_materials_book_wire;
mod bridge;
mod candidate;
mod old_policy;
use old_policy::apply as old_apply;
use bridge::apply as candidate_apply;
#[cfg(test)] mod regression_tests;
''',
        "old_policy.rs": "use horizon_client_core::text::Text;\npub(super) fn apply(sequence: u64, pages: &mut Vec<Text>, page: &mut usize, wire_bytes: &[u8]) {\n" + old + "}\n",
        "candidate.rs": "use super::{Payload, bridge::RetainedView};\n" + update,
        "bridge.rs": "use horizon_client_core::text::Text;\n" + view + '''
// Fixture entry only: no Game or native screen admission is synthesized.
pub(super) fn apply(sequence: u64, pages: &mut Vec<Text>, page: &mut usize, data: &[u8]) {
    let mut view = RetainedView { pages, page, attempted: false };
    super::candidate::update(&mut view, sequence, super::Payload { channel: "tconstruct:materials_book", bytes: data });
}
''',
        "tconstruct_materials_book_identity.rs": s["identity"],
        "tconstruct_materials_book_wire.rs": s["wire"],
        "regression_tests.rs": s["tests"],
    }
    names = {}
    for file, module in [("regression_tests.rs", "regression_tests"), ("tconstruct_materials_book_wire.rs", "tconstruct_materials_book_wire::tests"), ("tconstruct_materials_book_identity.rs", "tconstruct_materials_book_identity::tests")]:
        names[module] = sorted(module + "::" + name for name in re.findall(r"#\[test\]\s*fn (\w+)\(", files[file]))
    if names != config["expected_tests"]:
        raise ValueError("test inventory changed")
    return {name: text.encode() for name, text in files.items()}, {
        "source_composed_only": True, "native_executed": False,
        "extracted_original_response_sha256": sha(legacy.encode()),
        "extracted_candidate_update_sha256": sha(unique_slice(s["consumer"], "fn update(", "pub(in crate::game) fn ensure_registered").encode()),
        "exact_retained_view_sha256": sha(view.encode()),
        "files": {name: sha(text.encode()) for name, text in files.items()},
        "expected_tests": names,
        "scope": "Actual native Text/legacy codec/retained-value helper only when compiled. No Game, channel dispatch, actor, observer, UI or network proof.",
    }
