#!/usr/bin/env python3
"""Exact placement boundaries plus an explicit, archive-bound crafting composition.

The public library remains usable alone. Full Tinkers admission is a distinct
contract, never a fallback inferred from missing helpers or registrations.
"""
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import stat
import sys
import tomllib
import zipfile
ROOT = Path(__file__).resolve().parents[1]
MOD_ID = 'pluto_native_creative'
TC = 'tconstruct_native'
HORIZON = '41f822b659789780f58d32c63ddfb6887ea1b8bf'
MIXINS = '5a64af938ec86b00d53d16a408aabc0038fb3e57'
BOOTSTRAP = {
    'horizon_client_lib::crafting::Recipes::empty': 'crafting-shared-empty-startup',
    'horizon_client_lib::crafting::Recipes::from_sources': 'crafting-shared-sources-startup',
}
SECTIONS = {'injection': ('injections', 'injection_id'), 'module': ('modules', 'module_id'),
            'state': ('states', 'state_id'), 'crate': ('crates', 'crate_id'), 'bridge': ('bridges', 'bridge_id')}
LIMIT = 64 * 1024**2

def digest(value): return hashlib.sha256(value).hexdigest()
def entries(): return json.loads((ROOT/'tools/recipe-placement-boundaries.json').read_bytes())['boundaries']
def bounded(path, limit=LIMIT):
    path=Path(path)
    if path.is_symlink() or not path.is_file() or path.stat().st_size > limit: raise ValueError('Unbounded/nonregular crafting input')
    with path.open('rb') as stream: raw=stream.read(limit+1)
    if len(raw)>limit: raise ValueError('Crafting input grew beyond limit')
    return raw

def check_order(plan, metadata):
    if plan.get('schema') != 6: raise ValueError('unreviewed placement plan schema')
    for meta in metadata:
        selected=[h for h in plan.get('injections',[]) if h.get('target')==meta['target']]
        if len(selected)!=1: raise ValueError('placement requires exclusive native target: '+meta['target'])
        hook=selected[0];owner=meta.get('mod_id',MOD_ID)
        if hook.get('mod_id')!=owner or hook.get('injection_id') not in (meta['injection_id'],owner+'-'+meta['injection_id']):
            raise ValueError('placement owner changed')
        if meta.get('function_fingerprint') is None: raise ValueError('Unobserved native fingerprint')
        for key in ('file','kind','cancellable','priority','bindings','function_fingerprint'):
            if hook.get(key)!=meta[key]: raise ValueError('placement target/phase fingerprint drift: '+key)
        for key in ('node','contract'):
            if key in meta and (meta[key] is None or hook.get(key)!=meta[key]): raise ValueError('Unobserved/drifted placement '+key)
        if meta['kind']!='HEAD': raise ValueError('placement must remain HEAD')
        if owner==MOD_ID:
            if meta['cancellable'] is not True: raise ValueError('placement veto must remain cancellable')
        elif (owner!=TC or BOOTSTRAP.get(meta['target'])!=meta['injection_id'] or
              meta['cancellable'] is not False or meta['bindings']!=[] or
              meta.get('contract')!={'output':'()','value':None,'operation':None}):
            raise ValueError('Only the exact noncancellable constructor initialization is admitted')
        for key in ('argument','constant_site','call_site','site_fingerprint','selector','ordinal','require'):
            if hook.get(key) is not None: raise ValueError('Unexpected placement site selector')

def verify_source(plan, workspace, metadata, helpers=None):
    helpers=ROOT/'mixins/pluto_native_creative' if helpers is None else Path(helpers)
    for meta in metadata:
        if digest(bounded(Path(workspace)/meta['file']))!=meta['file_sha256']: raise ValueError('pristine placement source drift')
        if plan.get('inputs',{}).get(meta['file'])!=meta['file_sha256']: raise ValueError('placement plan does not bind pristine source')
        path=ROOT.parent/meta['source_path'] if 'source_path' in meta else helpers/meta['helper']
        source=bounded(path)
        if digest(source)!=meta['helper_sha256']: raise ValueError('placement helper drift')
        hook=next(h for h in plan['injections'] if h['target']==meta['target'])
        if hook.get('helper_source','').encode()!=source: raise ValueError('placement plan helper differs')

