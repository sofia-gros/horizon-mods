#!/usr/bin/env python3
"""Mandatory preflight for this development bundle's complete installed plan.

The official launcher does not run repository-specific guards. Use the guarded
staging route; do not apply this experimental bundle directly through that route.
"""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys


def _load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


protection = _load("check_protection_plan")
advancement = _load("check_advancement_plan")
equipment = _load("check_damage_equipment_plan")
payload = _load("check_client_payload_plan")
placement = _load("check_recipe_placement_plan")
current_book = _load("check_current_book_plan")
wire_snapshot = _load("check_wire_snapshot_plan")
matching = _load("check_crafting_match_plan")
terrain = _load("check_terrain_binding_plan")
cloud = _load("check_cloud_insertion_plan")
station = _load("check_station_ingredient_plan")
terrain_cell = _load("check_terrain_cell_plan")


def check(plan, workspace, crafting_package_contract=None):
    """Reject missing services, unsafe composition, source drift or copied edits."""
    protection.check(plan)
    advancement.check(plan, Path(workspace))
    entries = equipment.entries()
    equipment.check_order(plan, entries)
    equipment.verify_source(plan, Path(workspace), entries)
    metadata = payload.metadata()
    payload.check(plan, metadata)
    payload.check_source(Path(workspace), metadata)
    placement.check(plan, Path(workspace), crafting_package_contract)
    metadata = current_book.metadata()
    current_book.check(plan, metadata)
    current_book.check_source(Path(workspace), metadata)
    wire_snapshot.check(plan, Path(workspace))
    matching.check(plan, Path(workspace), crafting_package_contract)
    metadata = terrain.metadata()
    terrain.check(plan, metadata)
    terrain.check_source(Path(workspace), metadata)
    cloud.check(plan, Path(workspace))
    station.check(plan, Path(workspace))
    metadata = terrain_cell.metadata()
    terrain_cell.check(plan, metadata)
    terrain_cell.check_source(Path(workspace), metadata)


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    try:
        if len(argv) != 3:
            raise ValueError("usage: check_native_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE CRAFTING_PACKAGE_CONTRACT.json")
        check(json.loads(Path(argv[0]).read_text()), Path(argv[1]), Path(argv[2]))
    except (ValueError, OSError, KeyError, StopIteration, subprocess.CalledProcessError) as error:
        print(f"Pluto native preflight rejected: {error}", file=sys.stderr)
        return 1
    print("Complete Pluto native plan, ordering and reviewed host adapters verified.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
