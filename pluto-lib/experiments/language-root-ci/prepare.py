"""Fresh root-only payload; inspect actual schema6, no tools or native writes."""
import hashlib,importlib.util,json,re,stat,tomllib
from pathlib import Path,PurePosixPath
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('root_key_source',HERE.parent/'language-root-key/source.py')
candidate=importlib.util.module_from_spec(spec);spec.loader.exec_module(candidate)
REPAIR=HERE.parent/'language-overlay-repair'
MOD='pluto_language_root_probe'
sha=lambda b:hashlib.sha256(b).hexdigest()
IR_SOURCE_PINS={
 'crates/horizon-mixins-core/src/helpers.rs':'5d32a79d9344e8c35afdce6c1ba5e47324d2137ff2198884d70dc422efbe5661',
 'crates/horizon-mixins-core/src/source.rs':'b4fef8f78684bba421b5223e07e2c7806ce11c3e4e90b56ce166a751c4587637',
 'crates/horizon-mixins-core/src/scopes.rs':'ad89d65e7f178affeb38627ead9891bdad1faf0fd1e035832cdf48268c3f29fd',
}
def require(value,message):
    if not value:raise ValueError(message)
def verify_ir_sources(mixins):
    require(json.loads((HERE/'inputs.json').read_bytes())['ir_contract_source_sha256']==IR_SOURCE_PINS,'IR pins changed')
    for p,h in IR_SOURCE_PINS.items():candidate.read(mixins,p,h)
    return dict(IR_SOURCE_PINS)
def native_inputs():return json.loads((candidate.HERE/'inputs.json').read_bytes())['official_sources']
def verify_native_sources(official,mixins):
    candidate.compose(official,REPAIR,mixins)  # memory-only inverse; not actual application
    return dict(native_inputs())
def payload(official,mixins):
    verify_native_sources(official,mixins)
    result={MOD+'/'+p:(candidate.HERE/p).read_bytes() for p in ['horizon-mixin.toml','root-store.rs']}
    require({p:sha(b) for p,b in sorted(result.items())}==json.loads((HERE/'inputs.json').read_bytes())['probe_payload_sha256'],'Probe bytes changed')
    return result
def write_probe(destination,official,mixins):
    rows=payload(official,mixins);destination=Path(destination)
    require(not destination.exists() and not destination.is_symlink(),'Output already exists')
    destination.mkdir()
    for p,b in rows.items():
        file=destination/p;file.parent.mkdir(parents=True,exist_ok=True)
        with file.open('xb') as out:out.write(b)
    return {'members':2,'member_sha256':{p:sha(b) for p,b in sorted(rows.items())},'modules':0,'injections':1,'states':0,'crates':0,
      'scope':'Exactly one root out STORE with explicit stamp reference. No writer, stamp STORE, apply, native type or cache proof.'}
def fingerprint(value):return isinstance(value,str) and re.fullmatch('[0-9a-f]{64}',value)
def integer(value):return type(value) is int and value>=0
def local(value,name):
    require(isinstance(value,dict) and set(value)=={'id','name','ordinal','parameter','mutable','supported','ty','fingerprint'},'Local schema')
    require(integer(value['id']) and type(value['ordinal']) is int and value['ordinal']==0,'Local identity')
    require(value['name']==name and value['parameter'] is False and value['mutable'] is False and value['supported'] is True and value['ty'] is None,'Local declaration')
    require(fingerprint(value['fingerprint']),'Local fingerprint')
