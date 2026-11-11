#!/usr/bin/env python3
"""Copy pinned original island references and configured tree/fungus codecs.

Original NBT goes into reference/, not over the existing runtime palettes that
replace the unimplemented slime fluid with structure void. This tool does not
implement world fluids, natural starts or save loading. Different existing
outputs are never silently overwritten.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, required=True, help='read-only original Tinkers checkout')
    args = parser.parse_args()
    source = args.source.resolve()
    pin = json.loads((ROOT / 'tconstruct/compatibility.json').read_text(encoding='utf-8'))['tconstruct_revision']
    actual = subprocess.run(['git', '-C', str(source), 'rev-parse', 'HEAD'], capture_output=True, text=True, check=True).stdout.strip()
    if actual != pin:
        raise ValueError('original Tinkers source differs from the pinned revision')
    original = source / 'src/generated/resources/data/tconstruct'
    content = ROOT / 'tconstruct/content/data/tconstruct'
    reference = ROOT / 'tconstruct/content/reference/slime-islands/data/tconstruct'
    outputs = []

    def put(relative, data, destination_root=content):
        destination = destination_root / relative
        if destination.exists():
            if destination.read_bytes() != data:
                raise ValueError(f'refusing to overwrite different island data: {destination}')
        else:
            destination.parent.mkdir(parents=True, exist_ok=True)
            with destination.open('xb') as output:
                output.write(data)
        outputs.append({'path': str(destination.relative_to(ROOT)), 'sha256': hashlib.sha256(data).hexdigest()})

    for name in ('blood_island', 'clay_island', 'earth_slime_island', 'end_slime_island',
                 'ocean_skyslime_island', 'sky_slime_island'):
        relative = f'worldgen/structure/{name}.json'
        structure = json.loads((original / relative).read_text(encoding='utf-8'))
        curated = reference / relative
        if curated.exists() and structure != json.loads(curated.read_text(encoding='utf-8')):
            raise ValueError(f'curated island settings differ from the pinned generated data: {name}')
        data = (json.dumps(structure, indent=2) + '\n').encode()
        put(relative, data, reference)
        put(relative, data)
    for name in ('earth_slime_island_tree', 'sky_slime_island_tree', 'blood_slime_island_fungus'):
        relative = f'worldgen/configured_feature/{name}.json'
        configuration = json.loads((original / relative).read_text(encoding='utf-8'))
        data = (json.dumps(configuration, indent=2) + '\n').encode()
        put(relative, data, reference)
        # These codecs use the original config object directly, unlike the
        # Ender schema adapter below. Shipping data is not proof of dispatch.
        put(f'worldgen/feature/{name}.json', data)
    for name in ('ender_slime_tree', 'ender_slime_tree_tall'):
        tree = json.loads((original / f'worldgen/configured_feature/{name}.json').read_text(encoding='utf-8'))
        if tree != json.loads((reference / f'worldgen/configured_feature/{name}.json').read_text(encoding='utf-8')):
            raise ValueError('curated Ender tree differs from the pinned generated data')
        wrapped = {'type': 'tconstruct:source_tree', 'config': tree}
        put(f'worldgen/feature/{name}.json', (json.dumps(wrapped, indent=2) + '\n').encode())
    for kind in ('blood', 'dirt', 'earth', 'ender', 'sky'):
        for size in ('0x1x0', '2x2x4', '4x1x6', '8x1x11', '11x1x11'):
            relative = f'structures/islands/{kind}/{size}.nbt'
            path = original / relative
            if not path.is_file():
                # Dirt is the handwritten source template; the slime palettes
                # are generated from it. Both trees belong to the same pin.
                path = source / 'src/main/resources/data/tconstruct' / relative
            data = path.read_bytes()
            if len(data) > 1024 * 1024:
                raise ValueError('unexpectedly large original island template')
            put(relative, data, reference)
    print(json.dumps({'original_revision': pin, 'outputs': outputs,
                      'scope': 'original reference palettes retained; runtime palettes still omit custom world fluids'}, indent=2))


if __name__ == '__main__':
    main()
