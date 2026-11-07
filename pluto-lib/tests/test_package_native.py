"""Checks the native archive's source identity and public packaging contract."""
import importlib.util
import shutil
from pathlib import Path
import tempfile
import tomllib
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("package_native", ROOT / "tools/package_native.py")
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


class NativePackageTests(unittest.TestCase):
    def test_support_modules_have_no_file_attributes_or_inner_docs(self):
        # Mixins parses these as support modules and rejects file attributes,
        # including inner documentation after an ordinary license header.
        # This source convention catches that class before an official plan.
        for component in package.COMPONENTS:
            root = ROOT / "mixins" / component
            manifest = tomllib.loads((root / "horizon-mixin.toml").read_text())
            for module in manifest.get("module", []):
                with self.subTest(component=component, module=module["id"]):
                    self.assertNotRegex((root / module["source"]).read_text(), r"(?m)^(?://!|/\*!|#!)")

    def test_guide_lifetime_experiment_is_not_installed(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "library.hnmod"
            package.build(path)
            with zipfile.ZipFile(path) as archive:
                for name in archive.namelist():
                    self.assertNotIn("guide-lifetime", name)
                    self.assertNotIn("pluto_guide_lifetime_experiment", name)
                manifest = archive.read("mixins/pluto_native_creative/horizon-mixin.toml")
                self.assertNotIn(b"guide_lifetime", manifest)
    def test_includes_new_nested_api_modules_without_extra_registry_copies(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "pluto-lib"
            shutil.copytree(ROOT / "mixins", root / "mixins")
            shutil.copy2(ROOT / "horizon-native.json", root / "horizon-native.json")
            component = root / "mixins/pluto_native_creative"
            for crate in package.CRATE_DIRECTORIES:
                source = component / crate / "src/new_service/nested.rs"
                source.parent.mkdir()
                source.write_text("pub const CONTRACT_VERSION: u32 = 1;\n", newline="\n")
                (component / crate / "src/notes.txt").write_text("not Rust source")
                (component / crate / "target").mkdir()
                (component / crate / "target/generated.rs").write_text("not API source")
            previous = package.ROOT
            try:
                package.ROOT = root
                output = Path(temp) / "extended.hnmod"
                package.build(output)
                with zipfile.ZipFile(output) as archive:
                    names = archive.namelist()
                    for crate in package.CRATE_DIRECTORIES:
                        path = f"mixins/pluto_native_creative/pluto_native_creative/{crate}/src/new_service/nested.rs"
                        self.assertEqual(names.count(path), 1)
                        self.assertEqual(archive.read(path), b"pub const CONTRACT_VERSION: u32 = 1;\n")
                    self.assertFalse(any(name.endswith("notes.txt") or "/target/" in name for name in names))
            finally:
                package.ROOT = previous

    def test_rejects_symlinked_api_files_and_directories(self):
        for directory in (False, True):
            with self.subTest(directory=directory), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                crate = root / "api"
                (crate / "src").mkdir(parents=True)
                (crate / "Cargo.toml").write_text('[package]\nname = "example"\nversion = "0.1.0"\n')
                (crate / "src/lib.rs").write_text("pub mod linked;\n")
                target = root / "outside"
                if directory:
                    target.mkdir()
                    (target / "mod.rs").write_text("pub const VALUE: u32 = 1;\n")
                else:
                    target.write_text("pub const VALUE: u32 = 1;\n")
                (crate / "src" / ("linked" if directory else "linked.rs")).symlink_to(target, target_is_directory=directory)
                with self.assertRaisesRegex(ValueError, "regular source path"):
                    package._crate_sources(root, "api")

    def test_combines_components_without_copying_demos_or_registries(self):
        with tempfile.TemporaryDirectory() as temp:
            first, second = Path(temp) / "first.hnmod", Path(temp) / "second.hnmod"
            package.build(first)
            package.build(second)
            self.assertEqual(first.read_bytes(), second.read_bytes())
            with zipfile.ZipFile(first) as archive:
                names = archive.namelist()
                prefix = "mixins/pluto_native_creative/"
                self.assertEqual(sum(name.endswith("horizon-mixin.toml") for name in names), 1)
                self.assertFalse(any("demo" in name or "pluto_creative/" in name for name in names))
                combined = tomllib.loads(archive.read(prefix + "horizon-mixin.toml").decode())
                self.assertEqual(len(combined["crate"]), 2)
                self.assertEqual({entry["path"] for entry in combined["crate"]},
                                 {"pluto_native_creative/api", "pluto_native_creative/inventory-api"})
                self.assertIn(prefix + "pluto_native_creative/api/src/fluid.rs", names)
                self.assertIn(prefix + "pluto_native_creative/world-fluid.rs", names)
                self.assertFalse(any("/assets/" in name or "tconstruct" in name for name in names))
                item_sources = [name for name in names if name.endswith("/api/src/item.rs")]
                self.assertEqual(len(item_sources), 1)
                self.assertEqual(archive.read(item_sources[0]).count(b"static REGISTRY:"), 1)
                for section in ("crate", "module", "injection", "state"):
                    ids = [entry["id"] for entry in combined.get(section, [])]
                    self.assertEqual(len(ids), len(set(ids)), f"duplicate {section} IDs")
                self.assertIn(prefix + "pluto_native_creative/inventory-api/src/lib.rs", names)
                api = prefix + "pluto_native_creative/inventory-api/src/advancements.rs"
                self.assertEqual(archive.read(api), (ROOT / "mixins/pluto_native_creative/inventory-api/src/advancements.rs").read_bytes())
                self.assertNotIn(b"tconstruct", archive.read(api))
                self.assertNotIn(b"horizon_client", archive.read(api))
                for component in package.COMPONENTS:
                    root = ROOT / "mixins" / component
                    original = tomllib.loads((root / "horizon-mixin.toml").read_text())
                    for module in original.get("module", []):
                        entry = next(x for x in combined["module"] if x["id"] == f'{component}-{module["id"]}')
                        self.assertEqual(entry["name"], module["name"])
                        self.assertEqual(archive.read(prefix + entry["source"]), (root / module["source"]).read_bytes())
                    for injection in original.get("injection", []):
                        entry = next(x for x in combined["injection"] if x["id"] == f'{component}-{injection["id"]}')
                        self.assertEqual(entry["priority"], injection.get("priority", original["mod"].get("priority", 100)))
                        self.assertEqual(archive.read(prefix + entry["helper"]), (root / injection["helper"]).read_bytes())
                    for state in original.get("state", []):
                        state_id = state["id"] if component == "pluto_native_creative" else f'{component}-{state["id"]}'
                        matches = [x for x in combined.get("state", []) if x["id"] == state_id]
                        self.assertEqual(len(matches), 1)
                        expected = dict(state, id=state_id)
                        self.assertEqual(matches[0], expected)
                        self.assertTrue(matches[0].get("constructor"))
                        self.assertTrue(all(c.get("target") and c.get("selector") and c.get("require", 0) > 0
                                            for c in matches[0].get("constructor", [])))
                landing = next(x for x in combined["state"] if x["id"] == "landing-state")
                self.assertEqual(landing["target"], "horizon_client_render::chunk_map::ChunkMap")
                self.assertEqual(landing["constructor"], [{"target": "horizon_client_render::chunk_map::<ChunkMap as Default>::default", "selector": "Self", "require": 1}])
                module = next(x for x in combined["module"] if x["name"] == "pluto_landing_state")
                self.assertIn("pluto_landing_state::LandingState", landing["ty"])
                self.assertIn(prefix + module["source"], names)
                self.assertIn(prefix + "NOTICE.md", names)
            with self.assertRaises(FileExistsError):
                package.build(first)

    def test_tag_overlays_package_world_cache_and_native_boundaries(self):
        root = ROOT / "mixins/pluto_native_creative"
        manifest = tomllib.loads((root / "horizon-mixin.toml").read_text())
        states = {entry["id"]: entry for entry in manifest["state"]}
        self.assertEqual(states["advancement-tags"]["target"], "horizon_client_render::chunk_map::ChunkMap")
        self.assertNotIn("advancement-index-tags", states)
        self.assertTrue(all(entry["constructor"] for entry in states.values()))
        hooks = {entry["id"]: entry for entry in manifest["injection"]}
        self.assertIn("inventory_index", hooks["advancement-tag-index"]["target"])
        self.assertIn("::in_tag", hooks["advancement-tag-predicate"]["target"])
        index = (root / "advancement-tag-index.rs").read_text()
        self.assertIn("Arc::ptr_eq(built_tags, &snapshot)", index)
        self.assertIn("Arc::ptr_eq(built_tree, &tree)", index)
        self.assertNotIn("game.adv.inventory_index", index)
        self.assertIn("downcast_ref::<Cached>()", index)
        self.assertNotIn("last_inventory", index)
        self.assertNotIn("progress", index)
        self.assertIn("Continue(())", index)
        state = (root / "advancement-tag-state.rs").read_text()
        self.assertNotIn("pluto_inventory_api", state)
        names = ["advancement-tags.rs", "advancement-tag-index.rs", "advancement-tag-predicate.rs", "advancement-tag-state.rs", "advancement-tags-clear.rs", "inventory-api/src/advancements/tags.rs"]
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "tags.hnmod"
            package.build(path)
            with zipfile.ZipFile(path) as archive:
                prefix = "mixins/pluto_native_creative/pluto_native_creative/"
                for name in names:
                    expected = (root / name).read_bytes()
                    self.assertEqual(archive.read(prefix + name), expected)
                    self.assertNotIn(b"tconstruct", expected)

    def test_loaded_tree_projection_is_bundled_without_automatic_activation(self):
        root = ROOT / "mixins/pluto_native_creative"
        manifest = tomllib.loads((root / "horizon-mixin.toml").read_text())
        modules = {m["id"]: m for m in manifest["module"]}
        expected = {"advancement-projection": "crates/horizon-client/src/advancements.rs",
                    "advancement-projection-world": "crates/horizon-client/src/game.rs"}
        for name, target in expected.items():
            self.assertEqual(modules[name]["file"], target)
            source = (root / modules[name]["source"]).read_text()
            self.assertFalse(source.lstrip().startswith("//!"), "Mixins modules reject inner attributes/docs")
            self.assertNotIn("tconstruct", source)
            self.assertFalse(any(h["helper"] == modules[name]["source"] for h in manifest["injection"]))
        adapter = (root / "advancement-projection-world.rs").read_text()
        self.assertNotIn("advancement_tree()", adapter)
        self.assertNotIn("last_inventory =", adapter)
        self.assertNotIn("pending_tree.take()", adapter)
        self.assertNotIn("progress =", adapter)
        source = (root / "advancement-projection.rs").read_text()
        for index in ("index_recipes", "index_looking_at", "index_nbt", "index_player_facts"):
            self.assertIn("next." + index + "()", source)
        api = "inventory-api/src/advancements/projection.rs"
        self.assertNotIn("horizon_", (root / api).read_text())
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "projection.hnmod"
            package.build(path)
            with zipfile.ZipFile(path) as archive:
                prefix = "mixins/pluto_native_creative/pluto_native_creative/"
                for name in (api, "advancement-projection.rs", "advancement-projection-world.rs"):
                    self.assertEqual(archive.read(prefix + name), (root / name).read_bytes())

    def test_event_adapter_is_explicit_and_keeps_native_actor_pipeline(self):
        root = ROOT / "mixins/pluto_native_creative"
        manifest = tomllib.loads((root / "horizon-mixin.toml").read_text())
        module = next(x for x in manifest["module"] if x["id"] == "advancement-events")
        self.assertEqual(module["file"], "crates/horizon-client/src/game.rs")
        helper = (root / module["source"]).read_text()
        self.assertNotIn("tconstruct", helper)
        self.assertNotIn("static ", helper)
        self.assertLess(helper.index("game.remote.is_some()"), helper.index("game.load_advancements()"))
        self.assertLess(helper.index("game.no_player_here()"), helper.index("game.load_advancements()"))
        self.assertLess(helper.index("game.load_advancements()"), helper.index("game.player_entity_for(event.trigger)"))
        self.assertIn('adv::conditions(&conditions["player"], &context, world)', helper)
        self.assertIn("game.trigger(event.trigger", helper)
        self.assertFalse(any(x["helper"] == module["source"] for x in manifest["injection"]))
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "events.hnmod"
            package.build(path)
            with zipfile.ZipFile(path) as archive:
                prefix = "mixins/pluto_native_creative/pluto_native_creative/"
                api = "inventory-api/src/advancements/events.rs"
                self.assertEqual(archive.read(prefix + api), (root / api).read_bytes())
                self.assertEqual(archive.read(prefix + module["source"]), (root / module["source"]).read_bytes())

    def test_predicate_adapter_owns_host_boundary_and_packages_real_api(self):
        source = ROOT / "mixins/pluto_native_creative"
        manifest = tomllib.loads((source / "horizon-mixin.toml").read_text())
        hooks = [entry for entry in manifest["injection"] if entry["id"] == "advancement-item-predicate"]
        self.assertEqual(len(hooks), 1)
        self.assertEqual(hooks[0]["target"], "horizon_client_lib::advancements::item")
        helper = (source / hooks[0]["helper"]).read_text()
        gate = helper.index("if !api::has_item_predicates()")
        self.assertLess(helper.index("Continue(())", gate), helper.index("advancements::data"))
        self.assertIn("DispatchResult::Unsupported => Continue(())", helper)
        self.assertIn("DispatchResult::Conflict { .. } => Break(false)", helper)
        self.assertNotIn("register_item_predicate", helper)
        self.assertNotIn("tconstruct", helper)
        api = source / "inventory-api/src/advancements/predicates.rs"
        self.assertNotIn("horizon_", api.read_text())
        self.assertNotIn("tconstruct", api.read_text())
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "predicates.hnmod"
            package.build(path)
            with zipfile.ZipFile(path) as archive:
                prefix = "mixins/pluto_native_creative/pluto_native_creative/"
                self.assertEqual(archive.read(prefix + "inventory-api/src/advancements/predicates.rs"), api.read_bytes())
                self.assertEqual(archive.read(prefix + "advancement-item-predicate.rs"), (source / "advancement-item-predicate.rs").read_bytes())
                self.assertIn('serde_json = "1"', archive.read(prefix + "inventory-api/Cargo.toml").decode())

    def test_inventory_event_adapter_is_default_off_before_snapshot_work(self):
        source = ROOT / "mixins/pluto_native_creative"
        helper = (source / "inventory-advancements.rs").read_text()
        gate = helper.index("if !advancements::inventory_component_events_requested()")
        continuation = helper.index("std::ops::ControlFlow::Continue(())", gate)
        self.assertLess(continuation, helper.index("encode_components"))
        self.assertLess(continuation, helper.index("let slots:"))
        self.assertLess(continuation, helper.index("game.adv.last_inventory"))
        self.assertNotIn("request_inventory_component_events()", helper)
        api = (source / "inventory-api/src/advancements.rs").read_text()
        self.assertIn("AtomicBool::new(false)", api)
        self.assertEqual(api.count("static INVENTORY_COMPONENT_EVENTS:"), 1)
        self.assertNotIn("tconstruct", api)
        self.assertNotIn("tconstruct", helper)

    def test_combines_legacy_components_without_state(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "pluto-lib"
            shutil.copytree(ROOT / "mixins", root / "mixins")
            shutil.copy2(ROOT / "horizon-native.json", root / "horizon-native.json")
            manifest = root / "mixins/pluto_native_creative/horizon-mixin.toml"
            text = manifest.read_text()
            self.assertGreaterEqual(text.count('[[state]]'), 1)
            manifest.write_text(text.split('[[state]]', 1)[0])
            previous = package.ROOT
            try:
                package.ROOT = root
                output = Path(temp) / "legacy.hnmod"
                package.build(output)
                with zipfile.ZipFile(output) as archive:
                    prefix = "mixins/pluto_native_creative/"
                    combined = tomllib.loads(archive.read(prefix + "horizon-mixin.toml").decode())
                    self.assertNotIn("state", combined)
                    self.assertEqual(len(combined["crate"]), 2)
                    for section, key in (("module", "source"), ("injection", "helper")):
                        ids = [entry["id"] for entry in combined[section]]
                        self.assertEqual(len(ids), len(set(ids)))
                        for entry in combined[section]:
                            self.assertIn(prefix + entry[key], archive.namelist())
            finally:
                package.ROOT = previous


if __name__ == "__main__":
    unittest.main()
