"""Pure syntax-gate routing/receipt tests; formatter is an explicit test double."""
import ast
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch

TOOLS=Path(__file__).resolve().parents[1]/'tools';sys.path.insert(0,str(TOOLS))
import stage_rust_syntax as s

class Child:
    def __init__(self,code=0):self.code=code;self.killed=False
    def poll(self):return self.code
    def wait(self,timeout=None):return self.code
    def kill(self):self.killed=True;self.code=-9

class Syntax(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup);self.root=Path(self.temp.name)
        self.input=self.root/'input.rs';self.input.write_bytes(b'fn test() {}\n')
        self.formatter=self.root/'rustfmt';self.formatter.write_bytes(b'explicit test double')
        self.receipt=self.root/s.RECEIPT
    def run_gate(self):return s.parse_file(self.input,self.receipt,self.formatter)
    def test_exact_stdin_empty_config_nonwriting_invocation(self):
        calls=[]
        def child(argv,**kw):
            calls.append(argv);self.assertEqual(argv[1:5],['--edition','2024','--emit','stdout'])
            self.assertEqual(argv[5],'--config-path');self.assertEqual(Path(argv[6]).read_bytes(),b'')
            self.assertNotIn(str(self.input),argv);self.assertEqual(kw['stdin'].read(),self.input.read_bytes())
            self.assertEqual(kw['stdout'],subprocess.DEVNULL);return Child()
        with patch.object(s.subprocess,'Popen',side_effect=child):v=self.run_gate()
        self.assertEqual(len(calls),1);self.assertTrue(v['passed']);self.assertTrue(v['source_unchanged'])
        self.assertTrue(v['syntax_only']);self.assertFalse(v['type_checked']);self.assertFalse(v['writes_source'])
        self.assertEqual(json.loads(self.receipt.read_bytes()),v)
    def test_rejection_or_diagnostic_keeps_false_receipt(self):
        for code,diagnostic in [(1,b'bad syntax'),(0,b'warning')]:
            def child(*a,**kw):kw['stderr'].write(diagnostic);return Child(code)
            with patch.object(s.subprocess,'Popen',side_effect=child),self.assertRaises(ValueError):self.run_gate()
            v=json.loads(self.receipt.read_bytes());self.assertFalse(v['passed']);self.assertEqual(v['returncode'],code)
            self.assertEqual(self.input.read_bytes(),b'fn test() {}\n');self.receipt.unlink()
    def test_timeout_kills_same_child_before_temp_cleanup(self):
        child=Child(None)
        with patch.object(s.subprocess,'Popen',return_value=child),patch.object(s,'TIMEOUT',-1),self.assertRaises(TimeoutError):self.run_gate()
        self.assertTrue(child.killed);self.assertFalse(json.loads(self.receipt.read_bytes())['passed'])
        self.assertEqual([p for p in self.root.glob('rust-syntax-*') if p.is_dir()],[])
    def test_diagnostic_budget_rejects_without_unbounded_read(self):
        def child(*a,**kw):kw['stderr'].write(b'x'*(s.MAX_STDERR+1));return Child(1)
        with patch.object(s.subprocess,'Popen',side_effect=child),self.assertRaisesRegex(ValueError,'budget'):self.run_gate()
        self.assertFalse(json.loads(self.receipt.read_bytes())['passed'])
    def test_source_or_formatter_change_refuses(self):
        for target in (self.input,self.formatter):
            old=target.read_bytes()
            def child(*a,**kw):target.write_bytes(old+b'drift');return Child()
            with patch.object(s.subprocess,'Popen',side_effect=child),self.assertRaisesRegex(ValueError,'changed'):self.run_gate()
            self.assertFalse(json.loads(self.receipt.read_bytes())['passed']);self.receipt.unlink();target.write_bytes(old)
    def test_selected_route_requires_absolute_explicit_formatter(self):
        with patch.dict(os.environ,{'PLUTO_GUARD_RUSTFMT':'relative'},clear=False),self.assertRaises(ValueError):s.selected_formatter()
        with patch.dict(os.environ,{'PLUTO_GUARD_RUSTFMT':str(self.formatter)},clear=False):self.assertEqual(s.selected_formatter(),self.formatter)
    def test_fresh_receipt_and_input_budget_are_mandatory(self):
        self.receipt.write_bytes(b'existing')
        with self.assertRaises(ValueError):self.run_gate()
        self.receipt.unlink()
        with patch.object(s,'MAX_INPUT',1),self.assertRaises(ValueError):self.run_gate()

    def test_stage_order_is_guarded_apply_then_repair_then_parser(self):
        source=(TOOLS/'stage.py').read_bytes();tree=ast.parse(source)
        tail=next(x for x in tree.body if isinstance(x,ast.With)).body[-3:]
        self.assertEqual([ast.unparse(x.value.func) for x in tail],['subprocess.run','stage_float_range_repair.repair','stage_rust_syntax.check_generated'])
        self.assertTrue(next(k.value.value for k in tail[0].value.keywords if k.arg=='check'))
        for failure,expected in [('apply',['apply']),('repair',['apply','repair']),('syntax',['apply','repair','syntax']), (None,['apply','repair','syntax'])]:
            events=[]
            def call(name):
                def action(*a,**kw):
                    events.append(name)
                    if failure==name:raise ValueError(name)
                return action
            scope={'subprocess':SimpleNamespace(run=call('apply')),'stage_float_range_repair':SimpleNamespace(repair=call('repair')),
                   'stage_rust_syntax':SimpleNamespace(check_generated=call('syntax')),'output':self.root,'original':self.root,
                   'source':self.root,'plan':self.root/'plan','head':'h','revision':'m','cli':self.root/'cli','combined':self.root}
            code=compile(ast.Module(body=tail,type_ignores=[]),'<actual-stage-tail>','exec')
            if failure:
                with self.assertRaisesRegex(ValueError,failure):exec(code,scope)
            else:exec(code,scope)
            self.assertEqual(events,expected)
    def test_all_five_prior_sources_restore_exactly_and_stage_pin_matches(self):
        from windows_source_layers import before_rust_syntax
        root=TOOLS.parents[1];delta=json.loads((root/'tconstruct/experiments/stage-rust-syntax/inverse.json').read_bytes())
        self.assertEqual(len(delta['files']),5)
        for path,row in delta['files'].items():
            raw=(root/path).read_bytes();self.assertEqual(s.sha(before_rust_syntax(path,raw)),row['before_sha256'])
            with self.assertRaises(AssertionError):before_rust_syntax(path,raw+b'\n')
        tree=ast.parse((TOOLS/'play_windows_latest.py').read_bytes())
        pin=next(n.value.value for n in tree.body if isinstance(n,ast.Assign) and any(isinstance(x,ast.Name) and x.id=='STAGE_SHA' for x in n.targets))
        self.assertEqual(pin,s.sha((TOOLS/'stage.py').read_bytes()))

if __name__=='__main__':unittest.main()
