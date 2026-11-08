#!/usr/bin/env python3
"""Mods built on PlutoLib must not compete for injections.

PlutoLib owns each place in the game it hooks once and passes the event on to the callbacks mods register. This checks, over the
manifests of the repository:

1. PlutoLib's manifest hooks no `(target, kind, selector)` twice.
2. A mod other than PlutoLib hooks no place PlutoLib hooks.
3. A mod other than PlutoLib has no more injections than the recorded number (the number may only go down), so none is added.

`tools/injection_limits.json` records, per mod, `max` injections and the `known_overlaps` (ids that hook a place PlutoLib owns today and are
to move onto PlutoLib callbacks; the list may only shrink: an overlap not on it is a problem).
"""
import json
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PLUTO = ROOT / 'pluto-lib/mod/horizon-mixin.toml'
OTHERS = [ROOT / 'tconstruct/mixins/tconstruct/horizon-mixin.toml']
LIMITS = ROOT / 'tools/injection_limits.json'


def injections(path):
    return tomllib.loads(path.read_text(encoding='utf-8')).get('injection', [])


def place(injection):
    return (injection['target'], injection['kind'], injection.get('selector', ''))


def problems(pluto=PLUTO, others=None, limits=LIMITS):
    others = OTHERS if others is None else others
    found = []
    owned = {}
    for injection in injections(pluto):
        key = place(injection)
        if key in owned:
            found.append(f'PlutoLib hooks {key[0]} ({key[1]}) twice: `{owned[key]}` and `{injection["id"]}`; one injection per place, dispatching to callbacks')
        owned[key] = injection['id']
    recorded = json.loads(limits.read_text(encoding='utf-8')) if limits.exists() else {}
    for path in others:
        mine = injections(path)
        allowed = recorded.get(path.parent.name, {})
        known = set(allowed.get('known_overlaps', []))
        for injection in mine:
            key = place(injection)
            if key in owned and injection['id'] not in known:
                found.append(f'{path.parent.name}: `{injection["id"]}` hooks {key[0]}, which PlutoLib owns (`{owned[key]}`); register a callback with PlutoLib instead')
        if 'max' in allowed and len(mine) > allowed['max']:
            found.append(f'{path.parent.name}: {len(mine)} injections, more than the {allowed["max"]} recorded in tools/injection_limits.json; mods on PlutoLib add none')
    return found


def main():
    found = problems()
    for line in found:
        print(line, file=sys.stderr)
    print(f'injection ownership: {len(found)} problem(s)')
    return 1 if found else 0


if __name__ == '__main__':
    sys.exit(main())
