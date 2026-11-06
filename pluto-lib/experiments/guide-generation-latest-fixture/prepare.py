"""Source preparation only. The outer controller owns Git/Cargo and process groups."""
import hashlib
import json
from pathlib import Path
import tomllib

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
HORIZON = 'f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd'
API = 'pluto-lib/mixins/pluto_native_creative/inventory-api'
LOCAL = {('pluto-guide-generation-tests', '0.0.0', None, None), ('pluto-inventory-api', '0.1.0', None, None)}


def sha(raw): return hashlib.sha256(raw).hexdigest()
def require(ok, message):
    if not ok: raise ValueError(message)


def files(root):
    result = {}
    for p in root.rglob('*'):
        require(not p.is_symlink(), 'symlink in source: '+str(p))
        if p.is_file(): result[str(p.relative_to(root))] = sha(p.read_bytes())
    return result


def read(root, name):
    p = root/name
    require(not p.is_symlink() and p.is_file() and p.resolve().is_relative_to(root.resolve()), 'nonregular or escaping input: '+name)
    return p.read_bytes()


def config():
    value = json.loads((HERE/'inputs.json').read_text())
    require(value['official_revision']==HORIZON, 'wrong official revision')
    for name,digest in value['preparer_sha256'].items():
        require(sha(read(HERE,name))==digest,'preparer source changed: '+name)
    for name,digest in value['source_sha256'].items():
        require(sha(read(ROOT,name))==digest, 'changed Mod input: '+name)
    require(files(ROOT/API)==value['api_sha256'], 'complete actual API differs')
    return value


def manifest(host):
    native = tomllib.loads(read(host,'Cargo.toml').decode())
    patches = native['patch']['crates-io']
    require(set(patches)=={'wgpu-hal','naga','libmimalloc-sys'}, 'official patch set changed')
    result = '[package]\nname="pluto-guide-generation-tests"\nversion="0.0.0"\nedition="2024"\n[workspace]\nexclude=["inventory-api"]\ndefault-members=["."]\n[lib]\npath="lib.rs"\n[dependencies]\npluto-inventory-api={path="inventory-api"}\n'
    for name in ['horizon-client-core','horizon-client-render']:
        result += name+'={path='+json.dumps(str(host/'crates'/name))+',default-features=false}\n'
    result += '[patch.crates-io]\n'
    for name,item in patches.items():
        require(set(item)=={'path'}, 'unsupported official patch')
        path = (host/item['path']).resolve()
        require(path.is_relative_to(host.resolve()), 'escaping official patch')
        result += json.dumps(name)+'={path='+json.dumps(str(path))+'}\n'
    return result


LIB = '''pub use horizon_client_core::text;
pub use horizon_client_render::font;
pub use pluto_inventory_api as pluto_inventory;
#[path="guide-text.rs"] pub mod guide_text;
pub use guide_text as pluto_guide_text;
#[path="guide-layout.rs"] pub mod guide_layout;
pub use guide_layout as pluto_guide_layout;
extern crate self as horizon_client;
#[path="consumer.rs"] mod consumer;
#[cfg(test)] #[path="native/guide-generation-query/native.rs"] mod consumer_tests;
'''
COPIES = {
    'guide-text.rs':'pluto-lib/mixins/pluto_native_creative/guide-text.rs',
    'guide-layout.rs':'pluto-lib/mixins/pluto_native_creative/guide-layout.rs',
    'consumer.rs':'tconstruct/mixins/tconstruct/source-materials-book-text.rs',
    'native/guide-generation-query/native.rs':'tconstruct/tests/guide-generation-query/native.rs',
    'native/guide-text-migration/native.rs':'tconstruct/tests/guide-text-migration/native.rs',
    'native/guide-text-migration/guide_font_provider.json':'pluto-lib/tests/native/guide_font_provider.json',
}


def check_official_inputs(host, value):
    for name,digest in value['official_sha256'].items():
        require(sha(read(host,name))==digest, 'changed official input: '+name)


