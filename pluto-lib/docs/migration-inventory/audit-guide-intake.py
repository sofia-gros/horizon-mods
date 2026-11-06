"""Read-only Git object audit. Writes only the requested report artifact; never fetches, applies or edits PRs."""
import argparse, collections, hashlib, json, subprocess
from pathlib import Path
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--repo', required=True, type=Path, help='Approved horizon-mods checkout only')
parser.add_argument('--snapshot', required=True, type=Path, help='Exact published coordinator PR snapshot')
parser.add_argument('--output', required=True, type=Path, help='Audit JSON artifact, not a source checkout')
args=parser.parse_args()
ROOT=args.repo
MAIN='08afa992d5091e03360f3473ea93fd17aa793987'
CENTRAL='8715b1232c0fca133fe012f35830e2108e653d59'
CHAIN=[693,698,707,708,709,711,712,718,725,732,737,746,753,758,767,774,776,782,787,797,800,803,805]
GUIDE_BASE='442946c52b4a6e80ce0754295b1db3a61fa13372'
REMOTE_TREES={944:'9a97a8870625af2c16a53400b1643a29a090a6f3',945:'70ecd0de6435a3d7657c4fb1cfc4921304248185'}
REMOTE_945_CHAIN=['d4d798fbbdc9dbbe0f81adb0e4074c798737e5ad','a538d047894eec8f56c3b2e8724b19970198d4ab','9cf25e3ed1817611af263947dddabdded1dc4687','bcec17b5394aba2a58f20e9f77222424f9b2ad6f']
def git(*args):return subprocess.check_output(['git',*args],cwd=ROOT)
def text(*args):return git(*args).decode().strip()
def sha(raw):return hashlib.sha256(raw).hexdigest()
def blob(tree,path):
 r=subprocess.run(['git','rev-parse',tree+':'+path],cwd=ROOT,capture_output=True,text=True)
 return None if r.returncode else r.stdout.strip()
def entries(a,b):
 fields=git('diff','--raw','-z','--no-abbrev','-M','--no-ext-diff','--no-textconv',a,b).decode().split('\0');out=[];i=0
 while i<len(fields) and fields[i]:
  mode_a,mode_b,old,new,status=fields[i].split();i+=1;first=fields[i];i+=1
  second=fields[i] if status[0] in 'RC' else first
  if status[0] in 'RC':i+=1
  out.append({'status':status,'old_path':None if status=='A' else first,'path':second,'old_blob':None if set(old)=={'0'} else old,'new_blob':None if set(new)=={'0'} else new})
 return out
def delta(a,b):
 e=entries(a,b);patch=git('diff','--binary','--full-index','--no-color','--no-ext-diff','--no-textconv','-M',a,b)
 return {'changed_entries':len(e),'status_counts':dict(collections.Counter(x['status'][0] for x in e)),'patch_sha256':sha(patch),'patch_bytes':len(patch),'shortstat':text('diff','--shortstat','-M',a,b)}
def kind(p):
 if p.startswith('pluto-lib/'):
  if '/tests/' in p:return 'shared-tests'
  if '/docs/' in p or p.endswith('.md'):return 'shared-docs'
  return 'shared-foundation'
 if '/third_party/' in p:return 'attribution'
 if '/content/assets/tconstruct/book/' in p:return 'guide-content'
 if '/tests/' in p:return 'authored-tests'
 if '/docs/' in p:return 'consumer-docs'
 if p.endswith(('modifier-snapshot.rs','source-upgrade-rules.rs','source-tags.rs')):return 'cross-feature-eligibility'
 if 'source-materials-book' in p:return 'guide-consumer'
 if p.endswith('horizon-mixin.toml'):return 'host-wiring'
 return 'other-shared-integration'
def target(k):
 return {'guide-content':'keep-tconstruct-chapters','guide-consumer':'port-guide-consumer-mechanics-with-policy','cross-feature-eligibility':'coordinate-modifier-recipe-policy-owner','authored-tests':'carry-and-rerun-on-combined-target','attribution':'retain-source-notice','consumer-docs':'update-scope-without-overclaim','host-wiring':'replace-only-reviewed-conflicting-hooks','shared-foundation':'category01-preserve-newer-central','shared-tests':'category01-preserve-newer-tests','shared-docs':'category01-preserve-newer-proof-docs','other-shared-integration':'category01-owner-review'}[k]
