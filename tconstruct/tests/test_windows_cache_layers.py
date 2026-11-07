"""Historical cache witness updates only; no PowerShell or native execution."""
import copy
import hashlib
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import test_windows_pluto_cache as cache
from windows_source_layers import undo

ROOT = Path(__file__).resolve().parents[1]
EXTRA = """    raw=(ROOT/'tools/play-windows.ps1').read_bytes()
    for name in ('windows-link-only-inverse.json', 'windows-formatter-entry-inverse.json'):
        row=json.loads((ROOT/'tests'/name).read_bytes())['files']['tools/play-windows.ps1']
        raw=undo(raw,row)
    text=raw.decode('utf-8')
"""


class CacheLayers(unittest.TestCase):
    def test_old_test_body_and_three_witnesses_are_unchanged(self):
        text=(ROOT/'tests/test_windows_pluto_cache.py').read_text(encoding='utf-8')
        self.assertEqual(text.count('from windows_source_layers import undo\n'),1)
        text=text.replace('from windows_source_layers import undo\n','',1)
        self.assertEqual(text.count(EXTRA),1)
        text=text.replace(EXTRA,"    text=(ROOT/'tools/play-windows.ps1').read_text(encoding='utf-8')\n",1)
        self.assertEqual(hashlib.sha256(text.encode('utf-8')).hexdigest(),'f443c48cc3f4f55c00a3ac811cedb6528660e12b906396124c6fd2c36f48f9a0')
        for name,expected in {
            'windows-link-only-inverse.json':'06f699d8953790faa94f42a5172ad35399124104405d31460b72a61a7e3fc73d',
            'windows-latest-entry-inverse.json':'b0c409df288de7bcbdf62ccdb3fa8d24db05dd734a90d58bd15c01345e79c88b',
            'windows-pluto-cache-inverse.json':'cab985cf09edbb141fa7d6db342234e13353d590fb5ba37c56d0bd67cb092f06',
        }.items():
            self.assertEqual(hashlib.sha256((ROOT/'tests'/name).read_bytes()).hexdigest(),expected)

    def test_both_layers_reject_drift_missing_edit_wrong_order_and_prior_hash(self):
        raw=(ROOT/'tools/play-windows.ps1').read_bytes()
        rows=[json.loads((ROOT/'tests'/n).read_bytes())['files']['tools/play-windows.ps1']
              for n in ('windows-link-only-inverse.json','windows-formatter-entry-inverse.json')]
        with self.assertRaises(AssertionError):undo(raw,rows[1])
        for row in rows:
            for changed in (raw+b'\n', raw.replace(b'\n',b'\r\n')):
                with self.assertRaises(AssertionError):undo(changed,row)
            changed=copy.deepcopy(row);changed['edits']=changed['edits'][1:]
            with self.assertRaises(AssertionError):undo(raw,changed)
            changed=copy.deepcopy(row);changed['before_sha256']='0'*64
            with self.assertRaises(AssertionError):undo(raw,changed)
            raw=undo(raw,row)
        old=json.loads((ROOT/'tests/windows-latest-entry-inverse.json').read_bytes())
        self.assertEqual(hashlib.sha256(raw).hexdigest(),old['candidate_script_sha256'])
        self.assertEqual(hashlib.sha256(cache.legacy_script().encode('utf-8')).hexdigest(),old['base_script_sha256'])

    def test_legacy_entry_never_ignores_missing_layer_or_changed_current_source(self):
        original=Path.read_bytes
        current=ROOT/'tools/play-windows.ps1'
        for missing in ('windows-link-only-inverse.json','windows-formatter-entry-inverse.json'):
            def read(path):
                if path==ROOT/'tests'/missing:raise FileNotFoundError('fixture missing layer')
                return original(path)
            with patch.object(Path,'read_bytes',read), self.assertRaises(FileNotFoundError):cache.legacy_script()
        def changed(path):return original(path)+(b'\n' if path==current else b'')
        with patch.object(Path,'read_bytes',changed),self.assertRaises(AssertionError):cache.legacy_script()


if __name__=='__main__':unittest.main()
