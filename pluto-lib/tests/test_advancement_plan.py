import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("advancement_plan", Path(__file__).resolve().parents[1] / "tools/check_advancement_plan.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def metadata():
    return [dict(target="game::inventory_index", injection_id="advancement-tag-index", function_fingerprint="index-fingerprint", priority=100,
                 file="src/advancements.rs", bindings=[dict(name="game", ty="&mut Game<'_>", value="self")], helper="index.rs"),
            dict(target="game::check_inventory_trigger", injection_id="inventory-advancement-components", function_fingerprint="event-fingerprint", priority=100,
                 file="src/advancements.rs", bindings=[dict(name="game", ty="&mut Game<'_>", value="self")], helper="event.rs")]


def plan(meta):
    return {"schema": 6, "injections": [dict(mod_id="pluto_native_creative", injection_id=m["injection_id"], target=m["target"],
        file=m["file"], kind="HEAD", priority=m["priority"], cancellable=True, function_fingerprint=m["function_fingerprint"], bindings=m["bindings"])
        for m in meta]}


class AdvancementPlanTests(unittest.TestCase):
    def test_exact_unified_and_component_ids_are_accepted(self):
        meta=metadata();candidate=plan(meta);MODULE.check_order(candidate,meta)
        for hook in candidate["injections"]: hook["injection_id"]="pluto_native_creative-"+hook["injection_id"]
        MODULE.check_order(candidate,meta)

    def test_every_competing_phase_is_rejected_on_both_replacement_targets(self):
        meta=metadata()
        for target in [m["target"] for m in meta]:
            for kind in ("HEAD","RETURN","OVERWRITE","BEFORE_CALL","AFTER_CALL","AFTER_ASSIGN","MODIFY_ARG","MODIFY_ARGS","MODIFY_VARIABLE","MODIFY_EXPRESSION_VALUE","MODIFY_CONSTANT","WRAP_OPERATION"):
                with self.subTest(target=target,kind=kind):
                    candidate=plan(meta);candidate["injections"].append(dict(mod_id="other",injection_id="extra",target=target,kind=kind,priority=200))
                    with self.assertRaisesRegex(ValueError,"bypassed"):MODULE.check_order(candidate,meta)

    def test_canonical_predicate_fallback_boundary_is_not_a_copied_body(self):
        meta=metadata();candidate=plan(meta)
        candidate["injections"].append(dict(mod_id="other",injection_id="item",target="horizon_client_lib::advancements::item",kind="HEAD",priority=100))
        MODULE.check_order(candidate,meta)

    def test_missing_duplicate_and_spoofed_suffix_ids_are_rejected(self):
        meta=metadata()
        for mode in ("missing","duplicate","suffix","wrongmod"):
            candidate=plan(meta)
            if mode=="missing":candidate["injections"].pop()
            elif mode=="duplicate":candidate["injections"].append(copy.deepcopy(candidate["injections"][0]))
            elif mode=="suffix":candidate["injections"][0]["injection_id"]="unrelated-advancement-tag-index"
            else:candidate["injections"][0]["mod_id"]="other"
            with self.assertRaisesRegex(ValueError,"exactly one"):MODULE.check_order(candidate,meta)

    def test_schema_fingerprint_phase_and_actor_binding_are_pinned(self):
        meta=metadata()
        for key,value in (("function_fingerprint","changed"),("priority",0),("kind","RETURN"),("cancellable",False),("file","elsewhere.rs"),("bindings",[])):
            candidate=plan(meta);candidate["injections"][0][key]=value
            with self.subTest(key=key),self.assertRaises(ValueError):MODULE.check_order(candidate,meta)
        candidate=plan(meta);candidate["schema"]=7
        with self.assertRaisesRegex(ValueError,"schema"):MODULE.check_order(candidate,meta)

    def test_pristine_source_plan_inputs_local_helper_and_selected_helper_must_all_match(self):
        meta=metadata();candidate=plan(meta)
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);(root/"src").mkdir();source=b"reviewed official body\n";(root/"src/advancements.rs").write_bytes(source)
            for index,m in enumerate(meta):
                helper=("reviewed helper "+str(index)).encode();(root/m["helper"]).write_bytes(helper)
                m.update(file_sha256=MODULE.digest(source),helper_sha256=MODULE.digest(helper))
                candidate["injections"][index]["helper_source"]=helper.decode()
            candidate["inputs"]={"src/advancements.rs":MODULE.digest(source)}
            MODULE.verify_source(candidate,root,meta,root)
            broken=copy.deepcopy(candidate);broken["inputs"]["src/advancements.rs"]="other"
            with self.assertRaisesRegex(ValueError,"bind"):MODULE.verify_source(broken,root,meta,root)
            broken=copy.deepcopy(candidate);broken["injections"][0]["helper_source"]+="changed"
            with self.assertRaisesRegex(ValueError,"exact reviewed"):MODULE.verify_source(broken,root,meta,root)
            (root/meta[0]["helper"]).write_text("changed")
            with self.assertRaisesRegex(ValueError,"adapter changed"):MODULE.verify_source(candidate,root,meta,root)
            (root/"src/advancements.rs").write_text("changed")
            with self.assertRaisesRegex(ValueError,"pristine"):MODULE.verify_source(candidate,root,meta,root)


if __name__ == "__main__":unittest.main()
