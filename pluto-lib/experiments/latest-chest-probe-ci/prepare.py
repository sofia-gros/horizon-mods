"""41f-only tiny owned LOAD probe. No apply, Game type check or full bundle."""
import hashlib, importlib.util, json, re, stat, tomllib
from pathlib import Path, PurePosixPath
HERE=Path(__file__).resolve().parent
def load(name,path):
    s=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);return m
candidate=load('chest_candidate',HERE.parent/'latest-chest-pick/candidate.py')
mining=load('unchanged_mining_candidate',HERE.parent/'latest-mining-context/candidate.py')
HORIZON='41f822b659789780f58d32c63ddfb6887ea1b8bf'
sha=lambda b:hashlib.sha256(b).hexdigest()
CONTRACT={'output':'ItemStack','value':'ItemStack','operation':None}
IR_SOURCE_PINS={
 'crates/horizon-mixins-core/src/helpers.rs':'5d32a79d9344e8c35afdce6c1ba5e47324d2137ff2198884d70dc422efbe5661',
 'crates/horizon-mixins-core/src/source.rs':'b4fef8f78684bba421b5223e07e2c7806ce11c3e4e90b56ce166a751c4587637',
 'crates/horizon-mixins-core/src/scopes.rs':'ad89d65e7f178affeb38627ead9891bdad1faf0fd1e035832cdf48268c3f29fd',
}
def config():return json.loads((HERE/'inputs.json').read_text())
def verify_ir_sources(root):
    if config()['ir_contract_source_sha256']!=IR_SOURCE_PINS:raise ValueError('Wrong IR pins')
    for path,digest in IR_SOURCE_PINS.items():
        p=Path(root)/path
        if not stat.S_ISREG(p.lstat().st_mode) or sha(p.read_bytes())!=digest:raise ValueError('Changed IR source: '+path)
    return dict(IR_SOURCE_PINS)
def required_plan_inputs():
    return {'crates/horizon-client/src/game/block_items.rs':candidate.oracle()['official_block_items_sha256'],
      'Cargo.lock':config()['official']['horizon']['lock']}
def verify_official(root):
    result=candidate.verify_official(Path(root)/'crates/horizon-client/src/game/block_items.rs')
    for p,digest in required_plan_inputs().items():
        path=Path(root)/p
        if not stat.S_ISREG(path.lstat().st_mode) or sha(path.read_bytes())!=digest:raise ValueError('Wrong official source: '+p)
    return {**result,'required_inputs':required_plan_inputs(),'revision':HORIZON}
def unsealed_payload():
    base=mining.transform_manifest(mining.revision_only_manifest())
    modified=candidate.transform_manifest(base)
    header=modified[:modified.index(b'[[')]
    before=f'horizon_revision = "{mining.LATEST}"'.encode();after=f'horizon_revision = "{HORIZON}"'.encode()
    if header.count(before)!=1 or after in header:raise ValueError('Wrong prior header')
    changed=header.replace(before,after,1)
    if changed.replace(after,before,1)!=header:raise ValueError('Header inverse failed')
    out={candidate.MANIFEST:changed+candidate.oracle()['new_hook'].encode(),
      candidate.HELPER:candidate.transform_helper(candidate.oracle()['old_helper'].encode())}
    m=tomllib.loads(out[candidate.MANIFEST].decode())
    if set(m)!={'mod','compatibility','injection'} or len(m['injection'])!=1:raise ValueError('Unexpected work')
    if m['mod']['id']!='tconstruct_native' or m['compatibility']['horizon_revision']!=HORIZON:raise ValueError('Wrong owner/revision')
    return out,{'before_line':before.decode(),'after_line':after.decode(),'original_header_sha256':sha(header),'candidate_header_sha256':sha(changed),'inverse_verified':True}
def payload():
    out,revision=unsealed_payload()
    if {p:sha(b) for p,b in sorted(out.items())}!=config()['probe_payload_sha256']:raise ValueError('Wrong probe payload')
    return out
def write_probe(destination):
    out=payload();destination=Path(destination)
    if destination.exists() or destination.is_symlink():raise ValueError('Existing destination')
    destination.mkdir()
    for p,b in out.items():
        target=destination/p;target.parent.mkdir(parents=True,exist_ok=True)
        with target.open('xb') as f:f.write(b)
    return {'members':2,'member_sha256':{p:sha(b) for p,b in sorted(out.items())},'injections':1,'modules':0,'states':0,'crates':0,
      'revision_change':unsealed_payload()[1],
      'scope':'Reduced formal LOAD only. Exact helper references real storage owner not supplied or stubbed here. No Rust type or complete646 proof.'}
