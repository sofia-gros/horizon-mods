#!/usr/bin/env python3
"""Copy the original mod's generated tinkering data into the clean content root.

Source: the read-only checkout at .local/tconstruct-source (revision pinned by
tconstruct/compatibility.json). Files are copied byte for byte, never edited, so the content
root stays a mechanical projection of the original. Other-mod integration is left out:
the minotaur axe and every material whose condition depends on external ingot tags.
"""
import json
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / '.local/tconstruct-source/src/generated/resources/data/tconstruct/tinkering'
TARGET = ROOT / 'tconstruct/content/data/tconstruct/tinkering'
FOLDERS = ['tool_definitions', 'materials/definition', 'materials/stats', 'materials/traits', 'modifiers']
TAG_FOLDERS = ['tags/materials', 'tags/modifiers']
EXCLUDED_TOOLS = {'minotaur_axe'}


def external(condition):
    """True when a material condition is about ingots another mod must supply."""
    text = json.dumps(condition)
    return 'mantle:tag_filled' in text or 'forge:mod_loaded' in text


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
    print(f'copied {copied} files from {head}; {len(types)} modifier module types; excluded materials: {sorted(skipped_materials)}')


if __name__ == '__main__':
    sys.exit(main())
