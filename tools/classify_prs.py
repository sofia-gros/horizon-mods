#!/usr/bin/env python3
"""Classify open pull requests for re-absorption onto PlutoLib.

  A  pure logic / data: cherry-pick candidates (no host hooks, no host crates)
  B  host-API dependent: needs rewriting through PlutoLib
  C  UI / rendering hooks: rebuild (needs a PlutoLib station/UI API first)
  X  empty own diff: its content is already in the base branch

Each PR is judged on its OWN diff (against its base PR head when stacked).
Extra flags: fork-suspect (added Rust names a Horizon module that does not exist
in the official checkout), fork-ref (mentions the personal fork).

Needs: gh (logged in), pull refs fetched as origin/pr/N, an official horizon
checkout for --horizon. Output: JSON on stdout path --out.
"""
import argparse, collections, json, re, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
UI_TARGETS = re.compile(r'(menu_|draw|screen|render|tooltip|station_|click|inventory_(?:press|drag|mouse)|mouse|scroll|key\b|typing|text\b|slot|layout|gui|builder|mesh)', re.I)
UI_FILES = re.compile(r'(draw|render|screen|gui|tooltip|layout|builder|station|click|scroll|drag|mesh|-ui|_ui)', re.I)
HOST_CRATE = re.compile(r'\bhorizon_(world|items|core|client|client_lib|client_render|protocol)::([a-z_][a-z0-9_]*)')


def sh(*cmd, cwd=ROOT):
    return subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, encoding='utf-8', errors='replace').stdout


def host_modules(horizon):
    """crate (underscored) -> set of every `mod name` declared anywhere in it."""
    out = {}
    for crate in (horizon / 'crates').iterdir():
        mods = set()
        for rs in (crate / 'src').rglob('*.rs') if (crate / 'src').is_dir() else []:
            mods.update(re.findall(r'\bmod\s+([a-z_][a-z0-9_]*)', rs.read_text(encoding='utf-8', errors='replace')))
            mods.add(rs.stem)
        out[crate.name.replace('-', '_')] = mods
    return out


def classify(files, added_rust_names):
    if not files:
        return 'X', ['no own changes: already in the base']
    legacy = [f for f in files if f.startswith('tconstruct/mixins/')]
    mixin_rs = [f for f in legacy if f.endswith('.rs')]
    manifests = [f for f in legacy if f.endswith('horizon-mixin.toml')]
    rust_tests = [f for f in files if f.startswith('tconstruct/tests/rust/') or f.startswith('tconstruct/tests/') and f.endswith('.rs')]
    ui = [f for f in mixin_rs if UI_FILES.search(Path(f).name)]
    reasons = []
    if ui:
        reasons.append(f'UI/render helpers: {len(ui)}')
    if manifests:
        reasons.append('edits the injection manifest')
    if mixin_rs and not ui:
        reasons.append(f'host helper sources: {len(mixin_rs)}')
    if added_rust_names:
        reasons.append(f'names host crates: {len(added_rust_names)}')
    if ui:
        return 'C', reasons
    if mixin_rs or manifests or added_rust_names:
        return 'B', reasons
    if rust_tests:
        reasons.append('Rust fixtures (may import host crates)')
        return 'B', reasons
    return 'A', reasons or ['data / docs / python tests only']


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--horizon', type=Path, required=True, help='official horizon checkout (for the fork-suspect check)')
    ap.add_argument('--out', type=Path, required=True)
    ap.add_argument('--md', type=Path, help='also write the human-readable plan')
    a = ap.parse_args()
    prs = json.loads(sh('gh', 'pr', 'list', '--state', 'open', '--limit', '500', '--json',
                        'number,title,baseRefName,headRefName,isDraft,createdAt'))
    heads = {p['headRefName']: p['number'] for p in prs}
    mods = host_modules(a.horizon)
    rows = []
    for p in sorted(prs, key=lambda x: x['number']):
        n = p['number']
        base = f"origin/pr/{heads[p['baseRefName']]}" if p['baseRefName'] in heads else 'origin/main'
        diff = sh('git', 'diff', '--numstat', f'{base}...origin/pr/{n}').splitlines()
        files = [l.split('\t')[2] for l in diff if l.count('\t') >= 2]
        patch_rs = [f for f in files if f.endswith('.rs')]
        added = set()
        fork_ref = False
        if files:
            patch = sh('git', 'diff', '-U0', f'{base}...origin/pr/{n}', '--', *patch_rs) if patch_rs else ''
            for line in patch.splitlines():
                if line.startswith('+') and not line.startswith('+++'):
                    for crate, mod in HOST_CRATE.findall(line):
                        key = 'horizon_' + crate
                        if mod in mods.get(key, set()) or mod[0].isupper():
                            continue
                        added.add(f'{key}::{mod}')
            fork_ref = bool(re.search(r'sofia-gros/horizon(?![-_A-Za-z])', sh('git', 'diff', '-U0', f'{base}...origin/pr/{n}')))
        cls, reasons = classify(files, sorted(added))
        rows.append({'number': n, 'title': p['title'], 'base': p['baseRefName'], 'head': p['headRefName'],
                     'stacked_on': heads.get(p['baseRefName']), 'class': cls, 'reasons': reasons,
                     'files': len(files), 'fork_suspect': sorted(added), 'fork_ref': fork_ref})
    # Series = connected component of the stacking graph.
    parent = {r['number']: r['stacked_on'] for r in rows}
    def root(n):
        seen = set()
        while parent.get(n) and parent[n] not in seen:
            seen.add(n); n = parent[n]
        return n
    for r in rows:
        r['series_root'] = root(r['number'])
    a.out.write_text(json.dumps(rows, indent=1), encoding='utf-8')
    if a.md:
        write_md(a.md, rows)
    print(collections.Counter(r['class'] for r in rows), 'fork-suspect PRs:', sum(1 for r in rows if r['fork_suspect']))


