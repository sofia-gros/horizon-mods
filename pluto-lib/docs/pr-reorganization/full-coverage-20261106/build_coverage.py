#!/usr/bin/env python3
"""Read immutable domain ledgers; write a normalized coverage report only."""
import argparse
import collections
import hashlib
import json
from pathlib import Path

REPO = 'https://github.com/sofia-gros/horizon-mods'
SOURCES = {
    'recipe': ('ab037b6a708b647c9ed90af33a661adca49df1c7', 'pluto-lib/docs/pr-reorganization/recipe-advancement-20261106/ledger.json', 'c4711ebbf8e8d25c5e88e0cb948c8bfd6a94d39b2b74c7da4dad525d2bb63282'),
    'storage': ('5c4e75628259ee10cf4f7943b1fa6632ffe44f54', 'pluto-lib/docs/storage-pr-dependency-map-20261106.json', '526e75248122cab62d2b6284e8161cbf38b5555016238c7521f5087ea38dead4'),
    'guide': ('4294fb986c6cadca9ad66c73c5d781a2cf039a5d', 'pluto-lib/docs/migration-inventory/guide-own-inherited-intake-20261106.json', '7a21ba311a302ab22011374ebe7db959abc27fb4589f1887c7208256647a294d'),
    'armor_world': ('102aa57d27a88542cea4fae8ec0b472043ec7d57', 'pluto-lib/docs/inventory/armor-defense-pr-deltas-20261106.json', 'c316d2e850a602eb668e7ece930d54aff910819a39f228610d3988fd28c21cdf'),
    'items_tools': ('04c66027876fb4ebbce2833acdf872a090259ec4', 'pluto-lib/docs/pr-reorganization/items-tools-combat-20261106/ledger.json', 'b3ceb51d48606ae2ea30e9acdd0db9515e5cc08da0ee8be857675bc19317f96d'),
}
EXPECTED = {'recipe_advancement': 51, 'storage_machines': 70, 'guide': 24, 'foundation': 1, 'world_fluids': 23, 'armor_defense': 59, 'items_tools_combat': 37}

def read(path, expected=None):
    data = Path(path).read_bytes()
    sha = hashlib.sha256(data).hexdigest()
    if expected is not None:
        assert sha == expected, (str(path), sha, expected)
    return json.loads(data), sha

