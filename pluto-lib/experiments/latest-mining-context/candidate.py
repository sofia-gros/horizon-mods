"""Unapplied single native-phase adapter; never edits main or the original archives."""
import base64,hashlib,json,tomllib,zlib
from pathlib import Path
HERE=Path(__file__).resolve().parent
MAIN='08afa992d5091e03360f3473ea93fd17aa793987'
OLD='0c7b5b61a5e746d621afffba919b12aeccf9c0fa'
LATEST='f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd'
REVISION_ONLY_SHA='5fc01ef8d392bb0b38c6f7d1ee8bca5ba115ca57e8cd0193c104a495c2c9cfe6'
REVISION_ONLY_MEMBERS='692fa172e45e00dcddf62d201c6dbf206702f16e8b06a7109c8e6b6c13f0c125'
MANIFEST='tconstruct_native/horizon-mixin.toml'
HELPER='tconstruct_native/source-mining-context-native.rs'
COMMON_MANIFEST='pluto_native_creative/horizon-mixin.toml'
COMMON_SOURCE='pluto_native_creative/mining-context.rs'
COMMON_REVISION_ONLY_SHA='19887c15e3a8a9a2b9ac2e5f6c8042004f46a0ad53628cb7fa142be5fe829110'
COMMON_NEW_BLOCK=b'''\n[[module]]
id = "mining-context-facts"
file = "crates/horizon-client/src/game.rs"
name = "pluto_mining_context"
source = "mining-context.rs"
visibility = "pub(crate)"
'''

OLD_IDS=('source-host-mining-context','source-client-mining-context')
NEW_ID='source-native-mining-context'
NEW_BLOCK='''[[injection]]
id = "source-native-mining-context"
target = "horizon_client::game::native_hooks::Game<'_>::native_break_speed"
kind = "HEAD"
helper = "source-mining-context-native.rs"
bindings = [
  { name = "game", ty = "&crate::game::Game<'_>", value = "self" },
  { name = "pos", ty = "[i32; 3]", value = "pos" },
  { name = "state", ty = "horizon_world::registry::BlockStateId", value = "state" },
  { name = "speed", ty = "&mut f32", value = "&mut *speed" },
]

'''.encode()
sha=lambda b:hashlib.sha256(b).hexdigest()
def oracles():return json.loads((HERE/'oracles.json').read_text())
def original_manifest():
    o=oracles();b=zlib.decompress(base64.b64decode(o['manifest_zlib_base64'],validate=True))
    if sha(b)!=o['manifest_sha256'] or o['main']!=MAIN:raise ValueError('Original main manifest drift')
    # The existing main packager renames its native id/accessors; preserve that exact transform.
    b=b.replace(b'__horizon_mixins_tconstruct_',b'__horizon_mixins_tconstruct_native_')
    b=b.replace(b'id = "tconstruct"',b'id = "tconstruct_native"',1)
    if sha(b)!=o['packaged_manifest_sha256']:raise ValueError('Packaged main manifest drift')
    return b

def revision_only_manifest():
    b=original_manifest();before=f'horizon_revision = "{OLD}"'.encode();after=f'horizon_revision = "{LATEST}"'.encode()
    if b.count(before)!=1:raise ValueError('Original revision is not unique')
    b=b.replace(before,after,1)
    if sha(b)!=REVISION_ONLY_SHA:raise ValueError('Revision-only manifest drift')
    return b

def transform_manifest(before):
    if sha(before)!=REVISION_ONLY_SHA:raise ValueError('Wrong revision-only manifest')
    old=oracles()['old_hook_blocks'].encode()
    if before.count(old)!=1 or NEW_ID.encode() in before:raise ValueError('Ambiguous old hooks')
    after=before.replace(old,NEW_BLOCK,1)
    expected=tomllib.loads(before.decode());actual=tomllib.loads(after.decode())
    hooks=expected['injection'];indexes=[i for i,x in enumerate(hooks) if x['id'] in OLD_IDS]
    if len(indexes)!=2 or indexes[1]!=indexes[0]+1:raise ValueError('Old hook positions changed')
    if [hooks[i]['id'] for i in indexes]!=list(OLD_IDS):raise ValueError('Old hook order changed')
    hooks[indexes[0]:indexes[1]+1]=tomllib.loads(NEW_BLOCK.decode())['injection']
    if actual!=expected:raise ValueError('Non-hook semantic change')
    if after.count(NEW_BLOCK)!=1 or after.replace(NEW_BLOCK,old,1)!=before:raise ValueError('Manifest inverse failed')
    return after