def verify_plan(path):
    path=Path(path);st=path.lstat()
    if not stat.S_ISREG(st.st_mode) or not 0<st.st_size<=2*1024**2:raise ValueError('Unsafe plan')
    raw=path.read_bytes()
    if len(raw)!=st.st_size:raise ValueError('Plan changed')
    v=json.loads(raw)
    if v.get('schema')!=6 or not isinstance(v.get('transformer'),str) or not isinstance(v.get('rustc'),str):raise ValueError('Wrong schema')
    if len(v.get('injections',[]))!=1:raise ValueError('Wrong injection count')
    for k in ['modules','bridges','states','crates','data','post_build']:
        if v.get(k,[])!=[]:raise ValueError('Unexpected work: '+k)
    ins=v.get('inputs')
    if not isinstance(ins,dict) or not ins:raise ValueError('Missing inputs')
    for k,d in ins.items():
        if not isinstance(k,str) or PurePosixPath(k).is_absolute() or '..' in PurePosixPath(k).parts or not isinstance(d,str) or not re.fullmatch('[0-9a-f]{64}',d):raise ValueError('Invalid input')
    for p,d in required_plan_inputs().items():
        if ins.get(p)!=d:raise ValueError('Missing/drifted required source: '+p)
    out=payload();m=tomllib.loads(out[candidate.MANIFEST].decode());decl=m['injection'][0];i=v['injections'][0]
    for k,e in {'mod_id':'tconstruct_native','mod_version':m['mod']['version'],'injection_id':candidate.HOOK,'target':decl['target'],
      'file':'crates/horizon-client/src/game/block_items.rs','kind':'MODIFY_VARIABLE','cancellable':False,
      'priority':decl.get('priority',m['mod'].get('priority',0)),'contract':CONTRACT,'helper_source':out[candidate.HELPER].decode(),
      'bindings':decl['bindings'],'ordinal':3,'require':4,'site_fingerprint':sha(b'stack')}.items():
        if i.get(k)!=e:raise ValueError('Wrong selected injection: '+k)
    if i.get('captures',[])!=[]:raise ValueError('Unexpected capture')
    for k in ['argument','constant_site','call_site','selector','slice','hook','hook_version']:
        if i.get(k) is not None:raise ValueError('Unexpected field: '+k)
    if type(i.get('variable_site')) is not int or i['variable_site']<0:raise ValueError('Missing selected expression')
    if type(i.get('expression_site')) is not int or i['expression_site']!=i['variable_site']:raise ValueError('Expression/variable site mismatch')
    if not isinstance(i.get('node'),list) or not i['node'] or any(type(n)is not int or n<0 for n in i['node']):raise ValueError('Missing node')
    if not re.fullmatch('[0-9a-f]{64}',i.get('function_fingerprint','')):raise ValueError('Missing fingerprint')
    variable=i.get('variable',{});local=variable.get('local',{})
    if variable.get('at')!='LOAD':raise ValueError('Not a LOAD')
    for k,e in {'name':'stack','ordinal':0,'parameter':False,'mutable':True,'supported':True,'ty':None}.items():
        if local.get(k)!=e:raise ValueError('Wrong local: '+k)
    if type(local.get('id'))is not int or local['id']<0 or not re.fullmatch('[0-9a-f]{64}',local.get('fingerprint','')):raise ValueError('Missing local identity')
    subset={k:v[k] for k in ['schema','transformer','rustc','injections']}
    if len(json.dumps(subset).encode())>32*1024:raise ValueError('IR witness bound')
    return {'passed':True,'plan_sha256':sha(raw),'plan_bytes':len(raw),'required_inputs':required_plan_inputs(),'input_count':len(ins),
      'inputs_sha256':sha(json.dumps(ins,sort_keys=True).encode()),'selected_plan':subset,'native_type_verified':False,'full_composition_tested':False,
      'scope':'Exact source + official Scopes/plan contract pins bind the fourth LOAD to final Some(stack), after native decode. Formal IR only; inferred Rust type and Game/actor/storage lookup remain unproved.'}
