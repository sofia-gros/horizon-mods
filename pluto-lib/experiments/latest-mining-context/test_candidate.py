"""Pure source/byte guards only; no Rust, planner, Game or arithmetic execution."""
import ast,importlib.util,json,tomllib,unittest
from pathlib import Path
HERE=Path(__file__).resolve().parent
s=importlib.util.spec_from_file_location('mining_candidate',HERE/'candidate.py');m=importlib.util.module_from_spec(s);s.loader.exec_module(m)
class Tests(unittest.TestCase):
    def test_01_frozen_main_and_revision_only_bytes(self):
        self.assertEqual(m.sha(m.original_manifest()),m.oracles()['packaged_manifest_sha256'])
        b=m.revision_only_manifest();self.assertEqual(m.sha(b),m.REVISION_ONLY_SHA)
        self.assertEqual(tomllib.loads(b.decode())['compatibility']['horizon_revision'],m.LATEST)
    def test_02_only_two_old_hooks_become_one_head(self):
        before=m.revision_only_manifest();after=m.transform_manifest(before)
        old=tomllib.loads(before.decode());new=tomllib.loads(after.decode())
        self.assertEqual(len(old['injection']),290);self.assertEqual(len(new['injection']),289)
        ids=[x['id'] for x in new['injection']]
        for i in m.OLD_IDS:self.assertNotIn(i,ids)
        self.assertEqual(ids.count(m.NEW_ID),1)
        hook=next(x for x in new['injection'] if x['id']==m.NEW_ID)
        self.assertEqual(hook['kind'],'HEAD');self.assertEqual(hook['target'],"horizon_client::game::native_hooks::Game<'_>::native_break_speed")
        self.assertFalse({'variable','ordinal','selector','cancellable','result'} & hook.keys())
        for key in old.keys()-{'injection'}:self.assertEqual(old[key],new[key])
    def test_03_original_unbound_and_current_mutable(self):
        hook=tomllib.loads(m.NEW_BLOCK.decode())['injection'][0]
        self.assertEqual([b['name'] for b in hook['bindings']],['game','pos','state','speed'])
        self.assertEqual(hook['bindings'][-1],{'name':'speed','ty':'&mut f32','value':'&mut *speed'})
        self.assertFalse(any('$local' in x['value'] or x['value']=='original' for x in hook['bindings']))
    def test_04_all_other_hooks_keep_order_and_bytes(self):
        before=m.revision_only_manifest();after=m.transform_manifest(before)
        old=m.oracles()['old_hook_blocks'].encode()
        self.assertEqual(before.split(old),after.split(m.NEW_BLOCK))
        oldhooks=[x for x in tomllib.loads(before.decode())['injection'] if x['id'] not in m.OLD_IDS]
        newhooks=[x for x in tomllib.loads(after.decode())['injection'] if x['id']!=m.NEW_ID]
        self.assertEqual(oldhooks,newhooks)
        self.assertTrue(any(x['id']=='source-material-mining-speed' for x in newhooks))
    def test_05_manifest_inverse(self):
        before=m.revision_only_manifest();after=m.transform_manifest(before)
        self.assertEqual(after.replace(m.NEW_BLOCK,m.oracles()['old_hook_blocks'].encode(),1),before)
    def test_06_wrong_revision_policy_or_capture_rejected(self):
        b=m.revision_only_manifest()
        for bad in [b.replace(m.LATEST.encode(),m.OLD.encode()),b+b'\n',b.replace(b'ordinal = 1\nhelper = "source-mining-context-host.rs"',b'ordinal = 0\nhelper = "source-mining-context-host.rs"'),b.replace(b'name = "speed", ordinal = 0',b'name = "progress", ordinal = 0',1)]:
            self.assertNotEqual(b,bad)
            with self.assertRaises(ValueError):m.transform_manifest(bad)
    def test_07_second_application_rejected(self):
        with self.assertRaises(ValueError):m.transform_manifest(m.transform_manifest(m.revision_only_manifest()))
    def test_08_partial_or_wrong_payload_rejected(self):
        for bad in [{},{m.MANIFEST:m.revision_only_manifest()},[],{m.MANIFEST:'not bytes'}]:
            with self.assertRaises(ValueError):m.transform_payload(bad)
    def test_09_exact_candidate_helper(self):
        b=(HERE/'source-mining-context-native.rs').read_bytes();i=json.loads((HERE/'inputs.json').read_text())
        self.assertEqual(m.sha(b),i['candidate_helper_sha256'])
        text=b.decode();body=text[text.index(') {')+3:]
        self.assertEqual(body.count('tconstruct_source_mining_context::apply('),1)
        self.assertEqual(body.count('pluto_mining_context::read(game, pos, state)'),1)
        self.assertIn('Some((context.pos[1], context.max_y))',body)
        for forbidden in ['game.mining','original','clone(','.clamp(','.is_finite(','unsafe','dispatch_break_speed','native_break_speed','break_block','send_','damage_']:
            self.assertNotIn(forbidden,body)
    def test_10_packaged_policy_identity_separate_from_source(self):
        p=m.oracles()['preserved_policy'];self.assertEqual(len(p),6)
        self.assertNotEqual(p['source-mining-context-shared.rs']['source_sha256'],p['source-mining-context-shared.rs']['packaged_sha256'])
        for name in ['source-mining-context.rs','source-mining-context-host.rs','source-mining-return.rs','source-mining-rules.rs','source-mining-math.rs']:
            self.assertEqual(p[name]['source_sha256'],p[name]['packaged_sha256'])
    def test_11_transform_has_no_writes_or_processes(self):
        tree=ast.parse((HERE/'candidate.py').read_text())
        for name in ['transform_payload','transform_manifest']:
            node=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name==name)
            calls=[ast.unparse(n.func) for n in ast.walk(node) if isinstance(n,ast.Call)]
            self.assertFalse(any(c in {'open','exec','eval'} or c.endswith(('.write_text','.write_bytes','.mkdir','.unlink','.run','.Popen')) for c in calls))
    def test_12_source_seal_and_official_contract(self):
        i=json.loads((HERE/'inputs.json').read_text())
        self.assertEqual(i['official'],m.LATEST);self.assertEqual(len(i['source_inputs']),4)
        for path,want in i['source_sha256'].items():self.assertEqual(m.sha((HERE/path).read_bytes()),want)
    def test_13_common_module_only_delta(self):
        before=m.common_revision_only_manifest();after=m.transform_common_manifest(before)
        a=tomllib.loads(before.decode());b=tomllib.loads(after.decode())
        self.assertEqual(len(a['module']),69);self.assertEqual(len(b['module']),70)
        self.assertEqual(a['module'],b['module'][:-1])
        for k in a.keys()-{'module'}:self.assertEqual(a[k],b[k])
        new=b['module'][-1]
        self.assertEqual(new,{'id':'mining-context-facts','file':'crates/horizon-client/src/game.rs','name':'pluto_mining_context','source':'mining-context.rs'})
        self.assertNotIn('visibility',new)
        self.assertEqual(after[:-len(m.COMMON_NEW_BLOCK)],before)
    def test_14_common_wrong_and_duplicate_inputs_rejected(self):
        b=m.common_revision_only_manifest()
        for bad in [b+b'\n',m.transform_common_manifest(b),b.replace(m.LATEST.encode(),m.OLD.encode())]:
            with self.assertRaises(ValueError):m.transform_common_manifest(bad)
    def test_15_private_borrow_without_policy_or_projection(self):
        b=(HERE/'mining-context.rs').read_bytes();i=json.loads((HERE/'inputs.json').read_text())
        self.assertEqual(m.sha(b),i['private_adapter_sha256'])
        text=b.decode();body=text[text.index('pub(super) struct'):]
        for literal in ["facts: &'a horizon_items::blocks::StateFacts", "held: Option<&'a horizon_client::items::ItemStack>", "game: &'a crate::game::Game<'_>", 'facts: game.res.facts().get(state)', 'held: game.held()', 'max_y: game.world.shape.max_y()']:
            self.assertIn(literal,body)
        for forbidden in ['tconstruct','speed','original','&mut','clone(', 'components', 'snapshot','registry::register','unsafe','pub fn','pub struct','game.mining']:
            self.assertNotIn(forbidden,body)
    def test_16_existing_common_tool_owner_is_retained(self):
        before=tomllib.loads(m.common_revision_only_manifest().decode());after=tomllib.loads(m.transform_common_manifest(m.common_revision_only_manifest()).decode())
        self.assertEqual([x for x in before['module'] if x['name']=='pluto_tool_rules'],[x for x in after['module'] if x['name']=='pluto_tool_rules'])
        self.assertEqual([x for x in before['injection'] if x['id'] in ['pluto_tool_rules-speed','pluto_tool_rules-drops']],[x for x in after['injection'] if x['id'] in ['pluto_tool_rules-speed','pluto_tool_rules-drops']])
        self.assertEqual(len([x for x in after['injection'] if x['id'] in ['pluto_tool_rules-speed','pluto_tool_rules-drops']]),2)
if __name__=='__main__':unittest.main()
