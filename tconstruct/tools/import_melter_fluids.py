"""Extract original tank-fluid identifiers from the retained source tags."""
import json
import re
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ARCHIVE = ROOT / 'content/reference/tconstruct-fluid-tags.zip'
OUTPUT = ROOT / 'content/reference/tconstruct-melter-fluids.json'


def fluid_ids():
    result = set()
    with zipfile.ZipFile(ARCHIVE) as source:
        for name in source.namelist():
            value = json.loads(source.read(name))
            for raw in value.get('values', []):
                ident = raw if isinstance(raw, str) else raw.get('id', '')
                if ident.startswith('tconstruct:'):
                    if not re.fullmatch(r'tconstruct:[a-z0-9_./-]+', ident):
                        raise ValueError(f'invalid fluid ID: {ident}')
                    result.add(ident)
    return sorted(result)


if __name__ == '__main__':
    OUTPUT.write_text(json.dumps(fluid_ids(), indent=2) + '\n')
