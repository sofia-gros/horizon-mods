"""Generate the supported pickaxe sprites from pinned TConstruct textures.

Usage: python3 tools/generate_pickaxe_appearance.py /path/to/TinkersConstruct
Requires Pillow. The source tree must be at the revision in upstream-lock.json.
"""
from pathlib import Path
import itertools, json, subprocess, sys
from PIL import Image

root = Path(__file__).resolve().parents[1]
upstream = Path(sys.argv[1])
revision = 'f4e77d43f39ba1047bf93ae816b7e1beaab514c2'
if subprocess.check_output(['git', '-C', str(upstream), 'rev-parse', 'HEAD'], text=True).strip() != revision:
    raise SystemExit('wrong upstream revision')
asset = root / "Tinkers' onstruct/assets/tconstruct"
source = upstream / 'src/main/resources/assets/tconstruct/textures/item/tool/pickaxe'
generated = upstream / 'src/generated/resources/assets/tconstruct/tinkering/materials'
output = asset / 'textures/item/pickaxe_visual'
output.mkdir(parents=True, exist_ok=True)
for material in ('wood', 'flint'):
    data = json.loads((generated / f'{material}.json').read_text())
    palette = [(int(k), tuple(bytes.fromhex(v)[1:])) for k, v in data['generator']['transformer']['color_mapping']['palette'].items()]
    # Match the material fallback order and the upstream grey-to-color mapping.
    for part, base in [('handle','handle_stick'), ('head','head' if material == 'wood' else 'head_rock'), ('broken','head_broken'), ('binding','binding_primitive' if material == 'wood' else 'binding_rock')]:
        image = Image.open(source / f'{base}.png').convert('RGBA')
        pixels = []
        for r,g,b,a in image.getdata():
            if a == 0:
                pixels.append((0,0,0,0)); continue
            grey = max(r,g,b)
            lower = max((p for p in palette if p[0] <= grey), key=lambda p:p[0])
            upper = min((p for p in palette if p[0] >= grey), key=lambda p:p[0])
            if lower[0] == upper[0]: color = lower[1]
            else:
                color = tuple(x + int((y-x)*(grey-lower[0])/(upper[0]-lower[0])) for x,y in zip(lower[1],upper[1]))
            pixels.append(tuple((c * original // grey if grey and original < grey else c) for c,original in zip(color,(r,g,b))) + (a,))
        image.putdata(pixels)
        image.save(output / f'{part}_{material}.png')
models = asset / 'models/item/pickaxe_visual'; models.mkdir(parents=True, exist_ok=True)
items = asset / 'items'; items.mkdir(parents=True, exist_ok=True)
rules = []
for head, handle, binding, broken in itertools.product(('wood','flint'), ('wood','flint'), ('wood','flint'), (False,True)):
    name = f'pickaxe_{head}_{handle}_{binding}_{"broken" if broken else "intact"}'
    textures = {'layer0': f'tconstruct:item/pickaxe_visual/handle_{handle}', 'layer1': f'tconstruct:item/pickaxe_visual/{"broken" if broken else "head"}_{head}', 'layer2': f'tconstruct:item/pickaxe_visual/binding_{binding}'}
    (models / f'{name}.json').write_text(json.dumps({'parent':'minecraft:item/handheld','textures':textures}, indent=2)+'\n')
    (items / f'{name}.json').write_text(json.dumps({'model':{'type':'minecraft:model','model':f'tconstruct:item/pickaxe_visual/{name}'}}, indent=2)+'\n')
    rules.append({'when':{'/schema':1,'/tool':'tconstruct:pickaxe','/materials/0':f'tconstruct:{head}','/materials/1':f'tconstruct:{handle}','/materials/2':f'tconstruct:{binding}','/broken':broken},'model':f'tconstruct:{name}'})
path = asset / 'horizon/item_appearances'; path.mkdir(parents=True, exist_ok=True)
(path / 'pickaxe.json').write_text(json.dumps({'item':'minecraft:wooden_pickaxe','path':['tconstruct','state_json'],'cases':rules}, indent=2)+'\n')
