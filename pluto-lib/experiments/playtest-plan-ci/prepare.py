"""Fixed integration packaging and bounded complete-plan inputs; no repinning or apply."""
import hashlib, importlib.util, io, json, os, stat, subprocess, tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
HORIZON = '41f822b659789780f58d32c63ddfb6887ea1b8bf'
MIXINS = '5a64af938ec86b00d53d16a408aabc0038fb3e57'
COMPONENTS = ('pluto_native_creative','pluto_transport_3a','pluto_tool_rules',
              'pluto_component_display_3a','pluto_component_bar','pluto_items_b2')
MAX_ARCHIVE = 64 * 1024**2
MAX_UNPACKED = 64 * 1024**2
MAX_ENTRIES = 512
MAX_PLAN = 16 * 1024**2
sha = lambda b: hashlib.sha256(b).hexdigest()

def load(name, path):
    spec=importlib.util.spec_from_file_location(name,path);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module

old = load('playtest_old_package_reader', HERE.parent/'main-common-ci/prepare.py')
members = old.members
git_state = old.git_state
write_wrappers = old.write_wrappers
allowed = old.allowed
diagnostic = old.diagnostic

def regular(path, limit):
    path=Path(path)
    if any(p.is_symlink() for p in [path,*path.parents]):raise ValueError('Symlink input')
    st=path.lstat()
    if not stat.S_ISREG(st.st_mode) or not 0 < st.st_size <= limit:raise ValueError('Nonregular or oversized input')
    raw=path.read_bytes()
    if len(raw)!=st.st_size:raise ValueError('Input changed while reading')
    return raw

def plan_bytes(path):return regular(path, MAX_PLAN)

def validate_headers(root):
    """Refuse mixed pins before packaging; this function never edits any header."""
    root=Path(root)
    tc=tomllib.loads((root/'tconstruct/mixins/tconstruct/horizon-mixin.toml').read_text())
    compatibility=json.loads((root/'tconstruct/compatibility.json').read_text())
    if tc['mod']['id']!='tconstruct' or tc['compatibility']['horizon_revision']!=HORIZON:
        raise ValueError('TConstruct raw owner/revision mismatch')
    if (compatibility['horizon_repository'],compatibility['mixins_repository'],compatibility['horizon_revision'],compatibility['mixins_revision']) != ('https://github.com/horizon-rust/horizon.git','https://github.com/horizon-rust/horizon-mixins.git',HORIZON,MIXINS):
        raise ValueError('TConstruct compatibility header mismatch')
    header=json.loads((root/'pluto-lib/horizon-native.json').read_text())
    if set(header)!={'schema','id','version','game','horizon','mixins'} or header['schema']!=1 or header['id']!='pluto_native_creative' or header['horizon']!=HORIZON or header['mixins']!=MIXINS:
        raise ValueError('Pluto header identity/revision mismatch')
    for name in COMPONENTS:
        m=tomllib.loads((root/'pluto-lib/mixins'/name/'horizon-mixin.toml').read_text())
        if (m['mod']['id'],m['mod']['version'],m['compatibility']['horizon_revision'])!=(name,header['version'],HORIZON):
            raise ValueError('Pluto component identity/revision mismatch: '+name)
    return {'tconstruct_raw_id':tc['mod']['id'],'horizon':HORIZON,'mixins':MIXINS,'pluto_components':list(COMPONENTS)}

def archive_payload(path, expected_id):
    raw=regular(path,MAX_ARCHIVE);data=members(io.BytesIO(raw))
    if len(data)>MAX_ENTRIES or sum(map(len,data.values()))>MAX_UNPACKED:raise ValueError('Individual native package exceeds budget')
    if 'horizon-native.json' not in data:raise ValueError('Missing package header')
    header=json.loads(data['horizon-native.json'])
    if header.get('schema')!=1 or header.get('id')!=expected_id or header.get('horizon')!=HORIZON or header.get('mixins')!=MIXINS:
        raise ValueError('Wrong package identity/revision')
    # The historical ZIP reader verifies regular type. Retain exact generated permission bits too.
    import zipfile
    with zipfile.ZipFile(io.BytesIO(raw)) as archive:
        if any((i.external_attr>>16)&0o7777 != 0o644 for i in archive.infolist()):raise ValueError('Package file mode changed')
    return data,{'id':expected_id,'archive_sha256':sha(raw),'archive_bytes':len(raw),'members':len(data),
                 'unpacked_bytes':sum(map(len,data.values())),'header':header,
                 'member_sha256':{p:sha(b) for p,b in sorted(data.items())},'file_mode':'0644'}

