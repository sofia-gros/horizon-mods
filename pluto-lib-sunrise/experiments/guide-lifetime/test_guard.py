import copy
import importlib.util
import json
from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("guard", ROOT/"check_plan.py")
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)

class GuardTests(unittest.TestCase):
    def setUp(self):
        self.meta = json.loads((ROOT/"source.json").read_text())
        self.plan = {
            "inputs": self.meta["official_sources"],
            "injections": [{**h, "mod_id": guard.MOD} for h in self.meta["hooks"]]
                + self.meta["allowed_disjoint_clear_hooks"],
            "states": [{**self.meta["state"], "mod_id": guard.MOD}],
            "modules": [{**m, "mod_id": guard.MOD} for m in self.meta["modules"]] + self.meta["allowed_disjoint_modules"],
        }
    def rejected(self):
        with self.assertRaises(ValueError): guard.check(self.plan, self.meta)
    def test_support_modules_have_no_file_attributes_or_inner_docs(self):
        for name in ["state.rs", "native.rs"]:
            source = (ROOT/"mods/pluto_guide_lifetime_experiment"/name).read_text()
            self.assertFalse(any(line.lstrip().startswith(("//!", "#!")) for line in source.splitlines()), name)
    def test_accepts_reviewed_contract(self):
        guard.check(self.plan, self.meta)
    def test_accepts_omitted_default_visibility(self):
        self.plan["modules"][1] = dict(self.plan["modules"][1]); self.plan["modules"][1].pop("visibility"); guard.check(self.plan, self.meta)
    def test_rejects_alias_foreign_hook(self):
        hook = copy.deepcopy(self.plan["injections"][1]); hook["mod_id"] = "other"; hook["target"] = "alias"; self.plan["injections"].append(hook); self.rejected()
    def test_rejects_cancelled_invalidation(self):
        self.plan["injections"][0]["cancellable"] = True; self.rejected()
    def test_rejects_unreviewed_priority(self):
        self.plan["injections"][0]["priority"] = 1; self.rejected()
    def test_rejects_helper_replacement(self):
        self.plan["injections"][0]["helper_source"] = "fn nope() {}"; self.rejected()
    def test_rejects_state_constructor_drift(self):
        self.plan["states"][0]["constructors"] = []; self.rejected()
    def test_rejects_module_placement(self):
        self.plan["modules"][0]["file"] = "other.rs"; self.rejected()
    def test_rejects_source_drift(self):
        self.plan["inputs"] = dict(self.plan["inputs"]); self.plan["inputs"][next(iter(self.plan["inputs"]))] = "different"; self.rejected()
    def test_rejects_changed_disjoint_clear_state_module(self):
        self.plan["modules"][-1] = copy.deepcopy(self.plan["modules"][-1]); self.plan["modules"][-1]["source"] += "\n// changed"; self.rejected()
    def test_rejects_changed_disjoint_clear(self):
        self.plan["injections"][-1] = copy.deepcopy(self.plan["injections"][-1]); self.plan["injections"][-1]["cancellable"] = True; self.rejected()

if __name__ == "__main__": unittest.main()
