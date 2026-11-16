#!/usr/bin/env python3
"""Copy the original mod's generated tinkering data into the clean content root.

Source: the read-only checkout at .local/tconstruct-source (revision pinned by
tconstruct/compatibility.json). Files are copied byte for byte, never edited, so the content
root stays a mechanical projection of the original. Other-mod integration is left out:
the minotaur axe and every material whose condition depends on external ingot tags.
"""
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/tinkering'
TARGET = ROOT / 'tconstruct/content/data/tconstruct/tinkering'
FOLDERS = ['tool_definitions', 'materials/definition', 'materials/stats', 'materials/traits', 'modifiers']
TAG_FOLDERS = ['tags/materials', 'tags/modifiers']
EXCLUDED_TOOLS = {'minotaur_axe'}
# The original's Forge-era tag folders, renamed to the singular names the pinned host reads.
TAG_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/tags'
TAG_TARGET = ROOT / 'tconstruct/content/data/tconstruct/tags'
TAG_FOLDER_NAMES = {'items': 'item', 'blocks': 'block', 'entity_types': 'entity_type'}


def external(condition):
    """True when a material condition is about ingots another mod must supply."""
    text = json.dumps(condition)
    return 'mantle:tag_filled' in text or 'forge:mod_loaded' in text


def write_creative_parts(skipped):
    """creative_parts.rs: every tool part with the visible materials that have its stat type.

    Same selection and order as IMaterialItem.addVariants: visible (not hidden) concrete materials by
    tier, sort order, path and namespace, each kept when its stats include the part's stat type.
    """
    definitions = {}
    for path in (TARGET / 'materials/definition').glob('*.json'):
        data = json.loads(path.read_text(encoding='utf-8'))
        if 'redirect' in data or data.get('hidden', False):
            continue
        definitions[path.stem] = data
    order = sorted(definitions, key=lambda name: (definitions[name].get('tier', 0), definitions[name].get('sortOrder', 100), name, 'tconstruct'))
    stats = {name: set(json.loads((TARGET / 'materials/stats' / f'{name}.json').read_text(encoding='utf-8')).get('stats', {}))
             for name in order if (TARGET / 'materials/stats' / f'{name}.json').exists()}
    source = (ROOT / 'tconstruct/mixins/tconstruct/api/src/tool_parts.rs').read_text(encoding='utf-8')
    table = source.split('const PARTS')[1].split('];')[0]
    parts = re.findall(r'\("([a-z_]+)", "([a-z_]+)"\)', table)
    # RepairKitItem is a material item with its own stat type but is not a ToolPart.
    parts.append(('repair_kit', 'repair_kit'))
    lines = ['//! Tool parts with the visible shipped materials that can make them, in creative-tab order.',
             '//! Generated from the original data by tools/import_original_data.py; do not edit by hand.', '',
             'pub(crate) const PARTS: &[(&str, &[&str])] = &[']
    for item, stat in parts:
        materials = [name for name in order if f'tconstruct:{stat}' in stats.get(name, set())]
        lines.append(f'    ("{item}", &[' + ', '.join(f'"{name}"' for name in materials) + ']),')
    lines.append('];')
    (ROOT / 'tconstruct/mixins/tconstruct/api/src/creative_parts.rs').write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')


RECIPE_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/recipes/tools/materials'
RECIPE_TARGET = ROOT / 'tconstruct/content/data/tconstruct/recipes/tools/materials'


def copy_material_recipes(skipped_materials):
    """The item -> material recipes (type tconstruct:material), byte for byte. The fluid and melting
    recipes in the same folder belong to the smeltery domain. A recipe whose material was left out as
    other-mod integration is left out with it."""
    if RECIPE_TARGET.exists():
        shutil.rmtree(RECIPE_TARGET)
    count = 0
    for path in sorted(RECIPE_SOURCE.rglob('*.json')):
        data = json.loads(path.read_text(encoding='utf-8'))
        if data.get('type') != 'tconstruct:material':
            continue
        if data['material'].split(':')[-1].split('#')[0] in skipped_materials:
            continue
        destination = RECIPE_TARGET / path.relative_to(RECIPE_SOURCE)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, destination)
        count += 1
    return count


MODIFIER_RECIPE_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/recipes/tools/modifiers'
MODIFIER_RECIPE_TARGET = ROOT / 'tconstruct/content/data/tconstruct/recipes/tools/modifiers'