def make_package_contract(mode, archives):
    """Record caller-admitted immutable archives; never infer or downgrade mode.

    The execution runner must first check its fixed package input hashes. This
    helper only records those same files for the guards' independent readback.
    """
    wanted={'standalone-pluto':{MOD_ID:'0.1.0'},'full-tconstruct':{MOD_ID:'0.1.0',TC:'0.5.0'}}.get(mode)
    if wanted is None or not isinstance(archives,dict) or set(archives)!=set(wanted): raise ValueError('Explicit package mode/archive set differs')
    result={'schema':1,'mode':mode,'horizon':HORIZON,'mixins':MIXINS,'packages':[]}
    import io
    for owner,version in wanted.items():
        path=Path(archives[owner])
        if not path.is_absolute(): raise ValueError('Actual archive absolute path required')
        raw=bounded(path,16*1024**2);name='mixins/'+owner+'/horizon-mixin.toml'
        with zipfile.ZipFile(io.BytesIO(raw)) as z:
            names=z.namelist()
            if len(names)!=len(set(names)) or [n for n in names if n.endswith('/horizon-mixin.toml')]!=[name]: raise ValueError('Archive manifest set differs')
            if z.getinfo(name).file_size>1024**2: raise ValueError('Manifest size budget')
            source=z.read(name);m=tomllib.loads(source.decode())
        if m.get('mod',{}).get('id')!=owner or m['mod'].get('version')!=version or m.get('compatibility',{}).get('horizon_revision')!=HORIZON: raise ValueError('Manifest identity/revision differs')
        result['packages'].append({'mod_id':owner,'version':version,'archive':str(path),'archive_sha256':digest(raw),'manifest':name,'manifest_sha256':digest(source)})
    return result

def package_mode(plan, contract):
    if not isinstance(contract,dict) or set(contract)!={'schema','mode','horizon','mixins','packages'} or contract['schema']!=1:
        raise ValueError('Explicit crafting package contract required')
    if contract['horizon']!=HORIZON or contract['mixins']!=MIXINS: raise ValueError('Crafting package revision differs')
    wanted={'standalone-pluto':{MOD_ID:'0.1.0'},'full-tconstruct':{MOD_ID:'0.1.0',TC:'0.5.0'}}.get(contract['mode'])
    if wanted is None: raise ValueError('Unknown explicit crafting mode')
    packages=contract['packages']
    if not isinstance(packages,list) or len(packages)!=len(wanted) or {p.get('mod_id') for p in packages}!=set(wanted):
        raise ValueError('Declared package set differs')
    observed=set()
    for section in ('injections','modules','states','bridges','crates','data'):
        for row in plan.get(section,[]):
            owner=row.get('mod_id');observed.add(owner)
            if owner not in wanted or ('mod_version' in row and row['mod_version']!=wanted[owner]): raise ValueError('Plan owner/version outside declared packages')
    if observed!=set(wanted): raise ValueError('Plan omitted a declared package; no standalone fallback')
    for item in packages:
        if set(item)!={'mod_id','version','archive','archive_sha256','manifest','manifest_sha256'} or item['version']!=wanted[item['mod_id']]:
            raise ValueError('Crafting package fields differ')
        for key in ('archive_sha256','manifest_sha256'):
            if not isinstance(item[key],str) or re.fullmatch('[0-9a-f]{64}',item[key]) is None: raise ValueError('Unobserved package digest')
        archive=Path(item['archive'])
        if not archive.is_absolute(): raise ValueError('Actual archive absolute path required')
        raw=bounded(archive,16*1024**2)
        if digest(raw)!=item['archive_sha256']: raise ValueError('Actual package archive changed')
        import io
        with zipfile.ZipFile(io.BytesIO(raw)) as z:
            infos=z.infolist();names=[i.filename for i in infos]
            if len(names)>512 or len(names)!=len(set(names)) or sum(i.file_size for i in infos)>LIMIT: raise ValueError('Package size/member set invalid')
            for info in infos:
                parts=PurePosixPath(info.filename).parts
                if (not parts or info.filename.startswith('/') or '..' in parts or '\\' in info.filename or
                    stat.S_ISLNK(info.external_attr>>16) or info.is_dir() or info.flag_bits&1): raise ValueError('Unsupported package member')
            manifests=[n for n in names if n.endswith('/horizon-mixin.toml')]
            if manifests!=[item['manifest']]: raise ValueError('Package manifest set differs')
            if z.getinfo(item['manifest']).file_size>1024**2: raise ValueError('Manifest size budget')
            source=z.read(item['manifest'])
            if digest(source)!=item['manifest_sha256']: raise ValueError('Actual package manifest changed')
            declaration=tomllib.loads(source.decode())
            native=json.loads(z.read('horizon-native.json'))
            prefix=PurePosixPath(item['manifest']).parent
            for section,source_key,plan_key,id_key,body_key in [
                ('module','source','modules','module_id','source'),
                ('injection','helper','injections','injection_id','helper_source')]:
                for declared in declaration.get(section,[]):
                    rel=PurePosixPath(declared[source_key])
                    if rel.is_absolute() or '..' in rel.parts or '\\' in declared[source_key]: raise ValueError('Unsafe declared source path')
                    rows=[x for x in plan.get(plan_key,[]) if x.get('mod_id')==item['mod_id'] and x.get(id_key)==declared['id']]
                    if len(rows)!=1 or rows[0].get(body_key,'').encode()!=z.read((prefix/rel).as_posix()): raise ValueError('Package/plan body differs')
            for declared in declaration.get('crate',[]):
                rel=PurePosixPath(declared['path'])
                if rel.is_absolute() or '..' in rel.parts: raise ValueError('Unsafe declared crate path')
                start=(prefix/rel).as_posix()+'/'
                files={n[len(start):]:digest(z.read(n)) for n in names if n.startswith(start)}
                rows=[x for x in plan.get('crates',[]) if x.get('mod_id')==item['mod_id'] and x.get('crate_id')==declared['id']]
                if len(rows)!=1 or files!=rows[0].get('files'): raise ValueError('Package/plan crate files differ')
        if declaration.get('mod',{}).get('id')!=item['mod_id'] or declaration['mod'].get('version')!=item['version']:
            raise ValueError('Manifest identity differs')
        if native.get('id')!=item['mod_id'] or native.get('version')!=item['version']: raise ValueError('Native package identity differs')
        if declaration.get('compatibility',{}).get('horizon_revision')!=HORIZON: raise ValueError('Manifest official pin differs')
        for source_key,(plan_key,id_key) in SECTIONS.items():
            ids=[x['id'] for x in declaration.get(source_key,[])]
            rows=[x[id_key] for x in plan.get(plan_key,[]) if x.get('mod_id')==item['mod_id']]
            if len(ids)!=len(set(ids)) or len(rows)!=len(set(rows)) or set(rows)!=set(ids): raise ValueError('Declared '+source_key+' owner/ID set differs')
    return contract['mode']