def build(snapshot_path, paths):
    snapshot, snapshot_sha = read(snapshot_path, '326066269ca9c55183e3e6f53546b290db7331f05dc0bf467cf5becdab9a4651')
    frozen = {r['number']: r for r in snapshot['pull_requests']}
    assert len(frozen) == len(snapshot['pull_requests']) == 265
    inputs = {k: read(paths[k], spec[2])[0] for k, spec in SOURCES.items()}
    rows = []

    def add(key, partition, row, number, head, reported, own_base, count, digest, ancestor, locator, convention, patch_id=None):
        fixed = frozen[number]
        assert (head, reported) == (fixed['head_sha'], fixed['reported_base_sha']), (number, 'snapshot mismatch')
        for value in [head, reported, own_base]:
            assert isinstance(value, str) and len(value) == 40 and all(c in '0123456789abcdef' for c in value)
        assert isinstance(count, int) and count >= 0
        assert isinstance(digest, str) and len(digest) == 64
        rows.append({
            'number': number, 'title': fixed['title'], 'url': fixed['url'],
            'primary_research_partition': partition,
            'head_sha': head, 'head_ref': fixed['head_ref'],
            'reported_base_sha': reported, 'base_ref': fixed['base_ref'],
            'parent_open_pr': fixed['parent_open_pr'], 'snapshot_state': fixed['state'], 'snapshot_draft': fixed['draft'],
            'own_delta_base_sha': own_base,
            'own_delta_base_evidence': 'remote ancestry and exact-tree evidence in guide ledger' if number in [944, 945] else 'actual Git merge-base in domain ledger',
            'reported_base_is_ancestor': ancestor,
            'own_delta_path_count': count, 'path_count_convention': convention,
            'own_diff_sha256': digest, 'stable_patch_id': patch_id,
            'source_input': key, 'source_row': locator,
            'review_status': 'source_inventory_only_not_runtime_or_migration_completion',
        })

    for r in inputs['recipe']['prs']:
        add('recipe', 'recipe_advancement', r, r['n'], r['sha'], r['base_sha'], r['merge_base'], r['unique_delta_files'], r['patch_sha256'], r['base_is_ancestor'], f"prs[n={r['n']}]", 'domain Git no-rename path count', r['stable_patch_id'])
    for r in inputs['storage']['rows']:
        add('storage', 'storage_machines', r, r['number'], r['head_sha'], r['reported_base_sha'], r['actual_merge_base_sha'], len(r['incremental_paths']), r['incremental_diff_sha256'], r['base_is_ancestor_of_head'], f"rows[number={r['number']}]", 'domain Git no-rename path count', r['stable_incremental_patch_id'])
    for r in inputs['guide']['rows']:
        n = r['number']
        ancestor = r.get('reported_base_is_ancestor')
        if ancestor is None:
            # The guide ledger omits the boolean for an exact ancestor base.
            # Equality to its DAG-verified own base proves this fact; remote
            # parent-chain evidence supplies the same fact for #944/#945.
            assert r['effective_own_delta_base_sha'] == r['reported_base_sha']
            assert r['ancestry_evidence'] in [
                'local immutable commit DAG',
                'remote commit parent is exact main; local tree equality verified',
                'remote single-parent chain reaches exact #944 head in three commits',
            ]
            ancestor = True
        add('guide', 'foundation' if n == 944 else 'guide', r, n, r['head_sha'], r['reported_base_sha'], r['effective_own_delta_base_sha'], r['own_delta']['changed_entries'], r['own_delta']['patch_sha256'], ancestor, f'rows[number={n}]', 'guide rename-aware changed entries; #944 summarizes full 476-entry delta')
        rows[-1]['reported_base_ancestry_evidence'] = r['ancestry_evidence']
    for r in inputs['armor_world']['rows']:
        add('armor_world', 'armor_defense', r, r['number'], r['head_sha'], r['reported_base_sha'], r['actual_merge_base_sha'], r['incremental_path_count'], r['incremental_diff_sha256'], r['reported_base_is_ancestor'], f"rows[number={r['number']}]", 'domain Git no-rename path count', r['stable_incremental_patch_id'])
    for r in inputs['armor_world']['world_merge_base_supplement']:
        add('armor_world', 'world_fluids', r, r['number'], r['head_sha'], r['reported_base_sha'], r['actual_merge_base_sha'], r['incremental_path_count'], r['incremental_diff_sha256'], r['reported_base_is_ancestor'], f"world_merge_base_supplement[number={r['number']}]", 'domain Git no-rename path count', r['stable_incremental_patch_id'])
    for r in inputs['items_tools']['prs']:
        add('items_tools', 'items_tools_combat', r, r['number'], r['head_sha'], r['reported_base_sha'], r['actual_merge_base_sha'], r['own_delta_file_count'], r['exact_diff_sha256'], r['reported_base_is_ancestor'], f"prs[number={r['number']}]", 'domain Git no-rename path count', r['stable_patch_id'])

    counts = dict(collections.Counter(r['primary_research_partition'] for r in rows))
    ids = [r['number'] for r in rows]
    assert len(rows) == len(set(ids)) == 265
    assert set(ids) == set(frozen)
    assert counts == EXPECTED, counts
    assert all(isinstance(r['reported_base_is_ancestor'], bool) for r in rows)
    rows.sort(key=lambda r: r['number'])
    nonancestor = [r['number'] for r in rows if r['reported_base_is_ancestor'] is False]
    sources = {key: {'commit': commit, 'repository_path': path, 'sha256': sha,
                     'url': f'{REPO}/blob/{commit}/{path}' if path else f'{REPO}/commit/{commit}'}
               for key, (commit, path, sha) in SOURCES.items()}
    return {
        'schema': 'pluto_pr_coverage_v1', 'kind': 'read_only_fixed_snapshot_coverage_not_intake_or_runtime_proof',
        'repository': 'sofia-gros/horizon-mods', 'snapshot_observed_at_utc': snapshot['observed_at_utc'],
        'snapshot_sha256': snapshot_sha,
        'cutoffs': {'main': '08afa992d5091e03360f3473ea93fd17aa793987', 'central_local': '8715b1232c0fca133fe012f35830e2108e653d59', 'central_remote': '89744814b8d536a117e82f8ea3c8cdc9798faa95'},
        'architecture': 'Horizon -> Mixins -> PlutoLib for opting-in mods (Tinkers), otherwise directly -> Mixins Mod',
        'counts': {'snapshot_prs': 265, 'covered_primary_prs': len(rows), 'missing': 0, 'duplicate_primary': 0, 'head_base_matches': 265, 'own_delta_base_and_digest_present': 265, 'research_partitions': counts, 'known_reported_base_not_ancestor': nonancestor, 'remote_ancestry_instead_of_local_object': [944,945]},
        'source_inputs': sources,
        'additional_contract_ledgers': [
            {'scope': 'world-fluid contracts; 17 boundaries, not PR completion', 'url': f'{REPO}/commit/5f6c20539d7d7740bf559d06de60b52f38f62053'},
            {'scope': 'recipe/advancement contracts; 28 boundaries, not PR completion', 'url': f'{REPO}/commit/69e3ddc6f220def799031813ed025b19842de1f9'},
            {'scope': 'storage contracts; 14 boundaries, separate cutoffs', 'url': f'{REPO}/commit/016d867e7c693e31a462d97e8335e00c30b4c51c'},
            {'scope': 'guide contracts; 8 boundaries, foundation is cross-reference only', 'url': f'{REPO}/commit/42e9cbb3c044fe2f4bc0c324620dc0c58f2321a2'},
        ],
        'limits': [
            '265/265 is immutable metadata and own-delta inventory coverage, not reviewed, migrated, tested, merged, or feature-complete PRs.',
            'Seven research partitions are not the proposed eight product review groups and do not authorize PR state changes.',
            'Reported REST base, actual merge-base, first parent, and current main are different concepts; use the recorded own-delta base for intake.',
            'Diff digests and counts retain each domain auditor convention; rename-aware counts are not summed or treated as directly comparable to no-rename endpoints.',
            'Foundation #944 is counted once; Guide #945 depends on it rather than inheriting the 23-PR guide content chain.',
            'Head ancestry or matching blobs do not prove feature parity; same paths or native targets do not prove duplicate semantics.',
            'No current PR state, permissions, CI dispatch, source/runtime, or engine repository was changed by this aggregation.',
            'Original-source metadata references are recorded as evidence only, not used to fetch or inspect a user engine fork.',
        ],
        'prs': rows,
    }

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--snapshot', required=True)
    for key in SOURCES:
        parser.add_argument('--' + key.replace('_','-'), dest=key, required=True)
    parser.add_argument('--out', required=True)
    args = parser.parse_args()
    result = build(args.snapshot, {k: getattr(args,k) for k in SOURCES})
    out = Path(args.out)
    out.write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({'sha256': hashlib.sha256(out.read_bytes()).hexdigest(), 'bytes': out.stat().st_size, 'counts': result['counts']}, ensure_ascii=False))

if __name__ == '__main__':
    main()
