"""Fresh owned tiny payload and strict actual-plan inspection; never invokes tools."""
import hashlib,importlib.util,json,re,stat
from pathlib import Path,PurePosixPath
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('language_placement_source',HERE.parent/'language-overlay-placement/prepare.py')
placement=importlib.util.module_from_spec(spec);spec.loader.exec_module(placement)
sha=lambda b:hashlib.sha256(b).hexdigest()
IR_SOURCE_PINS={
 'crates/horizon-mixins-core/src/helpers.rs':'5d32a79d9344e8c35afdce6c1ba5e47324d2137ff2198884d70dc422efbe5661',
 'crates/horizon-mixins-core/src/source.rs':'b4fef8f78684bba421b5223e07e2c7806ce11c3e4e90b56ce166a751c4587637',
}
def require(value,message):
    if not value:raise ValueError(message)
def verify_ir_sources(mixins):
    require(json.loads((HERE/'inputs.json').read_bytes())['ir_contract_source_sha256']==IR_SOURCE_PINS,'IR pins changed')
    for p,h in IR_SOURCE_PINS.items():
        path=Path(mixins)/p
        require(stat.S_ISREG(path.lstat().st_mode) and sha(path.read_bytes())==h,'IR source changed: '+p)
    return dict(IR_SOURCE_PINS)
def native_inputs():return json.loads((placement.REPAIR/'inputs.json').read_bytes())['official_sources']
def verify_native_sources(official):
    placement.payload(official)  # exact seven-source/admission inverse, no host write
    return dict(native_inputs())
def payload(official):
    result={placement.MOD+'/'+p:b for p,b in placement.payload(official).items()}
    require({p:sha(b) for p,b in sorted(result.items())}==json.loads((HERE/'inputs.json').read_bytes())['probe_payload_sha256'],'Probe bytes changed')
    return result
def write_probe(destination,official):
    rows=payload(official);destination=Path(destination)
    require(not destination.exists() and not destination.is_symlink(),'Output already exists')
    destination.mkdir()
    for p,b in rows.items():
        file=destination/p;file.parent.mkdir(parents=True,exist_ok=True)
        with file.open('xb') as out:out.write(b)
    return {'members':4,'member_sha256':{p:sha(b) for p,b in sorted(rows.items())},'modules':1,'injections':2,'states':0,'crates':0,
      'scope':'Exact shared helper and separately proposed HEAD/STORE placement. No application, native typing, helper execution or build/cache proof.'}
def verify_plan(path,official):
    path=Path(path);info=path.lstat()
    require(stat.S_ISREG(info.st_mode) and 0<info.st_size<=2*1024**2,'Unsafe/oversized actual plan')
    raw=path.read_bytes();require(len(raw)==info.st_size,'Plan changed during read')
    value=json.loads(raw)
    require(isinstance(value.get('transformer'),str) and value['transformer'] and isinstance(value.get('rustc'),str) and value['rustc'],'Missing actual tool identity')
    inputs=value.get('inputs');require(isinstance(inputs,dict) and inputs,'Missing actual input map')
    for p,h in inputs.items():
        require(isinstance(p,str) and p and not PurePosixPath(p).is_absolute() and '..' not in PurePosixPath(p).parts and isinstance(h,str) and re.fullmatch('[0-9a-f]{64}',h),'Invalid source input')
    files=placement.payload(official);result=placement.inspect_plan(value,files,native_inputs())
    for module in value['modules']:
        require(re.fullmatch('[0-9a-f]{64}',module.get('fingerprint','')),'Missing module fingerprint')
    for row in value['injections']:
        require(isinstance(row.get('node'),list) and row['node'] and all(type(n) is int and n>=0 for n in row['node']),'Missing actual node')
        require(row.get('captures',[])==[],'Unexpected lexical capture')
        for key in ['argument','constant_site','call_site','selector','slice','hook','hook_version']:
            require(row.get(key) is None,'Unexpected selected field: '+key)
        if row['kind']=='HEAD':
            for key in ['site_fingerprint','expression_site','variable_site']:
                require(row.get(key) is None,'Unexpected HEAD site')
        else:
            require(type(row.get('ordinal')) is int and type(row.get('require')) is int and type(row.get('variable_site')) is int,'Wrong STORE ordinal types')
    require(len(json.dumps(result).encode())<=32*1024,'Selected witness exceeds bound')
    return {'passed':True,'plan_sha256':sha(raw),'plan_bytes':len(raw),'input_count':len(inputs),
      'inputs_sha256':sha(json.dumps(inputs,sort_keys=True).encode()),'selected_plan':result,
      'native_type_verified':False,'full_composition_tested':False,
      'retained_scope':'Exact tiny selected module/hooks are checked, native sources pinned, and helper bytes represented by hashes. No real writer or stamp execution.'}
