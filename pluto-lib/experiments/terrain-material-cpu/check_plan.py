#!/usr/bin/env python3
"""Additional fixture-only gate. Run all ordinary complete-plan guards too."""
import argparse
import importlib.util
import json
from pathlib import Path
HERE=Path(__file__).resolve().parent

def check(plan,evidence,source,fingerprint):
    if plan.get('schema')!=6:raise ValueError('unreviewed plan schema')
    for category,count in evidence['plan_counts'].items():
        if len(plan.get(category,[]))!=count:raise ValueError('overlay plan count differs: '+category)
        if any(e.get('mod_id')!='pluto_native_creative' for e in plan.get(category,[])):raise ValueError('foreign overlay owner: '+category)
    modules=[m for m in plan.get('modules',[]) if m.get('name')=='pluto_terrain_material_cpu' or m.get('module_id')=='terrain-material-cpu-fixture']
    if len(modules)!=1:raise ValueError('expected exactly one test-only fixture module')
    m=modules[0]
    if m.get('mod_id')!='pluto_native_creative' or m.get('module_id')!='terrain-material-cpu-fixture' or m.get('file')!='crates/horizon-client-render/src/mesher.rs' or m.get('name')!='pluto_terrain_material_cpu' or m.get('fingerprint')!=fingerprint or m.get('source')!=source:
        raise ValueError('fixture module identity, placement or exact regenerated source changed')
    for category in ['injections','states','crates']:
        for entry in plan.get(category,[]):
            if 'terrain-material-cpu' in json.dumps(entry,ensure_ascii=False):raise ValueError('fixture must add no injection/state/crate')
    for name,sha in evidence['native_input_sha256'].items():
        if plan.get('inputs',{}).get(name)!=sha:raise ValueError('fixture native input drift: '+name)

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('fixture',type=Path);p.add_argument('plan',type=Path);a=p.parse_args()
    spec=importlib.util.spec_from_file_location('terrain_cpu_prepare',HERE/'prepare.py');prepare=importlib.util.module_from_spec(spec);spec.loader.exec_module(prepare)
    evidence=prepare.verify(a.fixture)
    source=(a.fixture/'mods/pluto_native_creative'/prepare.MODULE).read_text()
    meta=json.loads((prepare.ROOT/'tools/terrain-cell-source.json').read_text())
    check(json.loads(a.plan.read_text()),evidence,source,meta['modules'][0]['fingerprint'])
    print('One test-only actual-type module and exact regenerated baseline verified; no execution claim.')
if __name__=='__main__':
    try:main()
    except (OSError,ValueError,KeyError) as error:raise SystemExit(str(error))
