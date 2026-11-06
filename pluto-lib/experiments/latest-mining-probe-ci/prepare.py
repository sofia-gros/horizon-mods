"""Only exact candidate module/HEAD bytes; deliberately not the full composition."""
import hashlib,importlib.util,json,re,stat,tomllib
from pathlib import Path,PurePosixPath
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('mining_candidate',HERE.parent/'latest-mining-context/candidate.py');candidate=importlib.util.module_from_spec(spec);spec.loader.exec_module(candidate)
sha=lambda b:hashlib.sha256(b).hexdigest()

def payload():
    full_main=candidate.transform_manifest(candidate.revision_only_manifest())
    full_common=candidate.transform_common_manifest(candidate.common_revision_only_manifest())
    main_header=full_main[:full_main.index(b'[[')]
    common_header=full_common[:full_common.index(b'[[')]
    result={candidate.MANIFEST:main_header+candidate.NEW_BLOCK,
      candidate.COMMON_MANIFEST:common_header+candidate.COMMON_NEW_BLOCK,
      candidate.HELPER:(candidate.HERE/'source-mining-context-native.rs').read_bytes(),
      candidate.COMMON_SOURCE:(candidate.HERE/'mining-context.rs').read_bytes()}
    main=tomllib.loads(result[candidate.MANIFEST].decode());common=tomllib.loads(result[candidate.COMMON_MANIFEST].decode())
    if set(main)!={'mod','compatibility','injection'} or set(common)!={'mod','compatibility','module'}:raise ValueError('Probe has unexpected manifest sections')
    if main['injection']!=tomllib.loads(candidate.NEW_BLOCK.decode())['injection'] or common['module']!=tomllib.loads(candidate.COMMON_NEW_BLOCK.decode())['module']:raise ValueError('Selected candidate declaration drift')
    for owner,manifest in [('tconstruct_native',main),('pluto_native_creative',common)]:
        if manifest['mod']['id']!=owner or manifest['compatibility']['horizon_revision']!=candidate.LATEST:raise ValueError('Wrong probe owner or revision')
    expected=json.loads((HERE/'inputs.json').read_text())['probe_payload_sha256']
    if {p:sha(b) for p,b in sorted(result.items())}!=expected:raise ValueError('Probe payload bytes changed')
    return result

def write_probe(destination):
    source=payload();destination=Path(destination)
    if destination.exists() or destination.is_symlink():raise ValueError('Probe output already exists')
    destination.mkdir()
    for p,body in source.items():
        path=destination/p;path.parent.mkdir(parents=True,exist_ok=True)
        with path.open('xb') as out:out.write(body)
    return {'members':4,'member_sha256':{p:sha(b) for p,b in sorted(source.items())},
      'modules':1,'injections':1,'states':0,'crates':0,
      'scope':'Reduced formal-entry probe only; unchanged shared policy is referenced but deliberately not supplied/stubbed or type checked.'}

def verify_plan(path):
    path=Path(path);info=path.lstat()
    if not stat.S_ISREG(info.st_mode) or not 0<info.st_size<=2*1024**2:raise ValueError('Unsafe or oversized actual plan')
    raw=path.read_bytes()
    if len(raw)!=info.st_size:raise ValueError('Plan changed during read')
    value=json.loads(raw)
    if value.get('schema')!=6 or not isinstance(value.get('transformer'),str) or not isinstance(value.get('rustc'),str):raise ValueError('Wrong actual plan schema')
    if len(value.get('injections',[]))!=1 or len(value.get('modules',[]))!=1:raise ValueError('Probe plan has wrong owner counts')
    for key in ['bridges','states','crates','data','post_build']:
        if value.get(key,[])!=[]:raise ValueError('Unexpected plan work: '+key)
    inputs=value.get('inputs')
    if not isinstance(inputs,dict) or not inputs:raise ValueError('Missing actual source input fingerprints')
    for name,digest in inputs.items():
        if not isinstance(name,str) or PurePosixPath(name).is_absolute() or '..' in PurePosixPath(name).parts or not isinstance(digest,str) or not re.fullmatch('[0-9a-f]{64}',digest):raise ValueError('Wrong source input fingerprint')
    source=payload();main=tomllib.loads(source[candidate.MANIFEST].decode());common=tomllib.loads(source[candidate.COMMON_MANIFEST].decode())
    injection=value['injections'][0];module=value['modules'][0];decl=main['injection'][0]
    for key,expected in {'mod_id':'tconstruct_native','mod_version':main['mod']['version'],'injection_id':candidate.NEW_ID,'target':decl['target'],'kind':'HEAD','file':'crates/horizon-client/src/game/native_hooks.rs','cancellable':False,'helper_source':source[candidate.HELPER].decode(),'bindings':decl['bindings']}.items():
        if injection.get(key)!=expected:raise ValueError('Wrong selected injection: '+key)
    if injection.get('captures',[])!=[]:raise ValueError('Lexical capture in formal probe')
    for key in ['argument','constant_site','call_site','site_fingerprint','selector','ordinal','require','expression_site','variable_site','variable','slice','hook','hook_version']:
        if injection.get(key) is not None:raise ValueError('Unexpected selected site: '+key)
    if not isinstance(injection.get('node'),list) or not injection['node'] or any(type(n) is not int or n<0 for n in injection['node']):raise ValueError('Missing actual selected node')
    if not re.fullmatch('[0-9a-f]{64}',injection.get('function_fingerprint','')):raise ValueError('Missing actual function fingerprint')
    for key,expected in {'mod_id':'pluto_native_creative','mod_version':common['mod']['version'],'module_id':'mining-context-facts','file':'crates/horizon-client/src/game.rs','name':'pluto_mining_context','source':source[candidate.COMMON_SOURCE].decode()}.items():
        if module.get(key)!=expected:raise ValueError('Wrong selected module: '+key)
    if module.get('visibility','pub(crate)')!='pub(crate)' or not re.fullmatch('[0-9a-f]{64}',module.get('fingerprint','')):raise ValueError('Wrong internal module visibility/fingerprint')
    subset={'schema':value['schema'],'transformer':value['transformer'],'rustc':value['rustc'],'injections':[injection],'modules':[module]}
    if len(json.dumps(subset).encode())>32*1024:raise ValueError('Selected plan witness exceeds bound')
    return {'passed':True,'plan_sha256':sha(raw),'plan_bytes':len(raw),'input_count':len(inputs),
      'inputs_sha256':sha(json.dumps(inputs,sort_keys=True).encode()),'selected_plan':subset,
      'native_type_verified':False,'full_composition_tested':False,
      'retained_scope':'Exact selected injection/module IR is retained; the full plan input map is represented only by count/hash.'}
