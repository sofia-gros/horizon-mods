"""Exact full candidate from immutable archives; no planner, compiler or applying."""
import hashlib,importlib.util,json,os,re,stat,tomllib
from pathlib import Path
HERE=Path(__file__).resolve().parent
s=importlib.util.spec_from_file_location('mining_small_prepare',HERE.parent/'latest-mining-probe-ci/prepare.py');small=importlib.util.module_from_spec(s);s.loader.exec_module(small)
candidate=small.candidate
sha=lambda b:hashlib.sha256(b).hexdigest()
MAX_PLAN=16*1024**2

def read_tree(root):
    root=Path(root)
    if not stat.S_ISDIR(root.lstat().st_mode):raise ValueError('Non-directory payload')
    result={}
    for parent,dirs,files in os.walk(root,followlinks=False):
        for name in dirs:
            if not stat.S_ISDIR((Path(parent)/name).lstat().st_mode):raise ValueError('Linked payload directory')
        for name in files:
            path=Path(parent)/name
            if not stat.S_ISREG(path.lstat().st_mode):raise ValueError('Nonregular payload')
            result[path.relative_to(root).as_posix()]=path.read_bytes()
    return result

def seal(payload):return sha(json.dumps({p:sha(b) for p,b in sorted(payload.items())},sort_keys=True).encode())

def restore_original(after,revision_changes):
    inverse=after.copy()
    for name in [candidate.HELPER,candidate.COMMON_SOURCE]:del inverse[name]
    if not inverse[candidate.COMMON_MANIFEST].endswith(candidate.COMMON_NEW_BLOCK):raise ValueError('Common suffix changed')
    inverse[candidate.COMMON_MANIFEST]=inverse[candidate.COMMON_MANIFEST][:-len(candidate.COMMON_NEW_BLOCK)]
    body=inverse[candidate.MANIFEST]
    if body.count(candidate.NEW_BLOCK)!=1:raise ValueError('Missing unique new HEAD')
    inverse[candidate.MANIFEST]=body.replace(candidate.NEW_BLOCK,candidate.oracles()['old_hook_blocks'].encode(),1)
    for row in revision_changes:
        before=row['before_line'].encode();after=row['after_line'].encode();body=inverse[row['path']]
        if body.count(after)!=1 or before in body:raise ValueError('Revision inverse is ambiguous')
        inverse[row['path']]=body.replace(after,before,1)
    return inverse

def inventory(payload):
    hooks=[];modules=[];counts={k:0 for k in ['injections','modules','states','crates']}
    for name,body in sorted(payload.items()):
        if not name.endswith('/horizon-mixin.toml'):continue
        manifest=tomllib.loads(body.decode());owner=manifest['mod']['id']
        hooks.extend((owner,r['id']) for r in manifest.get('injection',[]))
        modules.extend((owner,r['id'],r['file'],r['name']) for r in manifest.get('module',[]))
        for field,plural in [('injection','injections'),('module','modules'),('state','states'),('crate','crates')]:counts[plural]+=len(manifest.get(field,[]))
    if len(set(hooks))!=len(hooks) or len({(r[2],r[3]) for r in modules})!=len(modules):raise ValueError('Duplicate payload owner')
    return {'hooks':hooks,'modules':modules,'counts':counts}

def write_candidate(original,revised,destination,revision_changes):
    original_map=read_tree(original);before=read_tree(revised)
    after=candidate.transform_payload(before)
    if len(original_map)!=644 or len(after)!=646 or restore_original(after,revision_changes)!=original_map:raise ValueError('Full archive inverse mismatch')
    expected_paths={candidate.MANIFEST,candidate.COMMON_MANIFEST,candidate.HELPER,candidate.COMMON_SOURCE}
    changed={name for name in set(original_map)|set(after) if original_map.get(name)!=after.get(name)}
    if changed!=expected_paths:raise ValueError('Wrong four-path delta')
    info=inventory(after)
    if any(name in candidate.OLD_IDS for _,name in info['hooks']) or info['hooks'].count(('tconstruct_native',candidate.NEW_ID))!=1:raise ValueError('Mining hook ownership mismatch')
    manifests=[tomllib.loads(b.decode()) for n,b in after.items() if n.endswith('/horizon-mixin.toml')]
    target=tomllib.loads(candidate.NEW_BLOCK.decode())['injection'][0]['target']
    if sum(row['target']==target for mf in manifests for row in mf.get('injection',[]))!=1:raise ValueError('Another mining target hook')
    dest=Path(destination)
    if dest.exists() or dest.is_symlink():raise ValueError('Candidate destination already exists')
    dest.mkdir()
    for name,body in after.items():
        p=dest/name;p.parent.mkdir(parents=True,exist_ok=True)
        with p.open('xb') as out:out.write(body)
    if read_tree(dest)!=after:raise ValueError('Written candidate differs')
    return {'passed':True,'original_members':644,'candidate_members':646,
      'original_members_sha256':seal(original_map),'revision_only_members_sha256':seal(before),'candidate_members_sha256':seal(after),
      'inverse_matches_original_archives':True,'changed_paths':sorted(changed),
      'changes':[{'path':n,'before_sha256':sha(original_map[n]) if n in original_map else None,'after_sha256':sha(after[n]),'after_bytes':len(after[n])} for n in sorted(changed)],
      'counts':info['counts'],'owner_inventory_sha256':sha(json.dumps(info,sort_keys=True).encode()),
      'preserved_policy':candidate.oracles()['preserved_policy'],
      'scope':'Both original archives and all other payload bytes preserved. New compute-only boundary has no type/apply proof.'}

