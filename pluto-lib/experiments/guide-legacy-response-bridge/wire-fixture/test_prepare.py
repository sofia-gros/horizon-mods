"""Synthetic filesystem/metadata admission only; no Cargo or native execution."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import prepare as p


class PrepareTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.root = Path(self.temp.name)
        self.host = self.root / "official"; self.api = self.root / "api"; self.legacy = self.root / "legacy"; self.out = self.root / "out"
        for root in [self.host, self.api, self.legacy]: root.mkdir()
        self.write(self.host, "Cargo.toml", '[patch.crates-io]\nwgpu-hal={path="third_party/wgpu-hal"}\nnaga={path="third_party/naga"}\nlibmimalloc-sys={path="third_party/libmimalloc-sys"}\n')
        self.write(self.host, "Cargo.lock", 'version=4\n[[package]]\nname="horizon-client-core"\nversion="0.1.0"\n')
        self.write(self.host, "crates/horizon-client-core/Cargo.toml", '[package]\nname="horizon-client-core"\nversion="0.1.0"\n')
        self.write(self.host, "crates/horizon-client-core/src/text.rs", "// synthetic source admission input\n")
        self.write(self.api, "Cargo.toml", '[package]\nname="pluto-inventory-api"\nversion="0.1.0"\n')
        self.write(self.api, "src/lib.rs", "// synthetic API admission input\n")
        self.value = {"official_git_blobs": p.files(self.host), "api_git_blobs": p.files(self.api)}
        self.proof = {"source_composed_only": True, "native_executed": False}
        self.config = patch.object(p, "config", return_value=self.value); self.config.start()
        self.render = patch.object(p, "rendered", return_value=({"lib.rs": b"// synthetic fixture wiring\n"}, self.proof)); self.render.start()
        p.prepare(self.host, self.api, self.legacy, self.out)
        self.adapt_lock()
        self.metadata = {"version": 1, "workspace_root": str(self.out), "workspace_members": ["fixture"], "workspace_default_members": ["fixture"], "packages": [
            {"id": "fixture", "name": p.PACKAGE, "version": "0.0.0", "source": None, "manifest_path": str(self.out / "Cargo.toml")},
            {"id": "api", "name": "pluto-inventory-api", "version": "0.1.0", "source": None, "manifest_path": str(self.out / "inventory-api/Cargo.toml")},
            {"id": "core", "name": p.NATIVE, "version": "0.1.0", "source": None, "manifest_path": str(self.host / "crates/horizon-client-core/Cargo.toml")},
        ]}

    def tearDown(self): self.render.stop(); self.config.stop(); self.temp.cleanup()
    def write(self, root, name, data):
        target = root / name; target.parent.mkdir(parents=True, exist_ok=True); target.write_text(data)
    def adapt_lock(self):
        with (self.out / "Cargo.lock").open("a") as f:
            f.write('[[package]]\nname="' + p.PACKAGE + '"\nversion="0.0.0"\n[[package]]\nname="pluto-inventory-api"\nversion="0.1.0"\n')
    def reject(self, mutate):
        meta = copy.deepcopy(self.metadata); mutate(meta)
        with self.assertRaises(ValueError): p.verify(self.out, meta)

    def test_exact_synthetic_admission(self):
        self.assertEqual(p.verify(self.out, self.metadata)["official_revision"], p.HORIZON)
    def test_only_root_default_member_and_excluded_api(self):
        text = (self.out / "Cargo.toml").read_text()
        self.assertIn('exclude=["inventory-api"]', text); self.assertIn('default-members=["."]', text)
        self.assertIn('log="=0.4.34"', text); self.assertIn('serde_json="=1.0.151"', text)
        self.assertNotIn("horizon-client-render", text)
    def test_reject_extra_members(self): self.reject(lambda m: m["workspace_members"].append("api"))
    def test_reject_extra_default_members(self): self.reject(lambda m: m["workspace_default_members"].append("api"))
    def test_reject_wrong_workspace_root(self): self.reject(lambda m: m.update(workspace_root=str(self.api)))
    def test_reject_wrong_api_owner(self): self.reject(lambda m: m["packages"][1].update(manifest_path=str(self.api / "Cargo.toml")))
    def test_reject_wrong_core_owner(self): self.reject(lambda m: m["packages"][2].update(manifest_path=str(self.host / "other/Cargo.toml")))
    def test_reject_unknown_local_dependency(self):
        with (self.out / "Cargo.lock").open("a") as f: f.write('[[package]]\nname="unknown"\nversion="1.0.0"\n')
        self.reject(lambda m: m["packages"].append({"id":"unknown", "name":"unknown", "version":"1.0.0", "source":None, "manifest_path":str(self.root / "unknown/Cargo.toml")}))
    def test_reject_registry_substitution(self): self.reject(lambda m: m["packages"][2].update(source="registry+https://example.invalid"))
    def test_reject_duplicate_metadata_id(self): self.reject(lambda m: m["packages"].append(dict(m["packages"][1])))
    def test_reject_duplicate_api_with_distinct_metadata_id(self):
        def duplicate(meta):
            package = dict(meta["packages"][1]); package["id"] = "second-api-identity"
            meta["packages"].append(package)
        self.reject(duplicate)
    def test_reject_duplicate_lock_identity(self):
        with (self.out / "Cargo.lock").open("a") as f: f.write('[[package]]\nname="horizon-client-core"\nversion="0.1.0"\n')
        self.reject(lambda m: None)
    def test_reject_extra_or_modified_output(self):
        for name in ["lib.rs", "extra.txt"]:
            target = self.out / name; previous = target.read_bytes() if target.exists() else None; target.write_text("changed")
            with self.assertRaises(ValueError): p.verify(self.out, self.metadata)
            if previous is None: target.unlink()
            else: target.write_bytes(previous)
    def test_reject_extra_or_modified_api(self):
        self.write(self.api, "src/extra.rs", "new")
        with self.assertRaises(ValueError): p.verify(self.out, self.metadata)
    def test_reject_symlink_in_output(self):
        (self.out / "link").symlink_to(self.host / "Cargo.toml")
        with self.assertRaises(ValueError): p.verify(self.out, self.metadata)
    def test_reject_preexisting_output_before_writes(self):
        before = p.files(self.out)
        with self.assertRaises(ValueError): p.prepare(self.host, self.api, self.legacy, self.out)
        self.assertEqual(before, p.files(self.out))
    def test_reject_overlapping_output_before_writes(self):
        with self.assertRaises(ValueError): p.prepare(self.host, self.api, self.legacy, self.host / "new-output")
        self.assertFalse((self.host / "new-output").exists())
    def test_official_source_and_seed_lock_drift_rejected(self):
        for name in ["Cargo.lock", "crates/horizon-client-core/src/text.rs"]:
            target = self.host / name; old = target.read_bytes(); target.write_bytes(old + b"\n")
            with self.assertRaises(ValueError): p.verify(self.out, self.metadata)
            target.write_bytes(old)
    def test_bad_relative_paths_rejected(self):
        for path in ["", "../x", "/x", "a/../b", "a\\b", "a//b"]:
            with self.assertRaises(ValueError): p.safe_name(path)
    def test_receipt_content_is_rederived_not_trusted(self):
        path = self.out / "fixture-sources.json"; value = json.loads(path.read_text()); value["sources_sha256"]["lib.rs"] = "0" * 64; path.write_text(json.dumps(value))
        with self.assertRaises(ValueError): p.verify(self.out, self.metadata)
    def test_seed_lock_receipt_is_rederived(self):
        path = self.out / "fixture-sources.json"; value = json.loads(path.read_text()); value["seed_lock_sha256"] = "0" * 64; path.write_text(json.dumps(value))
        with self.assertRaises(ValueError): p.verify(self.out, self.metadata)


if __name__ == "__main__": unittest.main()