snapshot=args.snapshot
if sha(snapshot.read_bytes()) != '326066269ca9c55183e3e6f53546b290db7331f05dc0bf467cf5becdab9a4651':raise ValueError('coordinator snapshot drift')
source_snapshot=json.loads(snapshot.read_text())
if source_snapshot['repository'] != 'sofia-gros/horizon-mods':raise ValueError('wrong repository')
all_rows=[{'n':r['number'],'title':r['title'],'head':r['head_ref'],'base':r['base_ref'],'parent':r['parent_open_pr'],'draft':r['draft'],'sha':r['head_sha'],'base_sha':r['reported_base_sha'],'url':r['url']} for r in source_snapshot['pull_requests']]
selected={r['n']:r for r in all_rows if r['n'] in CHAIN+[944,945]}
rows=[]
for n in CHAIN+[944,945]:
 r=selected[n];head=r['sha'];reported=r['base_sha']
 if n==944:effective=MAIN;head_tree=REMOTE_TREES[n];base_tree=MAIN;ancestry='remote commit parent is exact main; local tree equality verified'
 elif n==945:effective=selected[944]['sha'];head_tree=REMOTE_TREES[n];base_tree=REMOTE_TREES[944];ancestry='remote single-parent chain reaches exact #944 head in three commits'
 else:effective=text('merge-base',reported,head);head_tree=head;base_tree=effective;ancestry='local immutable commit DAG'
 own=entries(base_tree,head_tree)
 for e in own:
  e['classification']=kind(e['path']);e['intake_route']=target(e['classification'])
  c=blob(CENTRAL,e['path']);e['central_blob']=c
  e['central_relation']='removed-path-present' if e['new_blob'] is None and c is not None else 'same' if c==e['new_blob'] else 'absent' if c is None else 'different'
 inherited=CHAIN[:CHAIN.index(n)] if n in CHAIN else [944] if n==945 else []
 inherited_base=GUIDE_BASE if n in CHAIN else MAIN
 inherited_delta=delta(inherited_base,base_tree) if inherited else None
 inherited_paths={e['path'] for e in entries(inherited_base,base_tree)} if inherited else set()
 row={'number':n,'title':r['title'],'url':r['url'],'head_sha':head,'reported_base_sha':reported,'base_ref':r['base'],'effective_own_delta_base_sha':effective,'head_tree':text('rev-parse',head_tree+'^{tree}'),'effective_base_tree':text('rev-parse',base_tree+'^{tree}'),'parent_pr':r.get('parent'),'ancestry_evidence':ancestry,'own_delta':delta(base_tree,head_tree),'inherited_prs':inherited,'inherited_delta':inherited_delta,'own_paths_also_changed_by_inherited_delta':sorted(e['path'] for e in own if e['path'] in inherited_paths),'own_class_counts':dict(collections.Counter(e['classification'] for e in own)),'central_relation_counts':dict(collections.Counter(e['central_relation'] for e in own)),'recorded_change_state':'source authored and published; this audit performs no intake','verification_state':'original exact-head combined latest-official integration unverified','required_destination':'category08 grouped guide consumer; category01 shared services are dependencies','new_tests_run':0,'pr_state_changes':0}
 if n==693:
  row['reported_base_is_ancestor']=False;row['endpoint_delta_NOT_FOR_INTAKE']=delta(reported,head);row['base_side_changes_excluded_from_own']=delta(effective,reported)
 if n==944:
  row['required_destination']='category01 foundation dependency, not a second guide implementation'
  row['verification_state']='v13 ordinary-library plan/guards/apply/type proof exists; full consumer/game/LAN proof absent'
  row['own_entries_detail_scope']='476 entries summarized; unchanged 444 are not duplicated here. All 32 evolved entries listed for preservation review; immutable before/after tree and patch hash reproduce the complete set.'
  row['own_entries']=[e for e in own if e['central_relation']!='same']
  row['integration_note']='Do not replay v13 snapshot over newer central; its runtime31cf7667 is an ancestor of central, and 444 own path blobs already match while 32 evolved.'
 elif n==945:
  row['verification_state']='source/guard 27, modeled helper 8, actual isolated plan + 11 guards passed; apply/type/liveUI/LAN unverified'
  row['own_entries']=own;row['remote_parent_chain']=REMOTE_945_CHAIN
  row['integration_note']='Take only 11 rename-aware own entries; do not replace newer central with inherited 476-file v13 tree. This PR does not inherit the 23 content PR chain.'
 else:
  row['own_entries']=own
  row['integration_note']='Preserve this incremental delta once, not a full stacked snapshot. Final cumulative guide tip may subsume earlier file edits; retain a per-PR trace and validate source policy.'
 rows.append(row)
