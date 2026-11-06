"""Pure source checks only. No compiler, metadata, plan, network or subprocess."""
from pathlib import Path
import hashlib,json,re,tomllib,unittest
HERE=Path(__file__).resolve().parent
EXPECTED={'bootstrap.rs','empty-head.rs','from-sources-head.rs','manifest.fragment.toml','audit.json','inputs.json','README.md','test_source.py','source-verification.json'}
h=lambda s:hashlib.sha256(s.encode()).hexdigest()

def validate(root=HERE):
    if {p.name for p in root.iterdir()} != EXPECTED: raise ValueError('source set drift')
    if any(p.is_symlink() or not p.is_file() for p in root.iterdir()): raise ValueError('unexpected file kind')
    receipt=json.loads((root/'source-verification.json').read_text())
    for path,digest in receipt['source_sha256'].items():
        if h((root/path).read_text())!=digest: raise ValueError('source identity drift: '+path)
    return json.loads((root/'inputs.json').read_text())

class SourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inputs=validate();cls.audit=json.loads((HERE/'audit.json').read_text())
        cls.code=(HERE/'bootstrap.rs').read_text();cls.manifest=tomllib.loads((HERE/'manifest.fragment.toml').read_text())
    def test_exact_excluded_scope(self):
        for key in ('ordinary_manifest_changed','old_hooks_removed','rust_executed','native_plan_executed'):self.assertIs(self.inputs[key],False)
        self.assertEqual(len(self.inputs['stored_candidate_sha256']),11)
    def test_one_owner_and_exact_registration(self):
        self.assertEqual(self.code.count('static REGISTERED:'),1)
        self.assertIn('require_registration(&REGISTERED, super::tconstruct_crafting_deny_policies::register)',self.code)
        for token in ('thread_local!','unsafe','RwLock','HashMap','BTreeMap'):self.assertNotIn(token,self.code)
    def test_error_cached_before_panic(self):
        production=self.code.split('#[cfg(test)]')[0]
        self.assertIn('if let Err(error) = state.get_or_init(register)',production)
        self.assertEqual(production.count('panic!('),1)
        self.assertNotIn('catch_unwind(',production)
        self.assertNotIn('unwrap_or_default',production)
    def test_two_exact_non_cancelling_heads(self):
        hooks=self.manifest['injection'];self.assertEqual(len(hooks),2)
        self.assertEqual({x['target'] for x in hooks},{'horizon_client::crafting::Recipes::empty','horizon_client::crafting::Recipes::from_sources'})
        for x in hooks:self.assertEqual((x['kind'],x['cancellable'],x['bindings']),('HEAD',False,[]))
    def test_both_helpers_same_owner(self):
        for name in ('empty-head.rs','from-sources-head.rs'):
            code=(HERE/name).read_text()
            self.assertEqual(code.count('crate::crafting::tconstruct_crafting_deny_bootstrap::ensure();'),1)
            self.assertNotIn('ControlFlow',code)
    def test_module_visibility_and_no_state(self):
        self.assertEqual(len(self.manifest['module']),2)
        self.assertNotIn('state',self.manifest)
        for x in self.manifest['module']:
            self.assertEqual(x['file'],'crates/horizon-client/src/crafting.rs');self.assertEqual(x['visibility'],'pub(crate)')
    def test_exact_constructor_fragments(self):
        self.assertEqual(self.audit['recipes_impl_self_literal_count'],2)
        for name,source in self.audit['constructors'].items():self.assertEqual(h(source),self.audit['constructor_sha256'][name])
        self.assertIn('pub fn empty() -> Self',self.audit['constructors']['empty'])
        self.assertIn(') -> Result<Self>',self.audit['constructors']['from_sources'])
        self.assertIn('Self::from_sources(root, items, overlay, sources)',self.audit['load_delegate'])
        self.assertEqual(self.audit['official_crafting_child_modules'],['tests'])
    def test_private_fields_and_distinct_native_type(self):
        for field in ('facts:','curses:','cook_index:'):
            self.assertIn(field,self.audit['native_type']);self.assertNotIn('pub '+field,self.audit['native_type'])
        snippets='\n'.join(x['text'] for p in self.audit['callers'].values() for x in p['excerpts'])
        self.assertIn('NativeRecipes as Recipes',snippets)
        self.assertLess(snippets.index('Recipes::from_sources(root'),snippets.index('self.recipe_tables.synced = Some(recipes)'))
    def test_five_unrun_helper_tests(self):
        names=re.findall(r'#\[test\]\s*fn (\w+)\(',self.code)
        self.assertEqual(names,self.inputs['bootstrap_test_names']);self.assertEqual(len(set(names)),5)
        self.assertEqual((self.inputs['api_new_tests'],self.inputs['differential_full_tests']), (6,16))
    def test_limits_are_explicit(self):
        doc=(HERE/'README.md').read_text()
        for phrase in ('not a graceful load error','No Rust, metadata','does not restore that list','not raw parse/over','type/startup tests','Overslime'):self.assertIn(phrase,doc)
        self.assertTrue(all(self.audit[k] is False for k in ('all_mod_composition_proved','actual_plan_accepted','actual_type_checked','actual_startup_executed')))

if __name__=='__main__':unittest.main()
