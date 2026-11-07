#!/usr/bin/env python3
"""Write a reduced copy of the TConstruct Mixins bundle for a vertical slice.

The full manifest stays untouched. Only the seed injections, the modules and
states their helpers (transitively) use, and all non-injection tables are kept.
The output is a bundle directory: for TConstruct place it as <mods>/tconstruct
and pass <mods> to stage.py --mods; for PlutoLib pass it as --pluto-bundle.
"""
import argparse, re, shutil, sys, tomllib
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
import slice_deps as sd

p = argparse.ArgumentParser()
p.add_argument('--src', type=Path, default=sd.D, help='bundle directory holding horizon-mixin.toml (default: TConstruct)')
p.add_argument('--keep-prefix', action='append', default=[], help='keep every injection and module whose id starts with this (repeatable)')
p.add_argument('--seed', action='append', default=[], help='injection id to keep (repeatable)')
p.add_argument('--keep-module-name', action='append', default=[], help='keep modules with this name (repeatable; for cross-bundle deps)')
p.add_argument('--tokens-out', type=Path, help='write identifiers used by the kept sources, for another bundle')
p.add_argument('--all-except', action='append', default=None, metavar='ID', help='keep every injection except these ids (repeatable; pass --all-except NONE for no exclusions)')
p.add_argument('--avoid-file', action='append', default=[], metavar='HOST_FILE',
               help='drop every injection whose helpers need a module that edits this Horizon source file (repeatable)')
p.add_argument('--out', type=Path, required=True, help='fresh bundle directory to write')
a = p.parse_args()
if a.out.exists():
    sys.exit(f'{a.out}: already exists')
sd.load(a.src)
missing = [x for x in a.seed if x not in sd.inj]
if missing:
    sys.exit(f'unknown injection id(s): {missing}')
if a.all_except is not None:
    a.seed = [i for i in sd.inj if i not in a.all_except]
seeds = list(a.seed) + [i for i in sd.inj if any(i.startswith(x) for x in a.keep_prefix)]
module_ids = [m['id'] for m in sd.T['module'] if any(m['id'].startswith(x) for x in a.keep_prefix) or m['name'] in a.keep_module_name]
if a.avoid_file:
    files_of = {}
    for m in sd.T['module']:
        files_of.setdefault(m['name'], set()).add(m['file'])
    def touches(ident):
        _, mods, _, _ = sd.closure([ident])
        return any(f in a.avoid_file for n in mods for f in files_of.get(n, ()))
    dropped_inj = [i for i in seeds if touches(i)]
    seeds = [i for i in seeds if i not in dropped_inj]
    print(f'avoiding {len(a.avoid_file)} host file(s): dropped {len(dropped_inj)} injection(s)')
if not seeds and not module_ids:
    sys.exit('nothing selected: pass --seed or --keep-prefix')
keep_i, keep_m, keep_s, tokens = sd.closure(seeds, module_ids)
bundle = a.out
shutil.copytree(a.src, bundle)
manifest = bundle / 'horizon-mixin.toml'
header = re.compile(r'^\[\[([A-Za-z_.]+)\]\]\s*$')
blocks, cur = [], ['', []]
for line in manifest.read_text(encoding='utf-8').splitlines(keepends=True):
    m = header.match(line)
    if m and not m.group(1).endswith('.constructor'):
        blocks.append(cur); cur = [m.group(1), []]
    cur[1].append(line)
blocks.append(cur)
out, dropped = [], {'injection': 0, 'module': 0, 'state': 0}
for kind, lines in blocks:
    body = ''.join(lines)
    if kind in dropped:
        t = tomllib.loads(body)[kind][0]
        ok = {'injection': t.get('id') in keep_i, 'module': t.get('name') in keep_m, 'state': t.get('id') in keep_s}[kind]
        if not ok:
            dropped[kind] += 1; continue
    out.append(body)
manifest.write_text(''.join(out), encoding='utf-8', newline='\n')
if a.tokens_out:
    a.tokens_out.write_text(' '.join(sorted(tokens)), encoding='utf-8', newline='\n')
print(f'kept injections={len(keep_i)} modules={len(keep_m)} states={len(keep_s)}; dropped {dropped}')
