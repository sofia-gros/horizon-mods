"""Two revision-only diagnostic manifests; original archives remain immutable."""
import importlib.util,json,tomllib
from pathlib import Path
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('original_baseline_prepare',HERE.parent/'main-common-ci/prepare.py')
baseline=importlib.util.module_from_spec(spec);spec.loader.exec_module(baseline)
MAIN=baseline.MAIN
HORIZON='f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd'
MIXINS='5a64af938ec86b00d53d16a408aabc0038fb3e57'
BUNDLE=baseline.BUNDLE
MAIN_BUNDLE='8178b440266664d3db4dd9784af02894ebdb29910b0dd8f13504b2d341cf8f42'
sha=baseline.sha
git_state=baseline.git_state
metadata_args=baseline.metadata_args
allowed=baseline.allowed
write_wrappers=baseline.write_wrappers
diagnostic=baseline.diagnostic
members=baseline.members
compose=baseline.compose

def contract():return json.loads((HERE/'official-contract.json').read_text())

def verify_official(roots):
    result=[]
    for item in contract()['sources']:
        key='mixins' if item['repo']=='horizon-mixins' else item['repo']
        root=roots[key];path=root/item['path']
        if path.is_symlink() or not path.is_file():raise ValueError('Nonregular official contract input')
        body=path.read_bytes()
        if len(body)!=item['size'] or sha(body)!=item['sha256']:raise ValueError('Official contract bytes changed')
        result.append({'repo':item['repo'],'path':item['path'],'sha256':item['sha256']})
    return result

def verify_main_bundle(root,archive):
    if sha(archive.read_bytes())!=MAIN_BUNDLE:raise ValueError('Original main archive differs from actual baseline')
    return baseline.verify_main_bundle(root,archive)

def revised_manifest(body,item):
    if len(body)!=item['bytes'] or sha(body)!=item['before_sha256']:raise ValueError('Wrong original manifest')
    before=item['before_line'].encode();after=item['after_line'].encode()
    if body.count(before)!=1 or after in body:raise ValueError('Ambiguous revision replacement')
    changed=body.replace(before,after,1)
    if len(changed)!=len(body) or sha(changed)!=item['after_sha256']:raise ValueError('Wrong candidate manifest')
    if changed.count(after)!=1 or changed.replace(after,before,1)!=body:raise ValueError('Revision inverse failed')
    expected=tomllib.loads(body.decode());actual=tomllib.loads(changed.decode())
    if expected['compatibility']['horizon_revision']!=item['prior']:raise ValueError('Wrong original revision field')
    expected['compatibility']['horizon_revision']=HORIZON
    if actual!=expected:raise ValueError('Non-revision semantic change')
    return changed

def revise(original,destination,original_members):
    changes=contract()['manifest_changes']
    if len(changes)!=2 or {x['path'] for x in changes}!={'tconstruct_native/horizon-mixin.toml','pluto_native_creative/horizon-mixin.toml'}:
        raise ValueError('Expected exactly two declared manifest changes')
    payload={}
    for path in original.rglob('*'):
        if path.is_symlink():raise ValueError('Original payload link')
        if path.is_dir():continue
        if not path.is_file():raise ValueError('Original payload special file')
        payload[path.relative_to(original).as_posix()]=path.read_bytes()
    before={p:sha(b) for p,b in sorted(payload.items())}
    if before!=original_members or len(before)!=644:raise ValueError('Original composition drift')
    candidate=payload.copy()
    for item in changes:candidate[item['path']]=revised_manifest(payload[item['path']],item)
    changed_paths=sorted(p for p in payload if payload[p]!=candidate[p])
    if changed_paths!=sorted(x['path'] for x in changes):raise ValueError('Unexpected candidate diff')
    # Complete inverse, including every unchanged body/data/crate byte, before any output.
    inverse=candidate.copy()
    for item in changes:inverse[item['path']]=inverse[item['path']].replace(item['after_line'].encode(),item['before_line'].encode(),1)
    if inverse!=payload:raise ValueError('Full candidate inverse mismatch')
    destination.mkdir()
    for relative,body in candidate.items():
        path=destination/relative;path.parent.mkdir(parents=True,exist_ok=True)
        with path.open('xb') as out:out.write(body)
    after={p:sha(b) for p,b in sorted(candidate.items())}
    return {'change_count':2,'changed_paths':changed_paths,'changes':changes,
      'original_members_sha256':sha(json.dumps(before,sort_keys=True).encode()),
      'candidate_members_sha256':sha(json.dumps(after,sort_keys=True).encode()),
      'inverse_matches_original':True,'unchanged_members':642,
      'actual_horizon':HORIZON,'actual_mixins_tool':MIXINS,
      'header_policy':'Original hnmod headers remain unchanged; only two unpacked manifest horizon_revision fields change. This is a diagnostic candidate, not a repaired package.'}
