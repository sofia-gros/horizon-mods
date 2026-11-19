"""The imported client assets (tools/import_original_data.py) must resolve inside the content mod and the pinned base assets."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
CONTENT = ROOT / 'tconstruct/content'
ASSETS = CONTENT / 'assets/tconstruct'
DECLARED = CONTENT / 'data/tconstruct/horizon'
BASE = ROOT / '.local/base-26.3/assets/minecraft'  # a read-only reference checkout; absent on some runners
# Declared items whose original model is plain (no tool/material loader); the rest wait for per-material rendering.
SIMPLE_ITEMS = {'modifier_crystal', 'pattern', 'venom_bottle'}


def load(path):
    return json.loads(path.read_text(encoding='utf-8'))


def split(reference):
    namespace, _, path = reference.partition(':') if ':' in reference else ('minecraft', '', reference)
    return namespace, path


class ContentAssets(unittest.TestCase):
    def resolves(self, kind, extension, reference):
        namespace, path = split(reference)
        if namespace == 'tconstruct':
            return (ASSETS / kind / f'{path}.{extension}').is_file()
        return namespace == 'minecraft' and (not BASE.is_dir() or (BASE / kind / f'{path}.{extension}').is_file())

    def test_every_model_parent_and_texture_resolves(self):
        models = sorted((ASSETS / 'models').rglob('*.json'))
        self.assertTrue(models)
        parents = {split(load(p)['parent'])[1] for p in models if 'parent' in load(p)}
        for path in models:
            model = load(path)
            leaf = path.relative_to(ASSETS / 'models').with_suffix('').as_posix() not in parents
            self.assertNotIn('loader', model, path)
            if 'parent' in model:
                self.assertTrue(self.resolves('models', 'json', model['parent']), f'{path}: parent {model["parent"]}')
            for name, texture in model.get('textures', {}).items():
                if not texture.startswith('#'):
                    self.assertTrue(self.resolves('textures', 'png', texture), f'{path}: texture {name}={texture}')
            for element in model.get('elements', []):
                for face in element['faces'].values():
                    reference = face['texture']
                    if leaf and reference.startswith('#'):
                        self.assertIn(reference[1:], self.inherited_textures(model), f'{path}: unbound {reference}')

    def inherited_textures(self, model):
        names = dict(model.get('textures', {}))
        parent = model.get('parent')
        while parent and split(parent)[0] == 'tconstruct':
            model = load(ASSETS / 'models' / f'{split(parent)[1]}.json')
            names = {**model.get('textures', {}), **names}
            parent = model.get('parent')
        return names

    def test_declared_blocks_and_simple_items_have_definitions(self):
        blocks = {p.stem for p in (DECLARED / 'block').glob('*.json')}
        items = {p.stem for p in (DECLARED / 'item').glob('*.json')}
        self.assertTrue(SIMPLE_ITEMS <= items)
        for name in blocks | SIMPLE_ITEMS:
            definition = load(ASSETS / 'items' / f'{name}.json')
            self.assertEqual(definition['model']['type'], 'minecraft:model')
            self.assertTrue(self.resolves('models', 'json', definition['model']['model']), name)
        for name in blocks:
            for variant in load(ASSETS / 'blockstates' / f'{name}.json')['variants'].values():
                self.assertTrue(self.resolves('models', 'json', variant['model']), name)
        for path in (ASSETS / 'items').glob('*.json'):
            self.assertIn(path.stem, blocks | items, f'item definition for an undeclared id: {path.name}')
        for path in (ASSETS / 'blockstates').glob('*.json'):
            self.assertIn(path.stem, blocks, f'blockstate for an undeclared block: {path.name}')

    def test_every_texture_is_used(self):
        used = set()
        for path in (ASSETS / 'models').rglob('*.json'):
            used |= {split(t)[1] for t in load(path).get('textures', {}).values() if split(t)[0] == 'tconstruct'}
        present = {p.relative_to(ASSETS / 'textures').with_suffix('').as_posix() for p in (ASSETS / 'textures').rglob('*.png')}
        self.assertEqual(present, used)


if __name__ == '__main__':
    unittest.main()
