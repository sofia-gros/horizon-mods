"""Strictly undo reviewed later deltas before historical whole-source checks."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def undo(raw, row):
    if hashlib.sha256(raw).hexdigest() != row['after_sha256']:
        raise AssertionError('Later source bytes differ from the frozen inverse')
    text = raw.decode('utf-8')
    for before, after in reversed(row['edits']):
        if text.count(after) != 1: raise AssertionError('Later inverse is not unique')
        text = text.replace(after, before, 1)
    original = text.encode('utf-8')
    if hashlib.sha256(original).hexdigest() != row['before_sha256']:
        raise AssertionError('Later inverse did not restore the exact prior source')
    return original


def before_float_range(relative, raw):
    delta = json.loads((ROOT/'experiments/stage-float-range/inverse.json').read_bytes())
    row = delta['files'].get(relative)
    return undo(raw, row) if row is not None else raw


def before_jobs_one(relative, raw):
    raw = before_float_range('tconstruct/'+relative, raw)
    delta = json.loads((ROOT/'experiments/windows-jobs-one/inverse.json').read_bytes())
    row = delta['files'].get(relative)
    return undo(raw, row) if row is not None else raw


def before_package_contract(relative, raw, *, before_stat=False):
    raw = before_jobs_one(relative, raw)
    delta = json.loads((ROOT/'experiments/windows-stage-package-contract/inverse.json').read_text(encoding='utf-8'))
    row = delta['files'].get('tconstruct/'+relative)
    if row is not None: raw = undo(raw, row)
    if before_stat and relative == 'tools/play_windows_latest.py':
        stat_delta = json.loads((ROOT/'experiments/windows-stat-compat/inputs.json').read_text(encoding='utf-8'))
        raw = undo(raw, stat_delta['files']['play_windows_latest.py'])
    return raw