def write_md(path, rows):
    by = collections.defaultdict(list)
    for r in rows:
        by[r['series_root']].append(r)
    count = collections.Counter(r['class'] for r in rows)
    L = ['# Open PR classification and re-absorption plan', '',
         'Generated by `tools/classify_prs.py` (heuristic, judged on the own diff of each PR). Re-run it; do not edit by hand.', '',
         '| Class | Meaning | PRs |', '|---|---|---|',
         f"| A | pure logic/data, cherry-pick candidates | {count['A']} |",
         f"| B | host-API dependent, rewrite through PlutoLib | {count['B']} |",
         f"| C | UI/render hooks, rebuild (blocked on a PlutoLib station-menu API) | {count['C']} |",
         f"| X | empty own diff, already in the base | {count['X']} |", '',
         f"Series (connected stacks): {len(by)}. `fork?` = added Rust names a Horizon module absent from the official checkout "
         f"({sum(1 for r in rows if r['fork_suspect'])} PRs); `forkref` = text mentions the personal fork "
         f"({sum(1 for r in rows if r['fork_ref'])} PRs). Both are signs of the old fork dependency.", '',
         '## Re-injection plan', '',
         'Rules: at most 5 open PRs at once; one PR = one verification unit (CI green + `tools/smoke-local`); batches of about 10 PRs per series, in stack order. Order:', '',
         '1. **Class A first** (data/logic): cheapest, no host risk. Batch them by series.',
         '2. **Class B** by series, largest dependency chains last. Each PR is rewritten onto PlutoLib APIs; add the missing PlutoLib API in its own PR first.',
         '3. **Class C** only after PlutoLib has the station-menu/screen API (see `part-builder-plutolib-gap.md`).',
         '4. Class X: close with a note.', '']
    L += ['## Series', '', '| Series (root) | PRs | A | B | C | X | fork? | forkref | first PR title |', '|---|---|---|---|---|---|---|---|---|']
    for root_n, v in sorted(by.items(), key=lambda kv: (-len(kv[1]), kv[0])):
        c = collections.Counter(r['class'] for r in v)
        v.sort(key=lambda r: r['number'])
        L.append(f"| #{root_n} | {len(v)} | {c['A']} | {c['B']} | {c['C']} | {c['X']} | {sum(1 for r in v if r['fork_suspect'])} | {sum(1 for r in v if r['fork_ref'])} | {next(r for r in v if r['number'] == root_n)['title'][:70] if any(r['number'] == root_n for r in v) else v[0]['title'][:70]} |")
    L += ['', '## Every PR', '', '| PR | Class | Stacked on | Files | Why | Flags | Title |', '|---|---|---|---|---|---|---|']
    for r in sorted(rows, key=lambda r: r['number']):
        flags = ' '.join(f for f, on in (('fork?', bool(r['fork_suspect'])), ('forkref', r['fork_ref'])) if on)
        L.append(f"| #{r['number']} | {r['class']} | {('#' + str(r['stacked_on'])) if r['stacked_on'] else ('main' if r['base'] == 'main' else r['base'])} | {r['files']} | {'; '.join(r['reasons'])} | {flags} | {r['title'][:80].replace('|', '/')} |")
    path.write_text(chr(10).join(L) + chr(10), encoding='utf-8', newline='')


if __name__ == '__main__':
    sys.exit(main())