def verify_tconstruct(root, archive):
    root=Path(root);value,evidence=archive_payload(archive,'tconstruct_native')
    config=tomllib.loads((root/'tconstruct/mixins/tconstruct/horizon-mixin.toml').read_text())
    expected={'horizon-native.json':(json.dumps({'schema':1,'id':'tconstruct_native','version':config['mod']['version'],
        'game':'26.3','horizon':HORIZON,'mixins':MIXINS},indent=2)+'\n').encode()}
    for relative,p in old.tracked(root,'tconstruct/mixins/tconstruct'):
        if p.suffix not in ('.rs','.toml'):raise ValueError('Unexpected tracked native file')
        inside=Path(relative).relative_to('tconstruct/mixins/tconstruct').as_posix()
        body=p.read_bytes().replace(b'__horizon_mixins_tconstruct_',b'__horizon_mixins_tconstruct_native_')
        if inside=='horizon-mixin.toml':body=body.replace(b'id = "tconstruct"',b'id = "tconstruct_native"',1)
        expected['mixins/tconstruct_native/'+inside]=body
    support={}
    for relative,p in old.tracked(root,'tconstruct/content','tconstruct/assets','tconstruct/third_party','tconstruct/docs','tconstruct/tools','pluto-lib/pluto_lib'):
        parts=Path(relative).parts
        if parts[:2]==('tconstruct','docs') and not p.name.endswith('.sha256'):continue
        if parts[:2]==('tconstruct','tools') and p.suffix!='.py':continue
        support[relative]=p.read_bytes()
    key='mixins/tconstruct_native/data/tconstruct-support.zip'
    if key not in value or members(io.BytesIO(value[key]))!=support:raise ValueError('Nested support file-set/content changed')
    expected[key]=value[key]
    if value!=expected:raise ValueError('TConstruct archive differs from exact tracked source transform')
    evidence['support']={'members':len(support),'unpacked_bytes':sum(map(len,support.values())),'member_sha256':{p:sha(b) for p,b in sorted(support.items())}}
    evidence['support_seal_sha256']=sha(json.dumps({p:sha(b) for p,b in sorted(support.items())},sort_keys=True).encode())
    return evidence

def verify_pluto(root, archive):
    root=Path(root);value,evidence=archive_payload(archive,'pluto_native_creative')
    module=load('exact_integration_packager',root/'pluto-lib/tools/package_native.py')
    if tuple(module.COMPONENTS)!=COMPONENTS or tuple(module.CRATE_DIRECTORIES)!=('api','inventory-api'):
        raise ValueError('Reviewed component/crate closure changed')
    header_raw=(root/'pluto-lib/horizon-native.json').read_bytes();header=json.loads(header_raw)
    manifest,files=module._bundle(header)
    prefix='mixins/pluto_native_creative/'
    expected={'horizon-native.json':header_raw,prefix+'horizon-mixin.toml':manifest}
    expected.update({prefix+p:b for p,b in files.items()})
    expected[prefix+'NOTICE.md']=(b'Historical source attribution; the legacy network adapters are not shipped in this bundle.\n\n'+(root/'pluto-lib/mixins/pluto_items/NOTICE.md').read_bytes())
    if value!=expected:raise ValueError('Pluto archive differs from fixed component closure')
    evidence['verification']='Same exact pinned packager closure reconstructed; not an independent reimplementation of packaging'
    return evidence

def compose(tc_archive, pluto_archive, destination):
    groups=[archive_payload(tc_archive,'tconstruct_native')[0],archive_payload(pluto_archive,'pluto_native_creative')[0]]
    combined={};headers=[]
    for group in groups:
        headers.append(json.loads(group['horizon-native.json']))
        for path,body in group.items():
            if path=='horizon-native.json':continue
            if not path.startswith('mixins/'):raise ValueError('Unexpected active package member')
            relative=path[len('mixins/'):]
            if relative.casefold() in {n.casefold() for n in combined}:raise ValueError('Cross-package duplicate owner path')
            combined[relative]=body
    if len(combined)>2*MAX_ENTRIES or sum(map(len,combined.values()))>2*MAX_UNPACKED:raise ValueError('Combined package budget exceeded')
    destination=Path(destination);destination.mkdir()
    for path,body in combined.items():
        target=destination/path;target.parent.mkdir(parents=True,exist_ok=True)
        with target.open('xb') as stream:stream.write(body)
    hashes={p:sha(b) for p,b in sorted(combined.items())}
    return {'headers':headers,'members':len(hashes),'unpacked_bytes':sum(map(len,combined.values())),
            'member_sha256':hashes,'members_seal_sha256':sha(json.dumps(hashes,sort_keys=True).encode())}

