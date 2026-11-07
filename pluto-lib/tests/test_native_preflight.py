import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location("native_preflight", Path(__file__).resolve().parents[1] / "tools/check_native_plan.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class NativePreflightTests(unittest.TestCase):
    def test_all_guards_run_in_order_on_the_same_complete_plan(self):
        plan = {"injections": ["complete composition"]}
        entries = ["reviewed metadata"]
        metadata = {"payload": "reviewed source"}
        calls = []
        with patch.object(MODULE.protection, "check", side_effect=lambda p: calls.append(("protection", p))), \
             patch.object(MODULE.advancement, "check", side_effect=lambda p, w: calls.append(("advancement", p, w))), \
             patch.object(MODULE.equipment, "entries", return_value=entries), \
             patch.object(MODULE.equipment, "check_order", side_effect=lambda p, e: calls.append(("order", p, e))), \
             patch.object(MODULE.equipment, "verify_source", side_effect=lambda p, w, e: calls.append(("source", p, w, e))), \
             patch.object(MODULE.payload, "metadata", return_value=metadata), \
             patch.object(MODULE.payload, "check", side_effect=lambda p, m: calls.append(("payload", p, m))), \
             patch.object(MODULE.payload, "check_source", side_effect=lambda w, m: calls.append(("payload-source", w, m))), \
             patch.object(MODULE.placement, "check", side_effect=lambda p, w: calls.append(("placement", p, w))), \
             patch.object(MODULE.current_book, "metadata", return_value=metadata), \
             patch.object(MODULE.current_book, "check", side_effect=lambda p, m: calls.append(("book", p, m))), \
             patch.object(MODULE.current_book, "check_source", side_effect=lambda w, m: calls.append(("book-source", w, m))), \
             patch.object(MODULE.wire_snapshot, "check", side_effect=lambda p, w: calls.append(("wire-snapshot", p, w))), \
             patch.object(MODULE.matching, "check", side_effect=lambda p, w: calls.append(("matching", p, w))), \
             patch.object(MODULE.terrain, "metadata", return_value=metadata), \
             patch.object(MODULE.terrain, "check", side_effect=lambda p, m: calls.append(("terrain", p, m))), \
             patch.object(MODULE.terrain, "check_source", side_effect=lambda w, m: calls.append(("terrain-source", w, m))), \
             patch.object(MODULE.cloud, "check", side_effect=lambda p, w: calls.append(("cloud", p, w))), \
             patch.object(MODULE.station, "check", side_effect=lambda p, w: calls.append(("station", p, w))):
            MODULE.check(plan, "pristine")
        self.assertEqual(calls, [("protection", plan), ("advancement", plan, Path("pristine")), ("order", plan, entries), ("source", plan, Path("pristine"), entries), ("payload", plan, metadata), ("payload-source", Path("pristine"), metadata), ("placement", plan, Path("pristine")), ("book", plan, metadata), ("book-source", Path("pristine"), metadata), ("wire-snapshot", plan, Path("pristine")), ("matching", plan, Path("pristine")), ("terrain", plan, metadata), ("terrain-source", Path("pristine"), metadata), ("cloud", plan, Path("pristine")), ("station", plan, Path("pristine"))])

    def test_advancement_failure_stops_before_equipment_application_checks(self):
        with patch.object(MODULE.protection, "check"), \
             patch.object(MODULE.advancement, "check", side_effect=ValueError("copied target conflict")), \
             patch.object(MODULE.equipment, "verify_source") as source:
            with self.assertRaisesRegex(ValueError, "copied target conflict"):
                MODULE.check({}, "pristine")
            source.assert_not_called()

    def test_failure_stops_before_later_guards(self):
        with patch.object(MODULE.protection, "check", side_effect=ValueError("unsafe ordering")), \
             patch.object(MODULE.equipment, "verify_source") as source:
            with self.assertRaisesRegex(ValueError, "unsafe ordering"):
                MODULE.check({}, "pristine")
            source.assert_not_called()

    def test_payload_composition_failure_cannot_skip_through_to_success(self):
        with patch.object(MODULE.protection, "check"), \
             patch.object(MODULE.advancement, "check"), \
             patch.object(MODULE.equipment, "entries", return_value=[]), \
             patch.object(MODULE.equipment, "check_order"), \
             patch.object(MODULE.equipment, "verify_source"), \
             patch.object(MODULE.payload, "metadata", return_value={}), \
             patch.object(MODULE.payload, "check", side_effect=ValueError("packet observer conflict")), \
             patch.object(MODULE.payload, "check_source") as source:
            with self.assertRaisesRegex(ValueError, "packet observer conflict"):
                MODULE.check({}, "pristine")
            source.assert_not_called()

    def test_placement_failure_stops_before_current_book_guard(self):
        from contextlib import ExitStack
        with ExitStack() as stack:
            for module, name in [(MODULE.protection, "check"), (MODULE.advancement, "check"),
                                 (MODULE.equipment, "check_order"), (MODULE.equipment, "verify_source"),
                                 (MODULE.payload, "check"), (MODULE.payload, "check_source")]:
                stack.enter_context(patch.object(module, name))
            stack.enter_context(patch.object(MODULE.placement, "check", side_effect=ValueError("placement bypass")))
            later = stack.enter_context(patch.object(MODULE.current_book, "check"))
            with self.assertRaisesRegex(ValueError, "placement bypass"):
                MODULE.check({}, "pristine")
            later.assert_not_called()

    def test_book_failure_stops_before_private_wire_guard(self):
        from contextlib import ExitStack
        with ExitStack() as stack:
            for module, name in [(MODULE.protection, "check"), (MODULE.advancement, "check"),
                                 (MODULE.equipment, "check_order"), (MODULE.equipment, "verify_source"),
                                 (MODULE.payload, "check"), (MODULE.payload, "check_source"),
                                 (MODULE.placement, "check")]:
                stack.enter_context(patch.object(module, name))
            stack.enter_context(patch.object(MODULE.current_book, "check", side_effect=ValueError("book conflict")))
            later = stack.enter_context(patch.object(MODULE.wire_snapshot, "check"))
            with self.assertRaisesRegex(ValueError, "book conflict"):
                MODULE.check({}, "pristine")
            later.assert_not_called()

    def test_wire_failure_stops_before_crafting_match_guard(self):
        from contextlib import ExitStack
        with ExitStack() as stack:
            for module, name in [(MODULE.protection, "check"), (MODULE.advancement, "check"),
                                 (MODULE.equipment, "check_order"), (MODULE.equipment, "verify_source"),
                                 (MODULE.payload, "check"), (MODULE.payload, "check_source"),
                                 (MODULE.placement, "check"), (MODULE.current_book, "check"),
                                 (MODULE.current_book, "check_source")]:
                stack.enter_context(patch.object(module, name))
            stack.enter_context(patch.object(MODULE.wire_snapshot, "check", side_effect=ValueError("Wire drift")))
            later = stack.enter_context(patch.object(MODULE.matching, "check"))
            with self.assertRaisesRegex(ValueError, "Wire drift"):
                MODULE.check({}, "pristine")
            later.assert_not_called()

    def test_match_failure_stops_before_terrain_guard(self):
        from contextlib import ExitStack
        with ExitStack() as stack:
            for module, name in [(MODULE.protection, "check"), (MODULE.advancement, "check"),
                                 (MODULE.equipment, "check_order"), (MODULE.equipment, "verify_source"),
                                 (MODULE.payload, "check"), (MODULE.payload, "check_source"),
                                 (MODULE.placement, "check"), (MODULE.current_book, "check"),
                                 (MODULE.current_book, "check_source"), (MODULE.wire_snapshot, "check")]:
                stack.enter_context(patch.object(module, name))
            stack.enter_context(patch.object(MODULE.matching, "check", side_effect=ValueError("match conflict")))
            later = stack.enter_context(patch.object(MODULE.terrain, "check"))
            with self.assertRaisesRegex(ValueError, "match conflict"):
                MODULE.check({}, "pristine")
            later.assert_not_called()

    def test_missing_or_unreadable_input_fails_closed(self):
        self.assertEqual(MODULE.main([]), 1)
        self.assertEqual(MODULE.main(["/not-a-plan", "/not-a-workspace"]), 1)

    def test_new_guard_failures_cannot_skip_through_to_success(self):
        from contextlib import ExitStack
        for failing in (MODULE.cloud, MODULE.station):
            with self.subTest(guard=failing.__name__), ExitStack() as stack:
                for module, name in [(MODULE.protection, "check"), (MODULE.advancement, "check"),
                                     (MODULE.equipment, "check_order"), (MODULE.equipment, "verify_source"),
                                     (MODULE.payload, "check"), (MODULE.payload, "check_source"),
                                     (MODULE.placement, "check"), (MODULE.current_book, "check"),
                                     (MODULE.current_book, "check_source"), (MODULE.wire_snapshot, "check"),
                                     (MODULE.matching, "check"), (MODULE.terrain, "check"),
                                     (MODULE.terrain, "check_source"), (MODULE.cloud, "check"),
                                     (MODULE.station, "check")]:
                    stack.enter_context(patch.object(module, name))
                stack.enter_context(patch.object(failing, "check", side_effect=ValueError("new boundary conflict")))
                with self.assertRaisesRegex(ValueError, "new boundary conflict"):
                    MODULE.check({}, "pristine")
                if failing is MODULE.cloud:
                    MODULE.station.check.assert_not_called()