def verify_closure(plan, workspace, mode, metadata=None):
    c=json.loads((ROOT/'tools/crafting-composition-closure.json').read_bytes()) if metadata is None else metadata
    if c['schema']!=1 or mode not in c['modes']: raise ValueError('Unknown crafting source closure')
    chosen=c['modes'][mode];inputs=plan.get('inputs',{})
    for path,h in chosen['native_files'].items():
        if inputs.get(path)!=h or digest(bounded(Path(workspace)/path))!=h: raise ValueError('Crafting dependency native source drift')
    for meta in chosen['modules']:
        selected=[m for m in plan.get('modules',[]) if m.get('file')==meta['file'] and m.get('name')==meta['name']]
        if len(selected)!=1: raise ValueError('Missing/duplicate crafting dependency module')
        actual=selected[0]
        for key in ('mod_id','mod_version','module_id','file','name','visibility'):
            if actual.get(key)!=meta.get(key): raise ValueError('Crafting dependency module identity differs')
        if digest(actual.get('source','').encode())!=meta['source_sha256']: raise ValueError('Crafting dependency module source differs')
    hooks=[h for h in plan.get('injections',[]) if h.get('file') in c['exclusive_module_files']]
    if len(hooks)!=len(chosen['hook_targets']) or {h.get('target') for h in hooks}!=set(chosen['hook_targets']): raise ValueError('Unknown/missing crafting entry hook; constructor bypass not admitted')
    expected={(m['file'],m['name']) for m in chosen['modules'] if m['file'] in c['exclusive_module_files']}
    actual={(m.get('file'),m.get('name')) for m in plan.get('modules',[]) if m.get('file') in c['exclusive_module_files']}
    if actual!=expected: raise ValueError('Unreviewed sibling crafting module')
    for section in ('bridges','states'):
        if any(m.get('file') in c['exclusive_module_files'] for m in plan.get(section,[])): raise ValueError('Unreviewed crafting state or bridge')
    expected_states=chosen['states']
    for expected in expected_states:
        rows=[s for s in plan.get('states',[]) if s.get('mod_id')==expected['mod_id'] and s.get('state_id')==expected['state_id']]
        if rows!=[expected]: raise ValueError('Station session state/constructor drift')
    api=[x for x in plan.get('crates',[]) if x.get('package')=='pluto-inventory-api']
    if len(api)!=1 or api[0]!=c['inventory_crate']: raise ValueError('Crafting requires the one exact public inventory API crate')
    # Retain the independent guest handler without claiming its live behavior.
    for expected in chosen['retained_injections']:
        rows=[h for h in plan.get('injections',[]) if h.get('target')==expected['target']]
        if rows!=[expected]: raise ValueError('Retained LAN station hook changed')

def read_contract(path):
    if path is None: raise ValueError('Explicit crafting package contract argument required')
    return json.loads(bounded(path,1024**2))

def check(plan, workspace, package_contract=None):
    mode=package_mode(plan,read_contract(package_contract))
    meta=entries();check_order(plan,meta);verify_source(plan,workspace,meta)
    verify_closure(plan,workspace,mode)

def main(argv=None):
    args=sys.argv[1:] if argv is None else argv
    try:
        if len(args)!=3: raise ValueError('usage: check_recipe_placement_plan.py COMPLETE_PLAN.json PRISTINE_OFFICIAL_WORKSPACE CRAFTING_PACKAGE_CONTRACT.json')
        check(json.loads(Path(args[0]).read_text()),Path(args[1]),Path(args[2]))
    except (ValueError,OSError,KeyError,TypeError,StopIteration,zipfile.BadZipFile) as error:
        print('Recipe placement composition rejected: '+str(error),file=sys.stderr);return 1
    print('Recipe placement owner, delegation, native sources and explicit package closure verified.');return 0
if __name__=='__main__':raise SystemExit(main())