def inventory(mods):
    """Enumerate the real composed manifest set, not a historical fixed member count."""
    result={};counts={k:0 for k in ('injections','modules','states','crates','bridges')}
    for path in sorted(Path(mods).glob('*/horizon-mixin.toml')):
        manifest=tomllib.loads(regular(path,1024**2).decode());owner=manifest['mod']['id']
        if owner in result or owner!=path.parent.name or manifest['compatibility']['horizon_revision']!=HORIZON:raise ValueError('Wrong manifest owner/revision')
        result[owner]=manifest
        for single,plural in [('injection','injections'),('module','modules'),('state','states'),('crate','crates'),('bridge','bridges')]:
            rows=manifest.get(single,[]);counts[plural]+=len(rows)
            if len({row['id'] for row in rows})!=len(rows):raise ValueError('Duplicate declared identity')
    if set(result)!={'tconstruct_native','pluto_native_creative'}:raise ValueError('Unexpected composed owner set')
    return result,counts

def verify_plan(path,mods,required_inputs):
    raw=plan_bytes(path);plan=json.loads(raw);manifests,counts=inventory(mods)
    if plan.get('schema')!=6:raise ValueError('Wrong complete plan schema')
    for name,count in counts.items():
        if len(plan.get(name,[]))!=count:raise ValueError('Incomplete manifest count: '+name)
    observed_inputs=plan.get('inputs',{})
    if not isinstance(observed_inputs,dict) or not observed_inputs:raise ValueError('Missing actual plan inputs')
    if any(observed_inputs.get(p)!=v for p,v in required_inputs.items()):raise ValueError('Required source input drift')
    for field,decls,id_field in [('injections','injection','injection_id'),('modules','module','module_id')]:
        actual={(r['mod_id'],r[id_field]):r for r in plan[field]}
        expected={(owner,row['id']):(m,row) for owner,m in manifests.items() for row in m.get(decls,[])}
        if len(actual)!=len(plan[field]) or set(actual)!=set(expected):raise ValueError('Plan owner set differs: '+field)
        for key,(mf,decl) in expected.items():
            row=actual[key];owner=key[0]
            if field=='injections':
                want={'mod_version':mf['mod']['version'],'kind':decl['kind'],'bindings':decl.get('bindings',[]),
                    'priority':decl.get('priority',mf['mod'].get('priority',0)),
                    'helper_source':regular(Path(mods)/owner/decl['helper'],1024**2).decode()}
                if 'target' in decl:want['target']=decl['target']
            else:
                want={'mod_version':mf['mod']['version'],'file':decl['file'],'name':decl['name'],
                    'source':regular(Path(mods)/owner/decl['source'],1024**2).decode()}
            if any(row.get(k)!=v for k,v in want.items()):raise ValueError('Plan body/declaration differs: '+str(key))
    return {'passed':True,'counts':counts,'required_inputs':required_inputs,'input_count':len(observed_inputs),
        'input_map_sha256':sha(json.dumps(observed_inputs,sort_keys=True).encode()),
        'manifest_map_sha256':sha(json.dumps(manifests,sort_keys=True).encode()),'plan_sha256':sha(raw),'plan_bytes':len(raw),
        'scope':'Exact injection/module owners and payload bytes; state/crate/bridge counts only. Unchanged guards remain separate, no Rust type or execution claim.'}

def verify_official(roots,contract):
    records=[]
    for row in contract['sources']:
        key='mixins' if row['repo']=='horizon-mixins' else row['repo']
        b=regular(Path(roots[key])/row['path'],2*1024**2)
        if sha(b)!=row['sha256'] or len(b)!=row['size']:raise ValueError('Official contract changed: '+row['path'])
        records.append({'repo':row['repo'],'path':row['path'],'sha256':sha(b)})
    return records
