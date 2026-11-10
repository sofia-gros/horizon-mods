#!/usr/bin/env python3
"""Copy pinned original End island references and wrap configured tree codecs.

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

    structure = json.loads((original / 'worldgen/structure/end_slime_island.json').read_text(encoding='utf-8'))
    if structure != json.loads((reference / 'worldgen/structure/end_slime_island.json').read_text(encoding='utf-8')):
        raise ValueError('curated island settings differ from the pinned generated data')
    put('worldgen/structure/end_slime_island.json', (json.dumps(structure, indent=2) + '\n').encode())
    for name in ('ender_slime_tree', 'ender_slime_tree_tall'):
        tree = json.loads((original / f'worldgen/configured_feature/{name}.json').read_text(encoding='utf-8'))
        if tree != json.loads((reference / f'worldgen/configured_feature/{name}.json').read_text(encoding='utf-8')):
            raise ValueError('curated Ender tree differs from the pinned generated data')
        wrapped = {'type': 'tconstruct:source_tree', 'config': tree}
        put(f'worldgen/feature/{name}.json', (json.dumps(wrapped, indent=2) + '\n').encode())
    for size in ('0x1x0', '2x2x4', '4x1x6', '8x1x11', '11x1x11'):
        data = (original / f'structures/islands/ender/{size}.nbt').read_bytes()
        if len(data) > 1024 * 1024:
            raise ValueError('unexpectedly large original island template')
        put(f'structures/islands/ender/{size}.nbt', data, reference)
    print(json.dumps({'original_revision': pin, 'outputs': outputs,
                      'scope': 'original reference palettes retained; runtime palettes still omit custom world fluids'}, indent=2))


if __name__ == '__main__':
    main()
