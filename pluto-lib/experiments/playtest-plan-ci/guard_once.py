#!/usr/bin/env python3
"""Capture one unchanged guard's complete bounded streams in the outer own PGID."""
import hashlib, json, os, stat, subprocess, sys, time
from pathlib import Path

GUARDS = (
    'check_protection_plan.py', 'check_advancement_plan.py',
    'check_damage_equipment_plan.py', 'check_client_payload_plan.py',
    'check_recipe_placement_plan.py', 'check_current_book_plan.py',
    'check_wire_snapshot_plan.py', 'check_crafting_match_plan.py',
    'check_terrain_binding_plan.py', 'check_cloud_insertion_plan.py',
    'check_station_ingredient_plan.py', 'check_terrain_cell_plan.py',
)
MAX_STREAM = 16 * 1024

def plain(path, directory=False):
    path = Path(path)
    if any(p.is_symlink() for p in [path, *path.parents]):
        raise ValueError('Linked guard input')
    info = path.lstat()
    if not (stat.S_ISDIR(info.st_mode) if directory else stat.S_ISREG(info.st_mode)):
        raise ValueError('Nonregular guard input')
    return path

def stream(path):
    plain(path)
    size = path.stat().st_size
    if size > MAX_STREAM: raise ValueError('Guard stream exceeds evidence budget')
    raw = path.read_bytes()
    if len(raw) != size: raise ValueError('Guard stream changed during read')
    return {'bytes': size, 'sha256': hashlib.sha256(raw).hexdigest(), 'text': raw.decode('utf-8')}

def validate_record(value, script):
    if not isinstance(value, dict) or set(value) != {'script','exit_code','stdout','stderr'}:
        raise ValueError('Malformed guard record')
    if value['script'] != script or type(value['exit_code']) is not int or value['exit_code'] not in (0,1):
        raise ValueError('Unexpected guard termination')
    for key in ('stdout', 'stderr'):
        v = value[key]
        if not isinstance(v,dict) or set(v) != {'bytes','sha256','text'} or not isinstance(v['text'],str):
            raise ValueError('Malformed guard stream')
        raw = v['text'].encode('utf-8')
        if type(v['bytes']) is not int or v['bytes'] != len(raw) or len(raw)>MAX_STREAM or hashlib.sha256(raw).hexdigest()!=v['sha256']:
            raise ValueError('Guard stream identity mismatch')
    return value

def execute(root, plan, official, script, output):
    if os.environ.get('GITHUB_ACTIONS') != 'true': raise ValueError('CI only')
    if script not in GUARDS: raise ValueError('Unlisted guard')
    root=plain(root,True); plan=plain(plan); official=plain(official,True)
    target=plain(root/'pluto-lib/tools'/script)
    output=Path(output)
    if output.exists() or output.is_symlink(): raise ValueError('Guard output already exists')
    plain(output.parent,True); output.mkdir()
    argv=[sys.executable,str(target),str(plan)]
    if script != 'check_protection_plan.py': argv.append(str(official))
    child=None
    try:
        with (output/'stdout').open('xb') as stdout, (output/'stderr').open('xb') as stderr:
            # No session/group creation. The inherited Runner owns teardown of all children.
            child=subprocess.Popen(argv,cwd=root,stdout=stdout,stderr=stderr)
            while child.poll() is None:
                if any((output/name).stat().st_size>MAX_STREAM for name in ('stdout','stderr')):
                    raise ValueError('Guard stream exceeds evidence budget')
                time.sleep(0.1)
            code=child.wait()
        value={'script':script,'exit_code':code,'stdout':stream(output/'stdout'),'stderr':stream(output/'stderr')}
        return validate_record(value,script)
    finally:
        if child is not None and child.poll() is None:
            child.terminate()
            try:child.wait(timeout=2)
            except subprocess.TimeoutExpired:child.kill();child.wait(timeout=2)

def main():
    if len(sys.argv)!=6:raise SystemExit('Expected root plan official guard output')
    try:value=execute(*sys.argv[1:])
    except BaseException as exc:
        print(type(exc).__name__+': '+str(exc),file=sys.stderr)
        raise SystemExit(98)
    print(json.dumps(value,sort_keys=True),flush=True)

if __name__=='__main__':main()