def prepare(host, output):
    value=config();host=host.resolve();output=output.absolute()
    require(not output.exists() and not output.is_symlink(), 'output already exists')
    require(not output.resolve().is_relative_to(host) and not host.is_relative_to(output.resolve()), 'overlapping host/output')
    require(not output.resolve().is_relative_to(ROOT.resolve()), 'output inside Mod source')
    check_official_inputs(host,value)
    contents={name:read(ROOT,origin) for name,origin in COPIES.items()}
    contents.update({'inventory-api/'+name:read(ROOT/API,name) for name in value['api_sha256']})
    contents['lib.rs']=LIB.encode();contents['Cargo.toml']=manifest(host).encode()
    # Atomic create rejects a preexisting directory or symlink. The controller
    # creates and seals its parent, and must reject concurrent external writers.
    output.mkdir()
    for name,raw in contents.items():
        path=output/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(raw)
    (output/'Cargo.lock').write_bytes(read(host,'Cargo.lock'))
    evidence={'official_root':str(host),'official_revision':HORIZON,
              'source_sha256':{name:sha(raw) for name,raw in contents.items()},
              'seed_lock_sha256':sha(read(host,'Cargo.lock')),
              'scope':'Exact production helper/API and actual latest Font/Text, six generation CPU tests. No Game, GPU, UI, LAN or installed package.'}
    (output/'fixture-sources.json').write_text(json.dumps(evidence,indent=2)+'\n')
    return evidence


def identity(package):
    return tuple(package.get(k) for k in ['name','version','source','checksum'])


def verify_workspace(output, metadata):
    # Exclusion keeps the copied API a dependency, not another test member.
    own=[p['id'] for p in metadata['packages'] if p['name']=='pluto-guide-generation-tests']
    require(len(own)==1 and metadata['workspace_members']==own,'wrong fixture workspace')
    require(metadata['workspace_default_members']==own,'wrong fixture default members')
    require(Path(metadata['workspace_root']).resolve()==output.resolve(),'wrong fixture workspace root')


def verify(output, metadata):
    value=config();evidence=json.loads(read(output,'fixture-sources.json'))
    host=Path(evidence['official_root']);check_official_inputs(host,value)
    require(evidence['official_revision']==HORIZON, 'wrong fixture target')
    source_names=set(COPIES)|{'inventory-api/'+n for n in value['api_sha256']}|{'lib.rs','Cargo.toml'}
    require(set(evidence['source_sha256'])==source_names, 'receipt source set changed')
    expected=source_names|{'Cargo.lock','fixture-sources.json'}
    actual=files(output)
    require(set(actual)==expected, 'fixture file set changed')
    for name,digest in evidence['source_sha256'].items():
        require(actual[name]==digest, 'fixture source drift: '+name)
    for name,origin in COPIES.items():require(read(output,name)==read(ROOT,origin),'copied source differs')
    require(files(output/'inventory-api')==value['api_sha256'],'copied API differs')
    require(read(output,'lib.rs')==LIB.encode() and read(output,'Cargo.toml')==manifest(host).encode(),'fixture wiring changed')
    allowed={identity(p) for p in tomllib.loads(read(host,'Cargo.lock').decode())['package']}
    lock={identity(p) for p in tomllib.loads(read(output,'Cargo.lock').decode())['package']}
    require(not lock-allowed-LOCAL and LOCAL<=lock,'lock identity drift')
    require(len(lock)==len(tomllib.loads(read(output,'Cargo.lock').decode())['package']),'duplicate lock package')
    require(metadata['version']==1,'unexpected metadata version')
    local={'pluto-guide-generation-tests':output/'Cargo.toml','pluto-inventory-api':output/'inventory-api/Cargo.toml'}
    found=set();package_ids=[]
    for package in metadata['packages']:
        path=Path(package['manifest_path']).resolve();name=package['name'];package_ids.append(package['id'])
        require((name,package['version'],package['source']) in {p[:3] for p in lock},'metadata outside adapted lock')
        if name in local:
            require(package['source'] is None and path==local[name].resolve(),'wrong fixture/API owner')
            found.add(name)
        elif package['source'] is None:
            require(path.is_relative_to(host.resolve()),'external local dependency')
    require(found==set(local),'missing actual fixture/API')
    require(len(set(package_ids))==len(package_ids),'duplicate metadata package')
    verify_workspace(output, metadata)
    for name in ['horizon-client-core','horizon-client-render']:
        candidates=[p for p in metadata['packages'] if p['name']==name]
        require(len(candidates)==1 and candidates[0]['source'] is None and Path(candidates[0]['manifest_path']).resolve()==host/'crates'/name/'Cargo.toml','wrong actual native dependency')
    return evidence
