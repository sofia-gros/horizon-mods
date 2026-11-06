#!/usr/bin/env python3
"""Read only existing MOD Git objects; never fetch, checkout, build or change PRs.

Usage: python reproduce.py --repo /path/to/horizon-mods --out /tmp/map
The generated JSON/CSVs are deterministic for the two frozen input JSON files.
Missing external PR objects are reported; missing selected objects are fatal.
"""
import argparse
import collections
import csv
import hashlib
import io
import json
from pathlib import Path
import subprocess
import tomllib

HERE = Path(__file__).resolve().parent
args = argparse.ArgumentParser()
args.add_argument('--repo', required=True)
args.add_argument('--out', type=Path, required=True)
a = args.parse_args()
repo = Path(a.repo).resolve()


def git(*xs, data=None, optional=False):
    p = subprocess.run(['git', '-C', str(repo), *xs], input=data,
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if p.returncode and not optional:
        raise RuntimeError(p.stderr.decode())
    return None if p.returncode else p.stdout


def sha(data):
    return hashlib.sha256(data).hexdigest()


def text(*xs):
    return git(*xs).decode().strip()


def manifest(commit, path):
    data = git('show', f'{commit}:{path}', optional=True)
    return {} if data is None else tomllib.loads(data.decode())


def diff(base, head, *extra):
    return git('diff', '--no-ext-diff', '--no-textconv', '--no-renames',
               '--full-index', '--binary', base, head, *extra)


snapshot = json.loads((HERE / 'frozen-prs.json').read_text())
scope = json.loads((HERE / 'scope.json').read_text())
by_n = {r['n']: r for r in snapshot}
assert len(by_n) == len(snapshot)
groups = {n: g for g in scope['groups'] for n in g['prs']}
assert len(groups) == sum(len(g['prs']) for g in scope['groups']) == 51
main = scope['historical_main']
file_rows, rows, all_paths, missing = [], [], {}, []
manifest_path = 'tconstruct/mixins/tconstruct/horizon-mixin.toml'
transitions = collections.defaultdict(list)
patch_ids = collections.defaultdict(list)

for p in sorted(snapshot, key=lambda p: p['n']):
    n, head, base = p['n'], p['sha'], p['base_sha']
    absent = [s for s in (base, head)
              if git('cat-file', '-e', s + '^{commit}', optional=True) is None]
    if absent:
        assert n not in groups, (n, absent)
        missing.append({'number': n, 'missing_commits': absent})
        continue
    bases = text('merge-base', '--all', base, head).splitlines()
    assert len(bases) == 1, (n, bases)
    mb = bases[0]
    paths = text('diff', '--no-renames', '--name-only', mb, head).splitlines()
    all_paths[n] = set(paths)
    if n not in groups:
        continue
    patch = diff(mb, head)
    patch_id = git('patch-id', '--stable', data=patch).decode().split()[0]
    patch_ids[patch_id].append(n)
    main_mb = text('merge-base', main, head)
    cumulative = text('diff', '--no-renames', '--name-only', main_mb, head).splitlines()
    parents = text('show', '-s', '--format=%P', head).split()
    stack = []
    cursor = p['parent']
    while cursor is not None:
        assert cursor in by_n and cursor not in stack
        stack.append(cursor)
        cursor = by_n[cursor]['parent']
    stats = {}
    for line in text('diff', '--no-renames', '--numstat', mb, head).splitlines():
        add, delete, path = line.split('\t', 2)
        stats[path] = (add, delete)
    raw = git('diff', '--no-renames', '--raw', '--no-abbrev', '-z', mb, head).split(b'\0')
    changed = []
    for i in range(0, len(raw) - 1, 2):
        oldmode, newmode, oldblob, newblob, status = raw[i].decode()[1:].split()
        path = raw[i+1].decode()
        add, delete = stats[path]
        fp = sha(diff(mb, head, '--', path))
        file_rows.append([n, status, path, oldmode, newmode, oldblob, newblob, add, delete, fp])
        transitions[(path, oldmode, newmode, oldblob, newblob)].append(n)
        changed.append(path)
    assert set(changed) == set(paths)
    hook_changes = []
    if manifest_path in paths:
        before, after = manifest(mb, manifest_path), manifest(head, manifest_path)
        for kind in ('injection', 'state', 'module'):
            old = {e['id']: e for e in before.get(kind, [])}
            new = {e['id']: e for e in after.get(kind, [])}
            for ident in sorted(old.keys() | new.keys()):
                if old.get(ident) != new.get(ident):
                    hook_changes.append({'table': kind, 'id': ident,
                                         'before': old.get(ident), 'after': new.get(ident)})
    rows.append({**p, 'review_group': groups[n]['id'],
                 'related_boundary_ids_not_migration_receipt': groups[n]['related_boundary_ids'],
                 'merge_base': mb, 'base_is_ancestor': mb == base,
                 'head_parents': parents, 'base_matches_first_parent': parents[0] == base,
                 'declared_parent_head_matches_base': p['parent'] is None or by_n[p['parent']]['sha'] == base,
                 'stack_ancestors': stack, 'patch_sha256': sha(patch), 'stable_patch_id': patch_id,
                 'unique_delta_files': len(paths), 'changed_paths': paths,
                 'main_comparison_merge_base': main_mb, 'cumulative_files_from_main': len(cumulative),
                 'cumulative_paths_not_in_this_delta': len(set(cumulative) - set(paths)),
                 'native_manifest_changes': hook_changes,
                 'prior_20261105_audit_not_revalidated': scope['prior_audit'][str(n)]})

paths_index = collections.defaultdict(list)
for n, paths in all_paths.items():
    for path in paths:
        paths_index[path].append(n)
overlaps = []
for path, ns in sorted(paths_index.items()):
    inside, outside = sorted(set(ns) & groups.keys()), sorted(set(ns) - groups.keys())
    if inside and len(ns) > 1:
        overlaps.append({'path': path, 'selected_prs': inside, 'other_prs': outside,
                         'meaning': 'same_path_review_intersection_not_duplicate_or_conflict_proof'})

# Compare only newly added/changed injection descriptors, not every inherited hook.
# Exact string equality is not alias normalization and is not a successful plan.
central_hooks = []
for path in text('ls-tree', '-r', '--name-only', scope['central_local'], '--', 'pluto-lib/mixins').splitlines():
    if path.endswith('horizon-mixin.toml'):
        for e in manifest(scope['central_local'], path).get('injection', []):
            central_hooks.append({'manifest': path, **e})
hook_intersections = []
for r in rows:
    for change in r['native_manifest_changes']:
        e = change['after']
        if change['table'] == 'injection' and e:
            matches = [h for h in central_hooks if h['target'] == e['target']]
            if matches:
                hook_intersections.append({'number': r['n'], 'old_pr_injection': e,
                                           'central_injections': matches,
                                           'meaning': 'exact_target_string_intersection_requires_composition_review'})

exact_transitions = [{'path': key[0], 'old_mode': key[1], 'new_mode': key[2],
                     'old_blob': key[3], 'new_blob': key[4], 'prs': ns}
                    for key, ns in sorted(transitions.items()) if len(ns) > 1]
duplicates = [{'stable_patch_id': k, 'prs': v} for k, v in sorted(patch_ids.items()) if len(v) > 1]
result = {
    'schema': 1, 'kind': 'immutable_scoped_pr_delta_ledger_not_merge_plan',
    'repository': scope['repository'],
    'input_sha256': {f: sha((HERE/f).read_bytes()) for f in ('frozen-prs.json', 'scope.json')},
    'central_source': scope['central_local'], 'central_remote': scope['central_remote'],
    'historical_main': main,
    'method': 'Git merge-base(base_sha,head_sha), then exact no-renames binary full-index diff. Patch IDs are normalized hints; SHA256 is exact diff identity. Path intersections are not semantic duplicates.',
    'summary': {'snapshot_prs': len(snapshot), 'selected_prs': len(rows),
                'external_prs_with_diff': len(all_paths) - len(rows), 'external_prs_missing_objects': len(missing),
                'base_is_ancestor': sum(r['base_is_ancestor'] for r in rows),
                'first_parent_is_base': sum(r['base_matches_first_parent'] for r in rows),
                'stacked_prs': sum(r['parent'] is not None for r in rows),
                'file_delta_records': len(file_rows), 'distinct_selected_paths': len({r[2] for r in file_rows}),
                'same_path_intersections': len(overlaps), 'cross_scope_path_intersections': sum(bool(r['other_prs']) for r in overlaps),
                'exact_same_file_transitions': len(exact_transitions), 'whole_patch_id_groups': len(duplicates),
                'exact_native_target_intersections': len(hook_intersections)},
    'missing_external_objects': missing, 'prs': rows, 'path_intersections': overlaps,
    'exact_file_transition_duplicates': exact_transitions, 'whole_patch_id_duplicates': duplicates,
    'central_native_target_intersections': hook_intersections,
    'limits': ['Not a live refreshed GitHub status or final category assignment.',
               'No tests, CI, plans, builds, merge, retarget, close or production activation performed.',
               'Shared APIs do not repair original PR heads or prove consumer activation.',
               'Prior audit findings retained as historical evidence, not a fresh official-host revalidation.',
               'Unselected recipe/station PRs may exist; external comparison covers paths, not semantic completeness.',
               'Alias-qualified native targets require semantic review beyond exact-string intersections.']}
a.out.mkdir(parents=True, exist_ok=True)
(a.out/'ledger.json').write_text(json.dumps(result, indent=2)+'\n')
with (a.out/'file-deltas.csv').open('w', newline='') as f:
    w=csv.writer(f, lineterminator="\n"); w.writerow(['pr','status','path','old_mode','new_mode','old_blob','new_blob','additions','deletions','patch_sha256']); w.writerows(file_rows)
with (a.out/'path-intersections.csv').open('w', newline='') as f:
    w=csv.writer(f, lineterminator="\n"); w.writerow(['path','selected_prs','other_prs','interpretation'])
    for r in overlaps: w.writerow([r['path'],' '.join(map(str,r['selected_prs'])),' '.join(map(str,r['other_prs'])),r['meaning']])
print(json.dumps(result['summary'], indent=2))
