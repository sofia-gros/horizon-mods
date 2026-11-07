"""Pure connection/inverse tests; no Git, formatter, planner or compiler."""
import ast
import hashlib
import json
from pathlib import Path
from types import SimpleNamespace
import unittest

from windows_source_layers import before_float_range, before_rust_syntax, undo

ROOT=Path(__file__).resolve().parents[1]


def tail():
    tree=ast.parse(before_rust_syntax('tconstruct/tools/stage.py',(ROOT/'tools/stage.py').read_bytes()))
    block=next(n for n in tree.body if isinstance(n,ast.With))
    return block.body[-2:]


class Connection(unittest.TestCase):
    def test_runtime_and_test_adapters_reverse_full_exact_base(self):
        value=json.loads((ROOT/'experiments/stage-float-range/inverse.json').read_bytes())
        self.assertEqual(value['base'],'9e032ef5b013487ea08b997eb08df84825fb09ba')
        self.assertEqual(len(value['files']),6)
        for rel,row in value['files'].items():
            raw=(ROOT.parent/rel).read_bytes()
            self.assertEqual(hashlib.sha256(before_float_range(rel,raw)).hexdigest(),row['before_sha256'])
            with self.assertRaises(AssertionError):before_float_range(rel,raw+b'\n')
            broken=dict(row,edits=row['edits'][1:])
            with self.assertRaises(AssertionError):undo(raw,broken)

    def test_one_unconditional_repair_follows_successful_apply(self):
        statements=tail();self.assertEqual(len(statements),2)
        apply,repair=[n.value for n in statements]
        self.assertEqual(ast.unparse(apply.func),'subprocess.run')
        self.assertIn("'apply'",ast.unparse(apply.args[0]))
        self.assertTrue(next(k.value.value for k in apply.keywords if k.arg=='check'))
        self.assertEqual(ast.unparse(repair.func),'stage_float_range_repair.repair')
        self.assertEqual([ast.unparse(a) for a in repair.args],['original',"output / 'modded'",'source','plan'])
        self.assertEqual({k.arg:ast.unparse(k.value) for k in repair.keywords},{'horizon_revision':'head','mixins_revision':'revision'})
        source=(ROOT/'tools/stage.py').read_text(encoding='utf-8')
        self.assertEqual(source.count('stage_float_range_repair.repair('),1)
        self.assertLess(source.index('check_native_plan.py'),source.index("'apply', '--workspace'"))

    def execute_tail(self, fail=None):
        events=[];out=Path('/fixture/output')
        def apply(*args,**kw):
            events.append('apply')
            if fail=='apply':raise ValueError('apply refused')
        def repair(*args,**kw):
            events.append('repair')
            self.assertEqual(args,(out/'original',out/'modded',out/'mixins-source',out/'complete-plan.json'))
            self.assertEqual(kw,{'horizon_revision':'h','mixins_revision':'m'})
            if fail=='repair':raise ValueError('repair refused')
        scope={'subprocess':SimpleNamespace(run=apply),'stage_float_range_repair':SimpleNamespace(repair=repair),
               'cli':Path('/cli'),'original':out/'original','combined':out/'combined','output':out,
               'source':out/'mixins-source','plan':out/'complete-plan.json','head':'h','revision':'m'}
        code=compile(ast.Module(body=tail(),type_ignores=[]),'<real-stage-tail>','exec')
        if fail:
            with self.assertRaisesRegex(ValueError,fail+' refused'):exec(code,scope)
        else:exec(code,scope)
        return events

    def test_apply_failure_never_repairs_and_repair_failure_propagates(self):
        self.assertEqual(self.execute_tail('apply'),['apply'])
        self.assertEqual(self.execute_tail('repair'),['apply','repair'])
        self.assertEqual(self.execute_tail(),['apply','repair'])

    def test_launcher_pins_stage_and_tools_in_source_cache_identity(self):
        source=(ROOT/'tools/play_windows_latest.py').read_text(encoding='utf-8')
        tree=ast.parse(source)
        pin=next(n.value.value for n in tree.body if isinstance(n,ast.Assign) and any(isinstance(x,ast.Name) and x.id=='STAGE_SHA' for x in n.targets))
        self.assertEqual(pin,hashlib.sha256((ROOT/'tools/stage.py').read_bytes()).hexdigest())
        self.assertIn("('mixins','content','assets','tools','third_party','docs')",source)
        self.assertIn("'stage_sha256':sha(read(repo/'tools/stage.py'))",source)
        self.assertLess(source.index("repo/'tools/stage.py','--horizon'"),source.index('run(compile_args('))

    def test_stage_cli_arguments_are_not_changed(self):
        raw=(ROOT/'tools/stage.py').read_bytes();old=before_float_range('tconstruct/tools/stage.py',raw)
        def arguments(data):
            return [ast.dump(n,include_attributes=False) for n in ast.walk(ast.parse(data))
                    if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute) and n.func.attr=='add_argument']
        self.assertEqual(arguments(raw),arguments(old))

    def test_ordinary_export_fixture_has_explicit_repair_double_and_order_assertion(self):
        source=(ROOT/'tests/test_stage.py').read_text(encoding='utf-8')
        self.assertIn("tools/stage_float_range_repair.py",source)
        self.assertIn("(generated / 'applied').read_text() == 'applied'",source)
        self.assertIn("(generated.parent / 'preflight-passed').read_text() == 'passed'",source)
        self.assertIn("{'fixture_only': True, 'after_apply': True}",source)
        self.assertIn("(rejected_output / 'float-range-repair-receipt.json').exists()",source)

if __name__=='__main__':unittest.main()