def common_revision_only_manifest():
    o=oracles();b=zlib.decompress(base64.b64decode(o['common_manifest_zlib_base64'],validate=True))
    if sha(b)!=o['common_manifest_sha256']:raise ValueError('Original common manifest drift')
    old=b'horizon_revision = "624c4f6265c02296fbe8a80f029dd0d987fb94c0"'
    new=f'horizon_revision = "{LATEST}"'.encode()
    if b.count(old)!=1:raise ValueError('Common revision is not unique')
    b=b.replace(old,new,1)
    if sha(b)!=COMMON_REVISION_ONLY_SHA:raise ValueError('Common revision-only drift')
    return b

def transform_common_manifest(before):
    if sha(before)!=COMMON_REVISION_ONLY_SHA:raise ValueError('Wrong common revision-only manifest')
    if not before.endswith(b'\n') or b'pluto_mining_context' in before:raise ValueError('Common module collision')
    after=before+COMMON_NEW_BLOCK
    expected=tomllib.loads(before.decode());actual=tomllib.loads(after.decode())
    addition=tomllib.loads(COMMON_NEW_BLOCK.decode())['module']
    if any(x['id']==addition[0]['id'] or (x['file'],x['name'])==(addition[0]['file'],addition[0]['name']) for x in expected['module']):
        raise ValueError('Common module owner collision')
    expected['module']+=addition
    if expected!=actual or after[:-len(COMMON_NEW_BLOCK)]!=before:raise ValueError('Common manifest inverse failed')
    return after

def transform_payload(payload):
    """Pure bytes-in/bytes-out. The caller owns a separate output directory and its seals."""
    if not isinstance(payload,dict) or any(not isinstance(p,str) or not isinstance(b,bytes) for p,b in payload.items()):
        raise ValueError('Expected path to immutable bytes map')
    before={p:sha(b) for p,b in sorted(payload.items())}
    if len(before)!=644 or sha(json.dumps(before,sort_keys=True).encode())!=REVISION_ONLY_MEMBERS:
        raise ValueError('Wrong full revision-only payload')
    if HELPER in payload or COMMON_SOURCE in payload:raise ValueError('Candidate source already present')
    for p,fact in oracles()['preserved_policy'].items():
        b=payload['tconstruct_native/'+p]
        if len(b)!=fact['packaged_bytes'] or sha(b)!=fact['packaged_sha256']:raise ValueError('Consumer policy drift')
    helper=(HERE/'source-mining-context-native.rs').read_bytes()
    if sha(helper)!=json.loads((HERE/'inputs.json').read_text())['candidate_helper_sha256']:
        raise ValueError('Candidate helper drift')
    private=(HERE/'mining-context.rs').read_bytes()
    if sha(private)!=json.loads((HERE/'inputs.json').read_text())['private_adapter_sha256']:raise ValueError('Private adapter drift')
    for path,body in payload.items():
        if path.endswith('/horizon-mixin.toml'):
            for module in tomllib.loads(body.decode()).get('module',[]):
                if module['file']=='crates/horizon-client/src/game.rs' and module['name']=='pluto_mining_context':
                    raise ValueError('Another payload owns the private native module')
    result=payload.copy();result[MANIFEST]=transform_manifest(payload[MANIFEST]);result[HELPER]=helper
    result[COMMON_MANIFEST]=transform_common_manifest(payload[COMMON_MANIFEST]);result[COMMON_SOURCE]=private
    if len(result)!=646 or {p for p in payload if payload[p]!=result[p]}!={MANIFEST,COMMON_MANIFEST}:raise ValueError('Unexpected payload changes')
    inverse=result.copy();del inverse[HELPER];del inverse[COMMON_SOURCE]
    inverse[COMMON_MANIFEST]=inverse[COMMON_MANIFEST][:-len(COMMON_NEW_BLOCK)]
    inverse[MANIFEST]=inverse[MANIFEST].replace(NEW_BLOCK,oracles()['old_hook_blocks'].encode(),1)
    if inverse!=payload:raise ValueError('Full payload inverse failed')
    return result

def verify_official(root):
    result=[]
    for p,want in json.loads((HERE/'inputs.json').read_text())['source_inputs'].items():
        f=Path(root)/p
        if f.is_symlink() or not f.is_file():raise ValueError('Nonregular official input')
        b=f.read_bytes();blob=hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()
        if len(b)!=want['bytes'] or sha(b)!=want['sha256'] or blob!=want['git_blob_sha']:
            raise ValueError('Official source drift: '+p)
        result.append(p)
    return result