def copy_modifier_recipes():
    """The modifier and incremental-modifier station recipes, byte for byte. Other recipe types in that
    folder (salvage, swappable, ...) are separate recipe kinds and are not imported yet."""
    if MODIFIER_RECIPE_TARGET.exists():
        shutil.rmtree(MODIFIER_RECIPE_TARGET)
    count = 0
    for path in sorted(MODIFIER_RECIPE_SOURCE.rglob('*.json')):
        if json.loads(path.read_text(encoding='utf-8')).get('type') not in ('tconstruct:modifier', 'tconstruct:incremental_modifier', 'tconstruct:swappable_modifier', 'tconstruct:modifier_salvage', 'tconstruct:remove_modifier', 'tconstruct:multilevel_modifier', 'tconstruct:multilevel_incremental_modifier', 'tconstruct:overslime_modifier'):
            continue
        destination = MODIFIER_RECIPE_TARGET / path.relative_to(MODIFIER_RECIPE_SOURCE)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, destination)
        count += 1
    return count


PART_RECIPE_SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/recipes/tools/parts/builder'
PART_RECIPE_TARGET = ROOT / 'tconstruct/content/data/tconstruct/recipes/tools/parts/builder'


def copy_part_builder_recipes():
    """The part builder recipes that make tool parts from materials, byte for byte. The sand and
    red sand cast recipes belong to the smeltery and come with it."""
    if PART_RECIPE_TARGET.exists():
        shutil.rmtree(PART_RECIPE_TARGET)
    count = 0
    for path in sorted(PART_RECIPE_SOURCE.rglob('*.json')):
        if json.loads(path.read_text(encoding='utf-8')).get('type') != 'tconstruct:part_builder':
            continue
        destination = PART_RECIPE_TARGET / path.relative_to(PART_RECIPE_SOURCE)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, destination)
        count += 1
    return count


def copy_tags():
    """Item, block and entity-type tags. Forge's tag namespace does not exist in this host, so a
    required reference to a forge tag becomes an optional one (as the original already does for
    other mods' entries); everything else is kept in order, unchanged."""
    if TAG_TARGET.exists():
        shutil.rmtree(TAG_TARGET)
    count = 0
    for source_name, target_name in TAG_FOLDER_NAMES.items():
        for path in sorted((TAG_SOURCE / source_name).rglob('*.json')):
            data = json.loads(path.read_text(encoding='utf-8'))
            values = []
            for value in data.get('values', []):
                text = value if isinstance(value, str) else value.get('id', '')
                if isinstance(value, str) and text.lstrip('#').startswith('forge:'):
                    value = {'id': text, 'required': False}
                values.append(value)
            data['values'] = values
            destination = TAG_TARGET / target_name / path.relative_to(TAG_SOURCE / source_name)
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_text(json.dumps(data, indent=2) + '\n', encoding='utf-8', newline='\n')
            count += 1
    return count


def main():
    pins = json.loads((ROOT / 'tconstruct/compatibility.json').read_text(encoding='utf-8'))
    head = subprocess.run(['git', '-C', str(ROOT / '.local/tconstruct-source'), 'rev-parse', 'HEAD'],
                          capture_output=True, text=True, check=True).stdout.strip()
    expected = pins.get('tconstruct_revision') or pins.get('java_revision') or head
    if head != expected:
        raise SystemExit(f'original checkout {head} differs from the pinned {expected}')
    skipped_materials = set()
    for path in sorted((SOURCE / 'materials/definition').glob('*.json')):
        if 'condition' in json.loads(path.read_text(encoding='utf-8')) and external(json.loads(path.read_text(encoding='utf-8'))['condition']):
            skipped_materials.add(path.stem)
    if TARGET.exists():
        shutil.rmtree(TARGET)
    copied = 0
    for folder in FOLDERS + TAG_FOLDERS:
        for path in sorted((SOURCE / folder).rglob('*.json')):
            relative = path.relative_to(SOURCE)
            if relative.parts[0] == 'tool_definitions' and path.stem in EXCLUDED_TOOLS:
                continue
            if relative.parts[0] == 'materials' and path.stem in skipped_materials:
                continue
            destination = TARGET / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, destination)
            copied += 1
    types = sorted({module['type'] for path in (TARGET / 'modifiers').glob('*.json')
                    for module in json.loads(path.read_text(encoding='utf-8')).get('modules', []) or []})
    lines = ['//! Module type IDs the original ships in its modifier data (SlimeKnights, MIT; see LICENSE).',
             '//! A listed module that Tinkers has not ported is recorded as unbound, never evaluated; an ID',
             '//! outside this list is an unknown module and rejects the whole modifier definition.',
             '//! Generated from the pinned original data by tools/import_original_data.py.', '',
             'pub(crate) const KNOWN: &[&str] = &[']
    lines += [f'    "{kind}",' for kind in types] + ['];']
    (ROOT / 'tconstruct/mixins/tconstruct/api/src/modifier_module_types.rs').write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')
    write_creative_parts(skipped_materials)
    tags = copy_tags()
    recipes = copy_material_recipes(skipped_materials) + copy_modifier_recipes() + copy_part_builder_recipes()
    print(f'copied {copied} files, {tags} tags and {recipes} material recipes from {head}; {len(types)} modifier module types; excluded materials: {sorted(skipped_materials)}')


if __name__ == '__main__':
    sys.exit(main())
