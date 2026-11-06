"""Source-only guards; no Cargo, metadata, native execution or output stage."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib
import unittest

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
INPUTS=json.loads((HERE/'inputs.json').read_text())
HOST=None

class PackChainSource(unittest.TestCase):
    def test_six_exact_cases_and_frozen_fixture(self):
        for path,digest in INPUTS['fixture_inputs'].items():
            self.assertEqual(hashlib.sha256((HERE/path).read_bytes()).hexdigest(),digest,path)
        text=(HERE/'fixture.rs').read_text()
        names=['tests::'+n for n in re.findall(r'#\[test\]\s+fn (\w+)\(',text)]
        self.assertEqual(names,INPUTS['expected_test_names']);self.assertEqual(len(set(names)),6)
        self.assertNotIn('#[ignore]',text)

    def test_real_owners_not_copied_private_merge(self):
        text=(HERE/'fixture.rs').read_text()
        for token in ['packs::folder(', 'packs::enabled(', 'packs::server_pack(', 'packs::build_overlay(', 'mod_content::build(', 'Package::write_files(', 'Package::read(', 'Content::read(', 'Language::load(&[&']:
            self.assertIn(token,text)
        for token in ['struct Language','struct Pack ','impl ResourceSource','fn merge(', 'build_overlay_uncached(', 'serde_json::', 'lang::set(', 'Command::new']:
            self.assertNotIn(token,text)

    def test_explicit_order_filters_and_failure_not_skips(self):
        text=(HERE/'fixture.rs').read_text()
        for token in ['s.selected(&["z", "a"])','selected.push(server)', '"filter":{"block"', 'do not silently skip a fixture pack', 'assert!(selected.iter().all', 'not json', 'do not resurrect filtered source keys']:
            self.assertIn(token,text)
        self.assertNotIn('if let Ok(',text)

    def test_owned_outputs_and_cache_delegation(self):
        text=(HERE/'fixture.rs').read_text()
        for token in ['fs::create_dir(&path)', 'fs::remove_dir_all(&self.0)', 'Entry::Link(fs::read_link(&path)', 'packs::build_overlay(&base, &selected, &root)', 'assert_eq!(tree(&root), before)']:
            self.assertIn(token,text)
        self.assertNotIn('remove_file(',text)

    def test_actual_dependencies_and_existing_zip_identity(self):
        c=tomllib.loads((HERE/'Cargo.toml.in').read_text())
        for name in ['horizon-client-lib','horizon-client-core','horizon-client-render','horizon-mods']:
            self.assertEqual(c['dependencies'][name]['path'],'@OFFICIAL@/crates/'+name)
        self.assertEqual(c['dependencies']['zip'],{'version':'=9.1.3','default-features':False,'features':['deflate']})
        packages=tomllib.loads((HOST/'Cargo.lock').read_text())['package']
        actual=[p for p in packages if p['name']=='zip' and p['version']=='9.1.3']
        self.assertEqual(len(actual),1)
        self.assertEqual({k:actual[0].get(k) for k in ['name','version','source','checksum']},INPUTS['zip_identity'])

    def test_exact_approved_official_and_unchanged_prior_inputs(self):
        env={**os.environ,'GIT_OPTIONAL_LOCKS':'0'}
        def git(*args):return subprocess.check_output(['git','-C',str(HOST),*args],env=env)
        self.assertEqual(git('remote','get-url','origin').decode().strip(),INPUTS['official_url'])
        self.assertEqual(git('rev-parse','HEAD').decode().strip(),INPUTS['horizon'])
        for item in INPUTS['official_inputs']:
            data=(HOST/item['path']).read_bytes();self.assertEqual(data,git('show',INPUTS['horizon']+':'+item['path']))
            self.assertEqual(hashlib.sha256(data).hexdigest(),item['sha256'],item['path'])
        for path,digest in INPUTS['unchanged_prior_inputs'].items():
            self.assertEqual(hashlib.sha256((ROOT/path).read_bytes()).hexdigest(),digest,path)

    def test_no_execution_or_repair_claim(self):
        self.assertFalse(INPUTS['execution']['run_now'])
        text=(HERE/'README.md').read_text()
        for token in ['uncompiled and unrun','No runner or trigger','not prove a cache hit','Game admission/pack settings','remains unimplemented']:
            self.assertIn(token,text)

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--host',required=True,type=Path)
    HOST=parser.parse_args().host.resolve();unittest.main(argv=[__file__],verbosity=2)
