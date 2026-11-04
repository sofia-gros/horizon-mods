"""Extract the six test rows from a prepared resource root, without mutation.

Usage: python3 tconstruct/tests/extract_budding_piston_snapshot.py RESOURCE_ROOT > snapshot.json
The compacted arrays retain the rows' original fields (including original IDs).
"""
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
facts = json.loads((root / 'facts.json').read_text())
game = json.loads((root / 'gameplay-facts.json').read_text())
if len(facts['states']) != len(game['states']):
    raise ValueError('unaligned facts and gameplay states')
names = {'minecraft:amethyst_block', 'minecraft:budding_amethyst'} | {
    f'tconstruct:budding_{kind}_slime_crystal' for kind in ('earth', 'sky', 'ender', 'ichor')}
indices = [i for i, state in enumerate(facts['states']) if state[0] in names]
if len(indices) != len(names) or {facts['states'][i][0] for i in indices} != names:
    raise ValueError('missing or duplicate selected states')
print(json.dumps({'facts': {'states': [facts['states'][i] for i in indices]},
                  'gameplay': {'states': [game['states'][i] for i in indices]}}, indent=2))