chain_tip=selected[805]['sha'];chain_own=[x for r in rows if r['number'] in CHAIN for x in r['own_entries']]
report={'schema':'guide-own-inherited-intake-v1','date_utc':'2026-11-06','kind':'read_only_git_object_classification_not_integration','cutoffs':{'main':MAIN,'central_local':CENTRAL,'central_remote':'89744814b8d536a117e82f8ea3c8cdc9798faa95','central_tree':'267be30ceab0c59e8763a1e2de4db0f4fff83cf5','dependency_snapshot_sha256':sha(snapshot.read_bytes()),'official_horizon':'624c4f6265c02296fbe8a80f029dd0d987fb94c0','official_mixins':'e8cd0aa38cffef49cb2e088ef716baa107148c02'},'counts':{'pr_rows':25,'content_chain_prs':23,'foundation_dependency_prs':1,'navigation_repair_prs':1,'new_native_jobs':0,'new_tests':0,'pr_state_changes':0},'content_chain_order':CHAIN,'content_chain_root_merge_base':GUIDE_BASE,'content_chain_tip':chain_tip,'content_chain_cumulative_delta':delta(GUIDE_BASE,chain_tip),'content_chain_incremental_touch_count':len(chain_own),'content_chain_unique_touched_paths':len({e['path'] for e in chain_own}),'rows':rows,'intake_rules':['Use actual merge-base for own changes, not stored REST base endpoints when divergent.','One layered content chain: do not apply every full head tree or double-count inherited deltas.','Keep TConstruct chapter/layout/metadata/recipe/modifier policy; move generic mechanics only through reviewed Pluto contracts.','Foundation#944 is a dependency already evolved centrally, not guide content equivalence. Preserve the 32 newer central paths.','Apply #945 own navigation delta only; its inherited v13 snapshot must not downgrade central.','The23 content heads have no established exact-head combined target proof. Source classification is not application, successful merge, or behavioral equivalence.','No PR may be closed, retargeted, marked ready, approved or merged by this mapping.'], 'contract_ledger_reference':'https://github.com/sofia-gros/horizon-mods/blob/42e9cbb3c044fe2f4bc0c324620dc0c58f2321a2/pluto-lib/docs/migration-inventory/guide-ui-lan-20261106.json','classification_limits':['File roles are intake triage, not per-hunk behavior proof; a production Rust file may contain test-only additions, for example #753 wire tests.','Blob identity proves exact file bytes only; differing bytes do not prove feature absence, and matching bytes do not prove integrated runtime behavior.','Patch SHA is the observed Git serialized diff; immutable trees and before/after blobs define source identity.','Original PR source content and inherited source are present in Git objects, not automatically integrated or verified on current main.']}
assert len(rows)==25 and len({r['number'] for r in rows})==25
out=args.output;out.write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'output':str(out),'sha256':sha(out.read_bytes()),'bytes':out.stat().st_size,'chain_cumulative':report['content_chain_cumulative_delta'],'incremental_touches':len(chain_own),'unique_paths':report['content_chain_unique_touched_paths'],'summary':[{'number':r['number'],'own':r['own_delta']['changed_entries'],'inherited_prs':len(r['inherited_prs']),'overlap':len(r['own_paths_also_changed_by_inherited_delta'])} for r in rows]}))
