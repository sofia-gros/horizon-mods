"""Excluded caller-source proposal only. No subprocess, stage or file writes."""
import hashlib
import json
from pathlib import Path
import re
import tomllib

HERE = Path(__file__).resolve().parent
TARGET = 'crates/horizon-client/src/mod_content.rs'
HORIZON = 'f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd'
MIXINS = '5a64af938ec86b00d53d16a408aabc0038fb3e57'
OLD = b'    let out = roots.join(&stamp[..ROOT_NAME]);\n'
NEW = b'    let out = separate_root(roots.join(&stamp[..ROOT_NAME]), &stamp);\n'
BEGIN = b'\n// BEGIN EXCLUDED LANGUAGE ROOT KEY\n'
END = b'// END EXCLUDED LANGUAGE ROOT KEY\n'
LIMIT = 2 * 1024 * 1024

def sha(raw): return hashlib.sha256(raw).hexdigest()
def require(ok, reason):
    if not ok: raise ValueError(reason)

def read(root, path, digest):
    relative = Path(path)
    require(not relative.is_absolute() and '..' not in relative.parts, 'unsafe relative input')
    file = Path(root).absolute() / relative
    require(not any(p.is_symlink() for p in (file,*file.parents)), 'symlink input component')
    require(file.is_file() and file.stat().st_size <= LIMIT, 'input size/type')
    with file.open('rb') as handle: raw = handle.read(LIMIT+1)
    require(len(raw) <= LIMIT and sha(raw) == digest, 'input hash: '+path)
    return raw

def caller(native):
    start = b'pub fn resource_root(base: &Path, mods: &[ContentMod], cache: &Path) -> Result<PathBuf> {'
    end = b'\n}\n\n/// The length of a root\'s folder name:'
    require(native.count(start) == native.count(end) == 1, 'caller bounds')
    a = native.index(start); b = native.index(end,a)+len(b'\n}')
    return native[a:b]

def compose(official, repair):
    raw_config = (HERE/'inputs.json').read_bytes(); c = json.loads(raw_config)
    require(c['horizon'] == HORIZON and c['mixins'] == MIXINS and c['status'] == 'excluded source proposal only', 'scope')
    originals = {p:read(official,p,h) for p,h in c['official_sources'].items()}
    native = originals[TARGET]; fn = caller(native)
    require(sha(fn) == c['caller_sha256'] and native.count(OLD) == 1, 'caller source')
    require(b'const ROOT_NAME: usize = 16;' in native and native.count(b'const ROOT_NAME:') == 1, 'root width')
    helper = read(HERE,'root-store.rs',c['helper_sha256'])
    merge = read(repair,'merge.rs',c['merge_sha256'])
    policies = re.findall(rb'pub\(super\) const POLICY_ID: &str = "([^"]+)";',merge)
    require(policies == [c['policy_id'].encode()] and helper.count(b'"'+policies[0]+b'"') == 1, 'single policy source identity')
    manifest = read(HERE,'horizon-mixin.toml',c['manifest_sha256'])
    parsed = tomllib.loads(manifest.decode())
    require(set(parsed) == {'mod','compatibility','injection'} and len(parsed['injection']) == 1, 'one root hook only')
    require(parsed['injection'][0] == c['injection'], 'root hook/binding contract')
    require(parsed['mod'] == {'id':'pluto_language_root_probe','version':'0.0.0','priority':100} and parsed['compatibility'] == {'horizon_revision':HORIZON}, 'manifest owner')
    require(BEGIN not in native and b'fn separate_root' not in native, 'already composed')
    generated = native.replace(OLD,NEW,1) + BEGIN + helper + END
    require(inverse(generated,helper) == native, 'full native inverse')
    require(originals == {p:read(official,p,h) for p,h in c['official_sources'].items()} and raw_config == (HERE/'inputs.json').read_bytes(), 'source changed while composing')
    return generated, {'scope':'source-only root basename separation; not native stamp repair',
        'native_sha256':sha(native),'generated_sha256':sha(generated),'generated_bytes':len(generated),
        'full_native_inverse':True,'native_stamp_bytes_unchanged':True,
        'mixins_plan_accepted':False,'rust_executed':False,'writer_connected':False,
        'actual_cache_tested':False,'full_composition_tested':False,'game_ui_tested':False}

def inverse(generated,helper):
    tail=BEGIN+helper+END
    require(generated.endswith(tail) and generated.count(BEGIN)==1, 'helper inverse boundary')
    body=generated[:-len(tail)]
    require(body.count(NEW)==1 and OLD not in body, 'caller inverse seam')
    return body.replace(NEW,OLD,1)
