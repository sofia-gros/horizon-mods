"""Check the generated supported appearance cases and layer order."""
import itertools
import json
from pathlib import Path

assets = Path(__file__).resolve().parents[1] / "Tinkers' onstruct/assets/tconstruct"
rules = json.loads((assets / 'horizon/item_appearances/pickaxe.json').read_text())
assert len(rules['cases']) == 16
for head, handle, binding, broken in itertools.product(('wood', 'flint'), ('wood', 'flint'), ('wood', 'flint'), (False, True)):
    expected = {'/schema': 1, '/tool': 'tconstruct:pickaxe', '/materials/0': 'tconstruct:' + head,
                '/materials/1': 'tconstruct:' + handle, '/materials/2': 'tconstruct:' + binding, '/broken': broken}
    cases = [case for case in rules['cases'] if case['when'] == expected]
    assert len(cases) == 1, expected
    item = cases[0]['model'].split(':')[1]
    definition = json.loads((assets / 'items' / (item + '.json')).read_text())
    model = definition['model']['model'].split(':')[1]
    textures = json.loads((assets / 'models' / (model + '.json')).read_text())['textures']
    assert textures['layer0'].endswith('handle_' + handle)
    assert textures['layer1'].endswith(('broken_' if broken else 'head_') + head)
    assert textures['layer2'].endswith('binding_' + binding)
print('16 supported appearances have unique selections and correct layer order')
