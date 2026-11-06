"""Pure exact-source ordinary postimage. Does not edit an installed tree or run tools."""
import hashlib
import json
import re
import tomllib

BASE = '59c23ce300e40eb6a477bb55f33552fddbc04599'
P = 'pluto-lib/mixins/pluto_native_creative/'
T = 'tconstruct/mixins/tconstruct/'
REMOVED = ('anvil-material-recipe', 'survival-melter-no-container',
           'overslime-crafting-assemble', 'overslime-crafting-remainders',
           'crafting-station-recipe-placement-guard')
DELEGATED = {
    REMOVED[0]: 'stored matching policy', REMOVED[1]: 'stored matching policy',
    REMOVED[2]: 'pure derivation assembly', REMOVED[3]: 'pure derivation remainders',
    REMOVED[4]: 'local station transaction',
}
INSERT = '    crate::game::recipe_book::tconstruct_station_transaction::ensure();\n'
POINT = '    let key = (pos[0], pos[1], pos[2]);\n'

def sha(value): return hashlib.sha256(value.encode()).hexdigest()

def array_blocks(source, kind):
    return list(re.finditer(r'^\[\['+kind+r'\]\]\n.*?(?=^\[\[|\Z)', source, re.M | re.S))

def module(id, file, name, source, visibility):
    return '\n[[module]]\n'+''.join(k+' = '+json.dumps(v)+'\n' for k,v in
        [('id',id),('file',file),('name',name),('source',source),('visibility',visibility)])

def injection(id, target, helper, bindings, cancellable):
    return ('\n[[injection]]\nid = '+json.dumps(id)+'\ntarget = '+json.dumps(target)+
        '\nkind = "HEAD"\ncancellable = '+str(cancellable).lower()+'\nhelper = '+json.dumps(helper)+
        '\nbindings = ['+', '.join('{ '+', '.join(k+' = '+json.dumps(v) for k,v in b.items())+' }' for b in bindings)+']\n')

def checked_set(values, expected):
    if set(values)!=set(expected) or any(sha(v)!=expected[k] for k,v in values.items()):
        raise ValueError('Source set/hash differs')