def verify_plan(path,official,mixins):
    path=Path(path);info=path.lstat()
    require(stat.S_ISREG(info.st_mode) and 0<info.st_size<=2*1024**2,'Unsafe/oversized actual plan')
    raw=path.read_bytes();require(len(raw)==info.st_size,'Plan changed during read')
    v=json.loads(raw)
    require(v.get('schema')==6,'Wrong plan schema')
    require(isinstance(v.get('transformer'),str) and v['transformer'] and isinstance(v.get('rustc'),str) and v['rustc'],'Missing actual tool identity')
    require(all(not v.get(k) for k in ['bridges','states','crates','data','post_build','modules']),'Unexpected additional work')
    inputs=v.get('inputs');require(isinstance(inputs,dict) and inputs,'Missing actual input map')
    for p,h in inputs.items():
        require(isinstance(p,str) and p and not PurePosixPath(p).is_absolute() and '..' not in PurePosixPath(p).parts and fingerprint(h),'Invalid source input')
    for p,h in native_inputs().items():require(inputs.get(p)==h,'Official input mismatch: '+p)
    rows=v.get('injections');require(isinstance(rows,list) and len(rows)==1,'Expected exactly one injection')
    files=payload(official,mixins)
    manifest=tomllib.loads(files[MOD+'/horizon-mixin.toml'].decode());candidate.validate_manifest(manifest)
    expected=manifest['injection'][0];row=rows[0]
    for key in ['target','kind','bindings']:require(row.get(key)==expected[key],'Wrong planned '+key)
    require((row.get('mod_id'),row.get('mod_version'),row.get('priority'),row.get('file'),row.get('injection_id'))==(MOD,'0.0.0',100,candidate.TARGET,'language-policy-root'),'Wrong owner')
    require(row.get('cancellable') is False,'Unexpected cancellation')
    require(row.get('helper_source','').encode()==files[MOD+'/root-store.rs'],'Helper payload mismatch')
    require(isinstance(row.get('node'),list) and row['node'] and all(integer(n) for n in row['node']),'Missing actual node')
    require(fingerprint(row.get('function_fingerprint')) and fingerprint(row.get('site_fingerprint')),'Missing actual fingerprints')
    for key in ['argument','constant_site','call_site','selector','slice','hook','hook_version']:
        require(row.get(key) is None,'Unexpected selected field: '+key)
    require(type(row.get('ordinal')) is int and row['ordinal']==0 and type(row.get('require')) is int and row['require']==1,'Wrong STORE count')
    variable=row.get('variable');require(isinstance(variable,dict) and set(variable)=={'at','local'} and variable['at']=='STORE','Wrong variable site')
    local(variable['local'],'out')
    require(integer(row.get('variable_site')) and integer(row.get('expression_site')) and row['variable_site']==row['expression_site'],'Wrong STORE site')
    contract=row.get('contract',{});require(all(contract.get(k) is None for k in ['operation','bundle','result']),'Unexpected contract')
    require(re.sub(r'\s+','',contract.get('output','')).removeprefix('::')=='std::path::PathBuf' and re.sub(r'\s+','',contract.get('value','')).removeprefix('::')=='std::path::PathBuf','Wrong owned output/value')
    captures=row.get('captures');require(isinstance(captures,list) and len(captures)==1,'Expected one actual capture')
    capture=captures[0];require(set(capture)=={'binding','local','expression','ty'},'Capture schema')
    require((capture['binding'],capture['expression'],capture['ty'])==('stamp','&stamp','&String'),'Wrong stamp reference')
    local(capture['local'],'stamp');require(capture['local']['id']<variable['local']['id'],'Capture not preceding out')
    selected={k:value for k,value in row.items() if k!='helper_source'}
    require(len(json.dumps(selected).encode())<=32*1024,'Selected witness exceeds bound')
    return {'passed':True,'plan_sha256':sha(raw),'plan_bytes':len(raw),'input_count':len(inputs),
      'inputs_sha256':sha(json.dumps(inputs,sort_keys=True).encode()),
      'selected_plan':{'schema':6,'transformer':v['transformer'],'rustc':v['rustc'],'injection':selected,'helper_sha256':sha(files[MOD+'/root-store.rs'])},
      'native_type_verified':False,'full_composition_tested':False,
      'retained_scope':'Tiny selected root STORE/capture only. ROOT_NAME/helper type and actual writer/cache behavior remain unverified.'}