def plan_bytes(path):
    path=Path(path);st=path.lstat()
    if not stat.S_ISREG(st.st_mode) or not 0<st.st_size<=MAX_PLAN:raise ValueError('Unsafe or oversized complete plan')
    raw=path.read_bytes()
    if len(raw)!=st.st_size:raise ValueError('Plan changed during read')
    return raw

def verify_plan(path,mods):
    raw=plan_bytes(path);plan=json.loads(raw);payload=read_tree(mods);want=inventory(payload)
    if plan.get('schema')!=6:raise ValueError('Wrong complete schema')
    for name,count in want['counts'].items():
        if len(plan.get(name,[]))!=count:raise ValueError('Wrong complete count: '+name)
    hooks=[(r.get('mod_id'),r.get('injection_id')) for r in plan['injections']]
    modules=[(r.get('mod_id'),r.get('module_id'),r.get('file'),r.get('name')) for r in plan['modules']]
    if len(set(hooks))!=len(hooks) or set(hooks)!=set(want['hooks']):raise ValueError('Complete hook owners differ')
    if len(set(modules))!=len(modules) or set(modules)!=set(want['modules']):raise ValueError('Complete module owners differ')
    required=small.required_plan_inputs()
    if any(plan.get('inputs',{}).get(k)!=v for k,v in required.items()):raise ValueError('Required complete source input differs')
    injections=[r for r in plan['injections'] if r['injection_id']==candidate.NEW_ID]
    selected=[r for r in plan['modules'] if r['module_id']=='mining-context-facts']
    if len(injections)!=1 or len(selected)!=1:raise ValueError('Ambiguous mining owner')
    row=injections[0];module=selected[0]
    mf=tomllib.loads(payload[candidate.MANIFEST].decode());decl=next(x for x in mf['injection'] if x['id']==candidate.NEW_ID)
    for k,v in {'mod_id':'tconstruct_native','kind':'HEAD','target':decl['target'],'bindings':decl['bindings'],'priority':mf['mod']['priority'],'cancellable':False,'contract':small.VOID_HEAD_CONTRACT,'helper_source':payload[candidate.HELPER].decode(),'file':'crates/horizon-client/src/game/native_hooks.rs'}.items():
        if row.get(k)!=v:raise ValueError('Mining contract differs: '+k)
    if any(r.get('target')==decl['target'] and r is not row for r in plan['injections']):raise ValueError('Competing mining target')
    if row.get('captures',[])!=[]:raise ValueError('Unexpected mining capture')
    if not isinstance(row.get('node'),list) or not row['node'] or any(type(n) is not int or n<0 for n in row['node']):raise ValueError('Missing mining node')
    if not isinstance(row.get('function_fingerprint'),str) or not re.fullmatch('[0-9a-f]{64}',row['function_fingerprint']):raise ValueError('Missing mining fingerprint')
    for k,v in {'mod_id':'pluto_native_creative','file':'crates/horizon-client/src/game.rs','name':'pluto_mining_context','source':payload[candidate.COMMON_SOURCE].decode()}.items():
        if module.get(k)!=v:raise ValueError('Mining module differs: '+k)
    if module.get('visibility','pub(crate)')!='pub(crate)' or not isinstance(module.get('fingerprint'),str) or not re.fullmatch('[0-9a-f]{64}',module['fingerprint']):raise ValueError('Missing mining module fingerprint')
    return {'passed':True,'counts':want['counts'],'owner_inventory_sha256':sha(json.dumps(want,sort_keys=True).encode()),
      'required_inputs':required,'selected_injection':row,'selected_module':module,
      'plan_sha256':sha(raw),'plan_bytes':len(raw),'native_type_verified':False,
      'scope':'Manifest ownership and selected mining IR only; no type/apply or full compatibility claim.'}
