"""Snapshot the source Smeltery controller, IO, tank and tag-required assets.

Usage: python import_smeltery_assets.py /path/to/TinkersConstruct
"""
import io
import subprocess
import sys
import zipfile
from pathlib import Path
import import_melter_assets as source

BLOCKS = ('smeltery_controller', 'seared_drain', 'seared_chute', 'seared_duct',
          'seared_ladder', 'seared_fuel_tank', 'seared_fuel_gauge',
          'seared_ingot_tank', 'seared_ingot_gauge', 'seared_tinted_glass',
          'foundry_controller', 'scorched_drain', 'scorched_chute', 'scorched_duct',
          'scorched_ladder', 'scorched_fuel_tank', 'scorched_fuel_gauge',
          'scorched_ingot_tank', 'scorched_ingot_gauge', 'scorched_tinted_glass')
OUTPUT = Path(__file__).resolve().parents[1] / 'content/reference/tconstruct-smeltery-assets.zip'


def snapshot(repo):
    old_blocks = source.BLOCKS
    try:
        source.BLOCKS = BLOCKS
        data = source.snapshot(repo)
    finally:
        source.BLOCKS = old_blocks
    result = io.BytesIO()
    with zipfile.ZipFile(io.BytesIO(data)) as original, zipfile.ZipFile(result, 'w', zipfile.ZIP_DEFLATED) as output:
        prefixes = ('assets/tconstruct/blockstates/', 'assets/tconstruct/models/',
                    'assets/tconstruct/textures/block/', 'assets/tconstruct/textures/item/')
        for name in sorted(original.namelist()):
            if not name.startswith(prefixes):
                continue
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            output.writestr(info, original.read(name))
        name = 'assets/tconstruct/textures/gui/heating_structure.png'
        path = 'src/main/resources/' + name
        data = subprocess.check_output(['git', '-C', str(repo), 'show', f'{source.REVISION}:{path}'])
        info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        output.writestr(info, data)
        for family in ('smeltery', 'foundry'):
            for suffix in ('u', 'd', 'l', 'r', 'ud', 'ul', 'ur', 'dl', 'dr', 'lr', 'udl', 'udr', 'ulr', 'dlr', 'udlr'):
                name = f'assets/tconstruct/textures/block/{family}/tinted_glass/{suffix}.png'
                data = subprocess.check_output(['git', '-C', str(repo), 'show', f'{source.REVISION}:src/main/resources/{name}'])
                info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
                info.compress_type = zipfile.ZIP_DEFLATED
                output.writestr(info, data)
        for name in ('assets/tconstruct/models/item/amethyst_bronze_ingot.json', 'assets/tconstruct/textures/item/materials/amethyst_bronze_ingot.png'):
            data = subprocess.check_output(['git', '-C', str(repo), 'show', f'{source.REVISION}:src/main/resources/{name}'])
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            output.writestr(info, data)
    return result.getvalue()


if __name__ == '__main__':
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    OUTPUT.write_bytes(snapshot(Path(sys.argv[1])))
    print(f'Wrote {OUTPUT}')
