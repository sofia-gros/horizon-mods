"""Snapshot source fluid-bucket models and derive icons from their source sprites.

Usage: python import_fluid_buckets.py /path/to/TinkersConstruct forge-bucket-mask.png
Requires Pillow only to regenerate this checked-in archive. The mask is Forge's
bucket_fluid_drip.png at commit 243490d78001917c7e6ee6e94b45ef98776aa36f.
"""
import hashlib
import io
import json
import subprocess
import sys
import zipfile
from pathlib import Path
from PIL import Image, ImageChops

ROOT = Path(__file__).resolve().parents[1]
SOURCE_REVISION = 'e14bb5f1794f561ff164f88f84b8b095191c8ecf'
FORGE_REVISION = '243490d78001917c7e6ee6e94b45ef98776aa36f'
MASK_SHA256 = '521f9f66e4cf2991af902a3fc52242e6eeae4f79d603cd6454c3b352cf9e533c'


def snapshot(repo, mask_path):
    mask_data = mask_path.read_bytes()
    if hashlib.sha256(mask_data).hexdigest() != MASK_SHA256:
        raise ValueError('Forge bucket mask differs from the pinned source')
    mask = Image.open(io.BytesIO(mask_data)).convert('RGBA').getchannel('A')
    if mask.size != (16, 16):
        raise ValueError('unexpected Forge bucket mask size')
    result = io.BytesIO()
    with zipfile.ZipFile(ROOT / 'content/reference/tconstruct-melter-assets.zip') as fluids, zipfile.ZipFile(result, 'w', zipfile.ZIP_DEFLATED) as out:
        mapping = json.loads(fluids.read('assets/tconstruct/melter-fluid-textures.json'))
        source_paths = set(subprocess.check_output(['git', '-C', str(repo), 'ls-tree', '-r', '--name-only', SOURCE_REVISION], text=True).splitlines())
        manifest = {}
        def emit(name, payload):
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0)); info.compress_type = zipfile.ZIP_DEFLATED
            out.writestr(info, payload)
        emit('reference/forge-bucket-fluid-drip.png', mask_data)
        for fluid, texture in sorted(mapping.items()):
            short = fluid.removeprefix('tconstruct:')
            name = short + '_bucket'
            source = 'src/generated/resources/assets/tconstruct/models/item/' + name + '.json'
            if source not in source_paths:
                continue
            model = subprocess.check_output(['git', '-C', str(repo), 'show', f'{SOURCE_REVISION}:{source}'])
            parsed = json.loads(model)
            if parsed.get('loader') != 'tconstruct:fluid_container' or parsed.get('fluid') != fluid:
                raise ValueError(f'unexpected bucket model: {source}')
            image = Image.open(io.BytesIO(fluids.read('assets/tconstruct/' + texture))).convert('RGBA')
            if image.width != 16 or image.height < 16:
                raise ValueError(f'unexpected fluid texture: {texture}')
            image = image.crop((0, 0, 16, 16))
            image.putalpha(ImageChops.multiply(image.getchannel('A'), mask))
            icon = io.BytesIO(); image.save(icon, format='PNG', optimize=False)
            emit('models/' + name + '.json', model)
            emit('textures/' + name + '.png', icon.getvalue())
            manifest[name] = {'fluid': fluid, 'source_texture': texture}
        emit('manifest.json', json.dumps({'tconstruct_revision': SOURCE_REVISION, 'forge_revision': FORGE_REVISION, 'buckets': manifest}, sort_keys=True, separators=(',', ':')).encode())
    return result.getvalue()


if __name__ == '__main__':
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    output = ROOT / 'content/reference/tconstruct-fluid-buckets.zip'
    output.write_bytes(snapshot(Path(sys.argv[1]), Path(sys.argv[2])))
    print(f'Wrote {output}')