def compose(i, before, proposals, bootstrap):
    if i['base']!=BASE or i['ordinary_installed'] or i['native_executed']:
        raise ValueError('Wrong cutoff or unproved activation')
    checked_set(before,i['before_sha256']);checked_set(proposals,i['proposal_sha256'])
    if sha(bootstrap)!=i['combined_bootstrap_sha256']:raise ValueError('Bootstrap source differs')
    if i['removed_hook_ids']!=list(REMOVED):raise ValueError('Removal scope differs')
    t=before[T+'horizon-mixin.toml'];p=before[P+'horizon-mixin.toml']
    original_t=tomllib.loads(t);original_p=tomllib.loads(p)
    removed={};replacements=[]
    for m in array_blocks(t,'injection'):
        entry=tomllib.loads(m[0])['injection'][0];id=entry['id']
        if id in REMOVED:
            if id in removed:raise ValueError('Duplicate removed hook')
            removed[id]=m[0]
            marker='# Delegated to '+DELEGATED[id]+' (old injection id: '+id+').\n\n'
            replacements.append((m.start(),m.end(),marker))
    if set(removed)!=set(REMOVED):raise ValueError('Missing old hook')
    for start,end,marker in reversed(replacements):t=t[:start]+marker+t[end:]
    unappended=t
    for id,old in removed.items():
        marker='# Delegated to '+DELEGATED[id]+' (old injection id: '+id+').\n\n'
        if unappended.count(marker)!=1:raise ValueError('Missing inverse marker')
        unappended=unappended.replace(marker,old)
    if unappended!=before[T+'horizon-mixin.toml']:raise ValueError('TConstruct removal inverse differs')

    for id,name,file in [
        ('crafting-stored-deny-policies','tconstruct_crafting_deny_policies','source-crafting-deny-policies.rs'),
        ('crafting-shared-bootstrap','tconstruct_crafting_bootstrap','source-crafting-bootstrap.rs'),
        ('overslime-derivation','tconstruct_overslime_derivation','source-overslime-derivation.rs')]:
        t+=module(id,'crates/horizon-client/src/crafting.rs',name,file,'pub(crate)')
    t+=module('station-transaction','crates/horizon-client/src/game/recipe_book.rs',
              'tconstruct_station_transaction','source-station-transaction.rs','pub(crate)')
    for id,target,helper in [
        ('crafting-shared-empty-startup','empty','source-crafting-empty-startup.rs'),
        ('crafting-shared-sources-startup','from_sources','source-crafting-sources-startup.rs')]:
        t+=injection(id,'horizon_client_lib::crafting::Recipes::'+target,helper,[],False)

    matches=[m for m in array_blocks(p,'injection') if tomllib.loads(m[0])['injection'][0]['id']=='recipe-placement-click']
    if len(matches)!=1:raise ValueError('Pluto placement HEAD not unique')
    m=matches[0];oldclick=m[0];entry=tomllib.loads(oldclick)['injection'][0]
    expected=[{'name':'game','ty':"&mut Game<'_>",'value':'self'},
              {'name':'book_type','ty':'BookType','value':'book_type'},
              {'name':'entry_index','ty':'usize','value':'entry_index'}]
    if entry['bindings']!=expected:raise ValueError('Pluto placement formals differ')
    lines=re.findall(r'^bindings = \[.*\]$',oldclick,re.M)
    if len(lines)!=1:raise ValueError('Binding source line not unique')
    newclick=oldclick.replace(lines[0],lines[0][:-1]+', { name = "use_max", ty = "bool", value = "use_max" }]')
    p=p[:m.start()]+newclick+p[m.end():]
    if p.replace(newclick,oldclick,1)!=before[P+'horizon-mixin.toml']:raise ValueError('Pluto binding inverse differs')
    p+=module('crafting-derivations','crates/horizon-client/src/crafting.rs','pluto_derivations','crafting-derivations.rs','pub')
    p+=module('recipe-transactions','crates/horizon-client/src/game/recipe_book.rs','pluto_transactions','recipe-transactions.rs','pub(crate)')
    p+=injection('crafting-pure-remainders','horizon_client_lib::crafting::Recipes::remainders','crafting-pure-remainders.rs',[
        {'name':'recipe','ty':'&str','value':'recipe'},
        {'name':'grid','ty':'&[Option<ItemStack>]','value':'grid'},
        {'name':'size','ty':'usize','value':'size'}],True)

    station=before[T+'source-crafting-station.rs']
    if station.count(POINT)!=1 or INSERT in station:raise ValueError('Station registration point differs')
    station=station.replace(POINT,INSERT+POINT)
    oldbody=before[T+'source-crafting-recipe-guard.rs']
    consumer=proposals['station/consumer-prefix.rs']+oldbody
    after={
        P+'horizon-mixin.toml':p,T+'horizon-mixin.toml':t,
        P+'inventory-api/src/crafting/matching.rs':proposals['stored/matching.rs'],
        P+'crafting-match.rs':proposals['stored/native-adapter.rs'],
        P+'crafting-match-veto.rs':proposals['pair/matching-head.rs'],
        P+'crafting-derivations.rs':proposals['pair/native-derivation.rs'],
        P+'crafting-pure-remainders.rs':proposals['pair/remainders-head.rs'],
        P+'recipe-transactions.rs':proposals['station/native-transaction.rs'],
        P+'recipe-placement-click.rs':proposals['station/single-head.rs'],
        T+'source-crafting-deny-policies.rs':proposals['stored/consumer.rs'],
        T+'source-overslime-derivation.rs':proposals['pair/consumer.rs'],
        T+'source-crafting-bootstrap.rs':bootstrap,
        T+'source-crafting-empty-startup.rs':proposals['bootstrap/empty-head.rs'].replace('tconstruct_crafting_deny_bootstrap::ensure','tconstruct_crafting_bootstrap::ensure'),
        T+'source-crafting-sources-startup.rs':proposals['bootstrap/from-sources-head.rs'].replace('tconstruct_crafting_deny_bootstrap::ensure','tconstruct_crafting_bootstrap::ensure'),
        T+'source-station-transaction.rs':consumer,
        T+'source-crafting-station.rs':station,
    }
    # Non-injection/module tables remain exact, including compatibility, crates,
    # state/constructors, other owners and previous cloud/payload adoption.
    for old,new in [(original_t,tomllib.loads(t)),(original_p,tomllib.loads(p))]:
        if {k:v for k,v in old.items() if k not in ['module','injection']}!={k:v for k,v in new.items() if k not in ['module','injection']}:
            raise ValueError('Unrelated manifest tables changed')
        for kind in ['module','injection']:
            ids=[x['id'] for x in new[kind]]
            if len(ids)!=len(set(ids)):raise ValueError('Duplicate manifest ID')
    if station.replace(INSERT,'')!=before[T+'source-crafting-station.rs']:raise ValueError('Station inverse differs')
    if consumer[len(proposals['station/consumer-prefix.rs']):]!=oldbody:raise ValueError('Old transaction differs')
    return after,{
        'base':BASE,'production_paths':len(after),'removed_hook_ids':list(REMOVED),
        'before_sha256':i['before_sha256'],'after_sha256':{n:sha(v) for n,v in after.items()},
        'preserved_legacy_files':{n:sha(v) for n,v in before.items() if n not in after},
        'inverse_checks':{'tc_manifest_removal':True,'pluto_click_formals':True,'station_registration_line':True,'old_transaction_body':True},
        'ordinary_installed':False,'native_executed':False,'constructor_fingerprints_observed':False,
    }
