import importlib.util
from pathlib import Path
import tempfile
import tomllib
import unittest
import zipfile
ROOT=Path(__file__).resolve().parents[1]
NATIVE=ROOT/'mixins/pluto_native_creative'
def load(name):
    spec=importlib.util.spec_from_file_location(name,ROOT/'tools'/f'{name}.py')
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
GUARD=load('check_station_ingredient_plan');PACKAGE=load('package_native')
class StationIngredient(unittest.TestCase):
    def test_generic_parser_has_one_registry_no_schema_or_cache(self):
        source=(NATIVE/'inventory-api/src/crafting/ingredients.rs').read_text()
        for forbidden in ('tconstruct:', 'forge:', 'horizon_', 'static mut', 'RecipeIndex'):
            self.assertNotIn(forbidden,source)
        self.assertEqual(source.count('static PARSERS:'),1)
        for expected in ('Unsupported','Handled','Conflict','UnknownItem','DuplicateItem','MAX_TOTAL_ID_BYTES','Option<&[String]>'):
            self.assertIn(expected,source)
    def test_borrowed_registry_and_native_fallback(self):
        source=(NATIVE/'station-ingredient.rs').read_text()
        self.assertIn('self.0.items.contains_key(id)',source)
        self.assertIn('self.0.item_tag_order.get(id)',source)
        self.assertIn('Outcome::Native => Continue(())',source)
        self.assertIn('Outcome::Conflict',source)
        self.assertIn('Outcome::Invalid',source)
        self.assertNotIn('&mut Items',source)
        self.assertLess(source.index('has_parsers()'),source.index('ingredients::evaluate'))
        manifest=tomllib.loads((NATIVE/'horizon-mixin.toml').read_text())
        hooks=[h for h in manifest['injection'] if h['id']=='station-ingredient-parser']
        self.assertEqual(len(hooks),1)
        self.assertEqual(hooks[0]['target'],'horizon_client_lib::stations::ingredient')
        self.assertEqual(hooks[0]['kind'],'HEAD');self.assertTrue(hooks[0]['cancellable'])
    def test_archive_carries_exact_api_adapter_and_head(self):
        with tempfile.TemporaryDirectory() as d:
            output=Path(d)/'station.hnmod';PACKAGE.build(output)
            with zipfile.ZipFile(output) as z:
                for file in ('inventory-api/src/crafting/ingredients.rs','station-ingredient.rs','station-ingredient-head.rs'):
                    self.assertEqual(z.read('mixins/pluto_native_creative/pluto_native_creative/'+file),(NATIVE/file).read_bytes())
    def metadata(self):
        return [dict(target='lib::stations::ingredient',injection_id='station-ingredient-parser',file='native.rs',kind='HEAD',cancellable=True,priority=100,bindings=[],function_fingerprint='fingerprint',helper='helper.rs')]
    def test_exclusive_native_boundary_rejects_every_other_phase(self):
        meta=self.metadata();plan=dict(schema=6,injections=[dict(meta[0],mod_id='pluto_native_creative')])
        GUARD.check_order(plan,meta)
        for kind in ('HEAD','RETURN','OVERWRITE','BEFORE_CALL','AFTER_CALL','MODIFY_ARG','WRAP_OPERATION'):
            other=dict(schema=6,injections=plan['injections']+[dict(target=meta[0]['target'],kind=kind)])
            with self.subTest(kind=kind),self.assertRaises(ValueError):GUARD.check_order(other,meta)
    def test_drift_and_missing_hooks_rejected(self):
        meta=self.metadata()
        for key,value in [('mod_id','other'),('kind','RETURN'),('priority',-1),('bindings',[{}]),('function_fingerprint','changed'),('file','other.rs'),('injection_id','other'),('cancellable',False)]:
            hook=dict(meta[0],mod_id='pluto_native_creative');hook[key]=value
            with self.subTest(key=key),self.assertRaises(ValueError):GUARD.check_order(dict(schema=6,injections=[hook]),meta)
        with self.assertRaises(ValueError):GUARD.check_order(dict(schema=6,injections=[]),meta)
    def test_plan_helper_and_pristine_source_are_bound(self):
        meta=self.metadata();hook=dict(meta[0],mod_id='pluto_native_creative',helper_source='helper')
        plan=dict(schema=6,injections=[hook],inputs={'native.rs':GUARD.digest(b'official')})
        meta[0].update(file_sha256=GUARD.digest(b'official'),helper_sha256=GUARD.digest(b'helper'))
        with tempfile.TemporaryDirectory() as d:
            p=Path(d);(p/'native.rs').write_text('official');(p/'helper.rs').write_text('helper')
            GUARD.verify_source(plan,p,meta,p)
            hook['helper_source']='different'
            with self.assertRaises(ValueError):GUARD.verify_source(plan,p,meta,p)
            hook['helper_source']='helper';(p/'native.rs').write_text('different')
            with self.assertRaises(ValueError):GUARD.verify_source(plan,p,meta,p)
if __name__=='__main__':unittest.main()
