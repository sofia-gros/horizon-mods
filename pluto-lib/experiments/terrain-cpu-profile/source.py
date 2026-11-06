"""In-memory source extension only. Does not prepare files or launch tools."""
import hashlib
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
BASE = HERE.parent/'terrain-material-cpu'
MARKER = b'    // INSERT EXACT NATIVE BASELINE\n'
START = b'    // BEGIN EXCLUDED PROFILE EXTENSION\n'
END = b'    // END EXCLUDED PROFILE EXTENSION\n'

def sha(data):
    return hashlib.sha256(data).hexdigest()

def validate():
    config = json.loads((HERE/'inputs.json').read_text())
    for path, digest in config['source_sha256'].items():
        p = ROOT/path
        if p.is_symlink() or sha(p.read_bytes()) != digest:
            raise ValueError('Frozen input changed: '+path)
    if config['successful_fixture'] != '1ac143d840eaf3dde9e108ada3f5f2ec9f4c451f':
        raise ValueError('Changed successful baseline')
    return config

def extend(template, extension):
    if not extension or len(extension) > 32768: raise ValueError('Unbounded extension')
    if template.count(MARKER) != 1 or START in template or END in template:
        raise ValueError('Ambiguous original fixture')
    if START in extension or END in extension or MARKER in extension:
        raise ValueError('Ambiguous extension')
    result = template.replace(MARKER, START+extension+END+MARKER, 1)
    if remove(result) != template:
        raise ValueError('Original fixture not preserved')
    return result

def remove(extended):
    if extended.count(START) != 1 or extended.count(END) != 1:
        raise ValueError('Missing or repeated extension boundary')
    begin = extended.index(START)
    finish = extended.index(END)+len(END)
    if finish <= begin: raise ValueError('Reversed extension boundaries')
    return extended[:begin]+extended[finish:]

def render(official):
    """Reuse the exact old extraction, replace only its one template marker."""
    validate()
    if (official/'.horizon-revision').read_text().strip() != '624c4f6265c02296fbe8a80f029dd0d987fb94c0':
        raise ValueError('Wrong official source marker')
    spec = importlib.util.spec_from_file_location('frozen_material_prepare',BASE/'prepare.py')
    base = importlib.util.module_from_spec(spec);spec.loader.exec_module(base)
    generated, metadata = base.baseline(official)
    template = (BASE/'fixture.rs').read_bytes()
    # The old marker includes indentation; preserve that indentation in the
    # original mechanically generated body instead of formatting either source.
    old_marker = b'// INSERT EXACT NATIVE BASELINE'
    old_before, old_after = template.split(old_marker)
    if not generated.startswith(old_before) or not generated.endswith(old_after):
        raise ValueError('Old generator/template disagreement')
    native = generated[len(old_before):-len(old_after)]
    extended = extend(template,(HERE/'profile.rs').read_bytes())
    result = extended.replace(old_marker,native,1)
    if remove(result) != generated: raise ValueError('Native full-body oracle changed')
    return result, {'baseline_generated_sha256':sha(generated),'extended_sha256':sha(result),
                    'native_function_sha256':metadata['function_sha256']}
