#!/usr/bin/env python3
"""Dependency closure over TConstruct's Mixins manifest.

Given seed injection ids, report the injections/modules/states that must stay
enabled for them (helpers reference module names; modules reference modules).
"""
import re, sys, tomllib
from pathlib import Path

D = Path(__file__).resolve().parents[1] / 'tconstruct/mixins/tconstruct'
ident = re.compile(r'[A-Za-z_][A-Za-z0-9_]*')
inj = mods = states = T = None

def load(bundle=D):
    """Select the bundle directory (holding horizon-mixin.toml) to analyse."""
    global D, T, inj, mods, states
    D = Path(bundle)
    T = tomllib.loads((D / 'horizon-mixin.toml').read_text(encoding='utf-8'))
    inj = {i['id']: i for i in T['injection']}
    mods = {m['name']: m for m in T['module']}
    states = {s['id']: s for s in T.get('state', [])}

def text(f):
    return (D / f).read_text(encoding='utf-8', errors='replace')

_defs = None
def _method_defs():
    global _defs
    if _defs is None or _defs[0] is not T:
        found = {}
        for name, m in mods.items():
            found.setdefault(name, set()).update(re.findall(r'fn\s+(__horizon_mixins_[A-Za-z0-9_]+)', text(m['source'])))
        _defs = (T, {k: v for k, v in found.items() if v})
    return _defs[1]

def refs(src):
    toks = set(ident.findall(src))
    # State accessors are generated (e.g. `<id>_mut`), so match state ids as substrings.
    st = {x for x in states if any(x.replace('-', '_') in t for t in toks)}
    # Injections are callable through generated `__horizon_mixins_<mod>_<id>` names.
    prefix = '__horizon_mixins_' + T['mod']['id'].replace('-', '_') + '_'
    ij = {i for i in inj if prefix + i.replace('-', '_') in toks}
    # Modules may also be reached through methods they define on host types.
    viaMethod = {m for m, fns in _method_defs().items() if fns & toks}
    return {m for m in mods if m in toks} | viaMethod, st, ij

def closure(seeds, module_ids=()):
    """Transitive closure from injection ids and module ids (not names).

    Returns (injections, module names, states, tokens) where tokens are every
    identifier in the kept sources (for cross-bundle dependency lookups).
    """
    keep_inj, keep_mod, keep_state, tokens = set(seeds), set(), set(), set()
    by_id = {m['id']: m['name'] for m in T['module']}
    todo = [('i', x) for x in seeds]
    for mid in module_ids:
        keep_mod.add(by_id[mid]); todo.append(('m', by_id[mid]))
    while todo:
        kind, name = todo.pop()
        src = text(inj[name]['helper']) if kind == 'i' else text(mods[name]['source'])
        tokens |= set(ident.findall(src))
        m, st, ij = refs(src)
        for x in m - keep_mod:
            keep_mod.add(x); todo.append(('m', x))
        for x in ij - keep_inj:
            keep_inj.add(x); todo.append(('i', x))
        for sid in st - keep_state:
            keep_state.add(sid)
            # A state's type/initializer may name modules (e.g. its struct).
            sm, _, _ = refs(repr(states[sid]))
            for x in sm - keep_mod:
                keep_mod.add(x); todo.append(('m', x))
    return keep_inj, keep_mod, keep_state, tokens

load()
if __name__ == '__main__':
    seeds = sys.argv[1:]
    for s in seeds:
        if s not in inj: sys.exit(f'unknown injection {s}')
    i, m, s, _ = closure(seeds)
    print(f'injections {len(i)}/{len(inj)}  modules {len(m)}/{len(mods)}  states {len(s)}/{len(states)}')
