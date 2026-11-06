"""Pure source and hash-contract checks. No Rust or Mixins execution."""
import ast
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import struct
import tempfile
import tomllib
import unittest

HERE=Path(__file__).resolve().parent
s=importlib.util.spec_from_file_location('root_source',HERE/'source.py');m=importlib.util.module_from_spec(s);s.loader.exec_module(m)
HOST=Path(os.environ['PLUTO_SELECTED_SOURCE']);REPAIR=Path(os.environ['PLUTO_REPAIR_SOURCE'])
C=json.loads((HERE/'inputs.json').read_bytes())

def transcript(parts):
    return b''.join(struct.pack('<Q',len(p.encode()))+p.encode() for p in parts)

class SourceTests(unittest.TestCase):
    def test_complete_exact_native_inverse(self):
        output,receipt=m.compose(HOST,REPAIR)
        self.assertEqual(m.inverse(output,(HERE/'root-store.rs').read_bytes()),(HOST/m.TARGET).read_bytes())
        self.assertTrue(receipt['full_native_inverse'])
        self.assertFalse(any(receipt[k] for k in ['mixins_plan_accepted','rust_executed','writer_connected','actual_cache_tested','full_composition_tested','game_ui_tested']))
    def test_scope_is_one_out_store_without_writer_or_module(self):
        v=tomllib.loads((HERE/'horizon-mixin.toml').read_text())
        self.assertEqual(set(v),{'mod','compatibility','injection'})
        self.assertEqual(v['injection'],[C['injection']])
        self.assertEqual(C['injection']['variable'],{'name':'out','ordinal':0,'at':'STORE'})
        self.assertEqual(C['injection']['bindings'][1],{'name':'stamp','ty':'&String','value':'&stamp'})
    def test_native_stamp_parent_compare_write_and_prune_order_unchanged(self):
        native=(HOST/m.TARGET).read_bytes();after,_=m.compose(HOST,REPAIR)
        old=m.caller(native);new=m.caller(after)
        self.assertEqual(new.replace(m.NEW,m.OLD),old)
        witnesses=[b'return Ok(base.to_owned());',b'let stamp = stamp(base, mods)?;',
            b'let roots = cache.join("mod-content");',m.NEW.strip(),
            b'fs::read(out.join(STAMP))',b'fs::write(out.join(STAMP), &stamp)',
            b'build(base, mods, &out)?',b'fs::write(out.join(STAMP), stamp)?',
            b'prune_roots(&roots, &out, KEEP_ROOTS);']
        offsets=[new.index(w) for w in witnesses];self.assertEqual(offsets,sorted(offsets))
        stamp=native[native.index(b'fn stamp('):native.index(b'\n#[cfg(test)]',native.index(b'fn stamp('))]
        self.assertIn(stamp,after)
    def test_full_stamp_and_policy_are_length_prefixed_before_hash(self):
        helper=(HERE/'root-store.rs').read_text()
        self.assertIn('hasher.update((part.len() as u64).to_le_bytes());',helper)
        self.assertIn('hasher.update(part.as_bytes());',helper)
        self.assertLess(helper.index('stamp.as_str()'),helper.index('hasher.finalize()'))
        self.assertNotIn('stamp[..',helper)
        self.assertIn('out.set_file_name(&separated[..ROOT_NAME]);',helper)
        self.assertNotEqual(transcript(['ab','c']),transcript(['a','bc']))
    def test_policy_suffix_and_native_suffix_affect_the_full_digest(self):
        # Independent hash-contract examples, not execution of the Rust helper.
        parts=['plutolib language root policy 1',C['policy_id'],'0'*64]
        hashes=[hashlib.sha256(transcript(p)).hexdigest() for p in [parts,
            [parts[0],parts[1]+' next',parts[2]], [*parts[:2],'0'*63+'1']]]
        self.assertEqual(len(set(hashes)),3)
        self.assertEqual(hashes[0],'b89008df129e2200e5ef52d530aced166e158a8e05782908a7f2d340cff47d39')
    def test_policy_is_exact_existing_helper_value(self):
        m.compose(HOST,REPAIR)
        helper=(HERE/'root-store.rs').read_text()
        self.assertEqual(helper.count('"'+C['policy_id']+'"'),1)
        self.assertNotIn('static ',helper)
        self.assertNotIn('std::fs',helper)
        self.assertNotIn('fs::',helper)
        self.assertNotIn('unwrap',helper)
    def test_invalid_native_caller_or_inverse_rejects(self):
        with self.assertRaises(ValueError):m.caller(b'not selected native')
        output,_=m.compose(HOST,REPAIR);helper=(HERE/'root-store.rs').read_bytes()
        for bad in [output+b'x',output.replace(m.NEW,m.OLD),output+output]:
            with self.assertRaises(ValueError):m.inverse(bad,helper)
    def test_symlink_and_hash_mismatch_reject(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp);(p/'source').write_bytes(b'x');(p/'link').symlink_to(p/'source')
            self.assertEqual(m.read(p,'source',m.sha(b'x')),b'x')
            for rel,h in [('source','0'*64),('link',m.sha(b'x')),('../source',m.sha(b'x'))]:
                with self.assertRaises(ValueError):m.read(p,rel,h)
    def test_preparation_has_no_execution_or_output_path(self):
        tree=ast.parse((HERE/'source.py').read_text())
        imports={a.name for n in ast.walk(tree) if isinstance(n,ast.Import) for a in n.names}
        self.assertFalse(imports&{'subprocess','shutil'})
        attrs={n.func.attr for n in ast.walk(tree) if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute)}
        self.assertFalse(attrs&{'write_bytes','write_text','mkdir','unlink','Popen','run'})

if __name__=='__main__':unittest.main()
