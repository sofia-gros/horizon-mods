#!/usr/bin/env python3
"""Small integrity/shape check only; does not invoke Rust, metadata or a host."""
import hashlib
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def main():
    inputs = json.loads((HERE / "inputs.json").read_text())
    for path, expected in inputs["source_sha256"].items():
        actual = ROOT / path
        if actual.is_symlink() or not actual.is_file():
            raise ValueError(f"not a regular source: {path}")
        if hashlib.sha256(actual.read_bytes()).hexdigest() != expected:
            raise ValueError(f"source drift: {path}")
    fixture = (HERE / "fixture.rs").read_text()
    names = re.findall(r"#\[test\]\s*fn (\w+)\(", fixture)
    if names != inputs["ownership_tests"]:
        raise ValueError("exact ten test names/order changed")
    if 'include!("../guide-response-phase/native-patch/retained_book.rs");' not in fixture:
        raise ValueError("fixture no longer reuses exact original storage")
    if 'include!("owned.rs");' not in fixture or 'book-view.rs' in fixture:
        raise ValueError("fixture scope changed")
    owned = (HERE / "owned.rs").read_text()
    if "static " in owned or "unsafe" in owned or "impl Clone" in owned or "impl Copy" in owned:
        raise ValueError("unexpected allocator/unsafe/clone capability")
    if re.search(r"#\[derive[^\]]*\]\s*pub\(super\) struct OwnedOpening", owned):
        raise ValueError("owner derivation requires renewed review")
    if not inputs["native_type_candidate_unrun"] or not inputs["rust_tests_unrun"]:
        raise ValueError("source candidate cannot claim execution")
    print(json.dumps({"source_files": len(inputs["source_sha256"]), "ownership_tests": len(names),
                      "rust_executed": False, "native_type_executed": False}))


if __name__ == "__main__":
    main()
