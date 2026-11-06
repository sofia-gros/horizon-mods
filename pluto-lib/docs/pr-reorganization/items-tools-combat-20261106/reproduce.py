#!/usr/bin/env python3
"""Read only existing horizon-mods objects. No fetch, checkout, compiler or PR writes.
Usage: python reproduce.py --repo MOD_CHECKOUT --snapshot FIXED_JSON --out OUTPUT
Scope lives beside this script. Output may not be inside the source checkout.
"""
import argparse, collections, csv, hashlib, json, pathlib, subprocess, tomllib
p=argparse.ArgumentParser();p.add_argument('--repo',type=pathlib.Path,required=True);p.add_argument('--snapshot',type=pathlib.Path,required=True);p.add_argument('--out',type=pathlib.Path,required=True);a=p.parse_args()
repo=a.repo.resolve();out=a.out.resolve();here=pathlib.Path(__file__).resolve().parent
if out.is_relative_to(repo):raise SystemExit('audit output must not modify source checkout')
def git(*args,data=None,optional=False):
 r=subprocess.run(['git','-C',str(repo),*args],input=data,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
 if r.returncode:
  if optional:return None
  raise RuntimeError(r.stderr.decode())
 return r.stdout
def text(*args):return git(*args).decode().strip()
def sha(b):return hashlib.sha256(b).hexdigest()
def is_ancestor(x,y):return git('merge-base','--is-ancestor',x,y,optional=True) is not None
def merge_base(x,y):
 bases=text('merge-base','--all',x,y).splitlines()
 if len(bases)!=1:raise ValueError('ambiguous merge base')
 return bases[0]
def patch(x,y,*rest):return git('diff','--no-ext-diff','--no-textconv','--no-renames','--full-index','--binary',x,y,*rest)
def paths(x,y):return git('diff','--no-renames','--name-only','-z',x,y).decode().rstrip('\0').split('\0') if x!=y else []
def ls_state(commit,path):
 raw=git('ls-tree','-z',commit,'--',path)
 if not raw:return None
 meta,found=raw.rstrip(b'\0').split(b'\t',1);mode,kind,blob=meta.decode().split()
 if found.decode()!=path:raise ValueError('unexpected tree result')
 return {'mode':mode,'type':kind,'blob':blob}
def manifest(commit,path):
 raw=git('show',commit+':'+path,optional=True)
 return {} if raw is None else tomllib.loads(raw.decode())
origin=git('remote','get-url','origin').decode().strip()
if origin not in ('https://github.com/sofia-gros/horizon-mods.git','https://github.com/sofia-gros/horizon-mods'):raise ValueError('not the approved MOD origin')
scope=json.loads((here/'scope.json').read_text());snapshot_raw=a.snapshot.read_bytes()
if sha(snapshot_raw)!=scope['snapshot_sha256']:raise ValueError('fixed snapshot bytes changed')
snapshot=json.loads(snapshot_raw);prs={r['number']:r for r in snapshot['pull_requests']};selected=set(scope['selected_prs'])
assert len(selected)==37 and selected<=prs.keys()
main=scope['main'];central=scope['central_local'];manifest_path='tconstruct/mixins/tconstruct/horizon-mixin.toml'
central_hooks=[]
for path in text('ls-tree','-r','--name-only',central,'--','pluto-lib/mixins').splitlines():
 if path.endswith('horizon-mixin.toml'):
  for h in manifest(central,path).get('injection',[]):central_hooks.append({'manifest':path,**h})
all_paths={};missing=[];rows=[];files=[];patchids=collections.defaultdict(list);transitions=collections.defaultdict(list)
for n,r in sorted(prs.items()):
 head,base=r['head_sha'],r['reported_base_sha']
 absent=[s for s in (head,base) if git('cat-file','-e',s+'^{commit}',optional=True) is None]
 if absent:
  if n in selected:raise ValueError('selected object absent: '+str(n))
  missing.append({'number':n,'missing_commits':absent});continue
 mb=merge_base(base,head);changed=paths(mb,head);all_paths[n]=set(changed)
 if n not in selected:continue
 rawpatch=patch(mb,head);pidraw=git('patch-id','--stable',data=rawpatch).decode().split();pid=pidraw[0] if pidraw else None
 if pid:patchids[pid].append(n)
 mainmb=merge_base(main,head);cumulative=paths(mainmb,head);parents=text('show','-s','--format=%P',head).split();records=[]
 raw=git('diff','--raw','--no-renames','--no-abbrev','-z',mb,head).split(b'\0')
 for i in range(0,len(raw)-1,2):
  oldmode,newmode,oldblob,newblob,status=raw[i].decode()[1:].split();path=raw[i+1].decode();new=None if newmode=='000000' else {'mode':newmode,'type':'blob','blob':newblob}
  mainstate,centralstate=ls_state(main,path),ls_state(central,path)
  rec={'number':n,'status':status,'path':path,'old_mode':oldmode,'new_mode':newmode,'old_blob':oldblob,'new_blob':newblob,'patch_sha256':sha(patch(mb,head,'--',path)),'main_file_state':mainstate,'central_file_state':centralstate,'main_matches_new_file_state':new==mainstate,'central_matches_new_file_state':new==centralstate}
  records.append(rec);files.append(rec);transitions[(path,oldmode,newmode,oldblob,newblob)].append(n)
 assert {x['path'] for x in records}==set(changed)
 hooks=[]
 if manifest_path in changed:
  before,after=manifest(mb,manifest_path),manifest(head,manifest_path)
  for kind in ('injection','state','module'):
   old={h['id']:h for h in before.get(kind,[])};new={h['id']:h for h in after.get(kind,[])}
   for ident in sorted(old.keys()|new.keys()):
    if old.get(ident)!=new.get(ident):hooks.append({'kind':kind,'id':ident,'before':old.get(ident),'after':new.get(ident)})
 pluto_hits=[]
 for path in changed:
  if path.endswith('.rs'):
   raw=git('show',head+':'+path,optional=True)
   if raw is not None:
    hits=[{'line':i+1,'text':line.strip()[:300]} for i,line in enumerate(raw.decode(errors='replace').splitlines()) if any(k in line for k in ('pluto_','pluto::','pluto_api','pluto_inventory'))]
    if hits:pluto_hits.append({'path':path,'matches':hits})
 stack=[];cur=r['parent_open_pr']
 while cur is not None:
  if cur in stack:raise ValueError('cycle')
  stack.append(cur);cur=prs[cur]['parent_open_pr']
 targets=[]
 for h in hooks:
  if h['kind']=='injection' and h['after']:
   matches=[c for c in central_hooks if c.get('target')==h['after'].get('target')]
   if matches:targets.append({'pr_injection':h['after'],'central_injections':matches})
 rows.append({**r,'actual_merge_base_sha':mb,'reported_base_is_ancestor':mb==base,'head_parents':parents,'own_range_commits':text('rev-list','--reverse',mb+'..'+head).splitlines(),'reported_base_is_first_parent':bool(parents) and parents[0]==base,'parent_open_pr_head_matches_reported_base':r['parent_open_pr'] is None or prs[r['parent_open_pr']]['head_sha']==base,'open_stack_ancestors':stack,'head_is_ancestor_of_main':is_ancestor(head,main),'head_is_ancestor_of_central':is_ancestor(head,central),'main_actual_merge_base_sha':mainmb,'own_delta_file_count':len(changed),'own_delta_paths':changed,'cumulative_main_merge_base_file_count':len(cumulative),'cumulative_paths_not_in_own_delta':sorted(set(cumulative)-set(changed)),'direct_main_tree_difference_count':len(paths(main,head)),'exact_diff_sha256':sha(rawpatch),'stable_patch_id':pid,'new_file_states_matching_main':sum(x['main_matches_new_file_state'] for x in records),'new_file_states_matching_central':sum(x['central_matches_new_file_state'] for x in records),'native_descriptor_changes':hooks,'exact_target_intersections_with_pluto':targets,'pluto_lexical_matches_in_changed_rust_files':pluto_hits,'scope':'read-only source identity, not runtime compatibility or safe removal proof'})
pathindex=collections.defaultdict(list)
for n,ps in all_paths.items():
 for path in ps:pathindex[path].append(n)
intersections=[{'path':path,'selected_prs':sorted(set(ns)&selected),'external_prs':sorted(set(ns)-selected),'interpretation':'same-path review intersection; not duplicate/conflict proof'} for path,ns in sorted(pathindex.items()) if set(ns)&selected and len(ns)>1]
result={'schema':1,'kind':'fixed_remaining_37_pr_own_inherited_audit','scope':scope,'source_snapshot_sha256':sha(snapshot_raw),'method':'actual merge-base(reported base,head), then full-index no-renames binary diff. Parent links are supplied open-PR reference links, not inferred semantic dependencies. Exact file-state equality is not behavior equivalence. Lexical Pluto matches are not a completed migration.', 'summary':{'selected_prs':len(rows),'file_records':len(files),'distinct_selected_paths':len({r['path'] for r in files}),'stacked_prs':sum(r['parent_open_pr'] is not None for r in rows),'reported_base_not_ancestor':sum(not r['reported_base_is_ancestor'] for r in rows),'head_already_in_main_history':sum(r['head_is_ancestor_of_main'] for r in rows),'head_already_in_central_history':sum(r['head_is_ancestor_of_central'] for r in rows),'exact_target_intersections':sum(len(r['exact_target_intersections_with_pluto']) for r in rows),'pr_heads_with_lexical_pluto_matches':sum(bool(r['pluto_lexical_matches_in_changed_rust_files']) for r in rows),'path_intersections':len(intersections),'cross_scope_path_intersections':sum(bool(r['external_prs']) for r in intersections)},'missing_external_objects':missing,'prs':rows,'same_path_intersections':intersections,'same_whole_patch_id_groups':[{'patch_id':k,'prs':v} for k,v in sorted(patchids.items()) if len(v)>1],'same_file_transition_groups':[{'path':k[0],'old_mode':k[1],'new_mode':k[2],'old_blob':k[3],'new_blob':k[4],'prs':v} for k,v in sorted(transitions.items()) if len(v)>1],'limits':['No native execution, tests, official plans, production changes or PR actions.','Shared APIs and a passing v15 library composition do not port these original feature heads.','Exact target strings miss aliases and call-order constraints; mismatches must be reviewed semantically.','Unselected PR comparisons cover available paths, not semantic equivalence.']}
out.mkdir(exist_ok=True,parents=True);(out/'ledger.json').write_text(json.dumps(result,indent=2)+'\n');(out/'file-deltas.json').write_text(json.dumps(files,indent=2)+'\n')
print(json.dumps(result['summary'],indent=2));print('missing external objects',missing)
