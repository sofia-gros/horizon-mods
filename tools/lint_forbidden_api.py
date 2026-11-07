#!/usr/bin/env python3
"""Fail when TConstruct code reaches around PlutoLib to host internals.

Policy (AGENTS.md): TConstruct uses PlutoLib APIs only. Legacy code that
predates the policy is tolerated through a ratchet baseline: per (rule, file)
the count may stay or shrink, never grow, and new files start at zero.
Zero-tolerance rules have no baseline entries.

  lint_forbidden_api.py                  check (CI)
  lint_forbidden_api.py --update-baseline   lower the baseline after cleanup
"""
import argparse, json, re, subprocess, sys
from fnmatch import fnmatch
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASELINE = ROOT / 'tools/forbidden_api_baseline.json'

# (rule id, file glob, regex, zero_tolerance, explanation)
RULES = [
    ('fork-reference', '*', r'sofia-gros/horizon(?![-_A-Za-z])', True,
     'depends on the owner\'s horizon fork; use the official horizon-rust repositories'),
    ('cargo-git-not-official', '*Cargo.toml', r'git\s*=\s*"(?!https://github\.com/horizon-rust/)', True,
     'git dependency outside horizon-rust'),
    ('host-crate-direct', 'tconstruct/mixins/*.rs', r'\bhorizon_(?:world|items|core|client|client_render|client_lib|protocol)::', False,
     'TConstruct code names a Horizon host crate; go through a PlutoLib API'),
    ('host-inventory-direct', 'tconstruct/mixins/*.rs', r'\bgame\.inventory\b', False,
     'direct inventory mutation; use PlutoLib inventory APIs'),
    ('host-injection-new', 'tconstruct/mixins/*/horizon-mixin.toml', r'^\[\[injection\]\]', False,
     'new TConstruct injection into the host; add a PlutoLib hook/API instead'),
]


# Files that quote the forbidden patterns in order to define or test them.
SELF = {'tools/lint_forbidden_api.py', 'tests/test_lint_forbidden_api.py'}


def tracked():
    out = subprocess.run(['git', 'ls-files', '-z'], cwd=ROOT, capture_output=True, check=True).stdout
    return [f for f in out.decode('utf-8').split('\0') if f]


def scan():
    counts = {}
    for rel in tracked():
        if rel in SELF:
            continue
        for rule, glob, pattern, _, _ in RULES:
            if not fnmatch(rel, glob):
                continue
            try:
                text = (ROOT / rel).read_text(encoding='utf-8', errors='replace')
            except OSError:
                continue
            n = len(re.findall(pattern, text, re.M))
            if n:
                counts.setdefault(rule, {})[rel] = n
    return counts


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('--update-baseline', action='store_true')
    a = ap.parse_args(argv)
    counts = scan()
    zero = {r[0] for r in RULES if r[3]}
    if a.update_baseline:
        old = json.loads(BASELINE.read_text()) if BASELINE.is_file() else {}
        new = {rule: {f: min(n, old.get(rule, {}).get(f, n)) for f, n in files.items()}
               for rule, files in counts.items() if rule not in zero}
        BASELINE.write_text(json.dumps(new, indent=1, sort_keys=True) + '\n')
        print('baseline written (counts only ever lowered, never raised)')
        return 0
    base = json.loads(BASELINE.read_text()) if BASELINE.is_file() else {}
    explain = {r[0]: r[4] for r in RULES}
    bad = []
    for rule, files in counts.items():
        for f, n in files.items():
            allowed = 0 if rule in zero else base.get(rule, {}).get(f, 0)
            if n > allowed:
                bad.append(f'{f}: [{rule}] {n} > allowed {allowed} - {explain[rule]}')
    for line in sorted(bad):
        print(line, file=sys.stderr)
    legacy = sum(sum(v.values()) for k, v in base.items())
    print(f'forbidden-API lint: {len(bad)} violation(s); {legacy} legacy occurrence(s) tolerated by baseline')
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main())
