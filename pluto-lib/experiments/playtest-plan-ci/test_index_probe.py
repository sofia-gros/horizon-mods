"""Synthetic admission tests only. No Cargo, metadata command, Rust or Index run."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('index_probe', HERE / 'index_probe.py')
P = importlib.util.module_from_spec(spec); spec.loader.exec_module(P)
REG = 'registry+https://github.com/rust-lang/crates.io-index'
CORE_MANIFEST = b'[package]\nname="horizon-mixins-core"\nversion="0.6.0"\n'
MAIN = b'fn main() {}\n'


def lock(entries):
    lines = ['version = 4\n']
    for e in entries:
        lines.append('[[package]]\n')
        for k, v in e.items(): lines.append(k + ' = ' + json.dumps(v) + '\n')
    return ''.join(lines).encode()


def fixtures():
    core = {'name':P.CORE,'version':P.CORE_VERSION,'dependencies':['serde_json']}
    serde = {'name':'serde_json','version':'1.0.154','source':REG,'checksum':'a'*64}
    unused = {'name':'unused-registry','version':'1.0.0','source':REG,'checksum':'b'*64}
    seed = lock([core,serde,unused])
    entries = [core,serde,{'name':P.NAME,'version':P.VERSION,'dependencies':[P.CORE,'serde_json']}]
    return seed, entries


def metadata(root, mixins):
    entries = [
        {'id':'core-id','name':P.CORE,'version':P.CORE_VERSION,'source':None,'manifest_path':str(mixins/'crates/horizon-mixins-core/Cargo.toml'),'targets':[{'kind':['lib'],'crate_types':['lib'],'name':'horizon_mixins_core','src_path':str(mixins/'crates/horizon-mixins-core/src/lib.rs')}]},
        {'id':'serde-id','name':'serde_json','version':'1.0.154','source':REG,'manifest_path':'/registry/serde_json/Cargo.toml','targets':[]},
        {'id':'harness-id','name':P.NAME,'version':P.VERSION,'source':None,'manifest_path':str(root/'Cargo.toml'),'targets':[{'kind':['bin'],'crate_types':['bin'],'name':P.NAME,'src_path':str(root/'src/main.rs')}]},
    ]
    return {'version':1,'workspace_root':str(root),'workspace_members':['harness-id'],'workspace_default_members':['harness-id'],'packages':entries,
            'resolve':{'root':'harness-id','nodes':[{'id':'core-id','features':[],'dependencies':['serde-id'],'deps':[{'pkg':'serde-id'}]}, {'id':'serde-id','features':[],'dependencies':[],'deps':[]}, {'id':'harness-id','features':[],'dependencies':['core-id','serde-id'],'deps':[{'pkg':'core-id'},{'pkg':'serde-id'}]}]}}


def result():
    return {'schema':1,'targets':[{'target':t,'file':f,'resolved_id':t,'node':n if n is not None else [44,67], 'function_fingerprint':fp or 'f'*64} for t,f,n,fp in P.TARGETS]}


class ProbeTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name).resolve(); self.mixins=self.base/'mixins'; self.core=self.mixins/'crates/horizon-mixins-core'
        self.core.mkdir(parents=True); (self.core/'Cargo.toml').write_bytes(CORE_MANIFEST)
        self.seed,self.entries=fixtures(); (self.mixins/'Cargo.lock').write_bytes(self.seed)
        self.main=self.base/'main.rs';self.main.write_bytes(MAIN);self.root=self.base/'probe'
        for key, value in [('LOCK_SHA',P.sha(self.seed)),('CORE_MANIFEST_SHA',P.sha(CORE_MANIFEST)),('MAIN_SHA',P.sha(MAIN))]:
            active=patch.object(P,key,value);active.start();self.addCleanup(active.stop)

    def verify(self, value=None):
        self.root.mkdir(exist_ok=True)
        return P.verify_metadata(value or metadata(self.root,self.mixins),self.root,self.mixins,lock(self.entries),self.seed)

    def test_fresh_manifest_preserves_exact_source_paths(self):
        facts=P.prepare(self.root,self.mixins,self.main)
        parsed=P.tomllib.loads((self.root/'Cargo.toml').read_text())
        self.assertEqual(parsed['workspace']['default-members'],['.'])
        self.assertEqual(parsed['dependencies'][P.CORE]['path'],str(self.core))
        self.assertEqual(parsed['dependencies']['serde_json'],'=1.0.154')
        self.assertEqual((self.root/'Cargo.lock').read_bytes(),self.seed)
        self.assertEqual(facts['main_sha256'],P.sha(MAIN))

    def test_preexisting_root_and_symlink_rejected(self):
        self.root.mkdir()
        with self.assertRaises(ValueError):P.prepare(self.root,self.mixins,self.main)
        self.root.rmdir();self.root.symlink_to(self.base/'missing')
        with self.assertRaises(ValueError):P.prepare(self.root,self.mixins,self.main)

    def test_inside_official_tree_rejected(self):
        with self.assertRaises(ValueError):P.prepare(self.mixins/'probe',self.mixins,self.main)

    def test_each_source_pin_rejects_drift_before_creation(self):
        for path in [self.main,self.mixins/'Cargo.lock',self.core/'Cargo.toml']:
            before=path.read_bytes();path.write_bytes(before+b'\n')
            with self.assertRaises(ValueError):P.prepare(self.root,self.mixins,self.main)
            self.assertFalse(self.root.exists());path.write_bytes(before)

    def test_core_symlink_is_rejected(self):
        moved=self.base/'moved';self.core.rename(moved);self.core.symlink_to(moved)
        with self.assertRaises(ValueError):P.prepare(self.root,self.mixins,self.main)

    def test_lock_subset_plus_one_harness_is_allowed(self):
        self.assertEqual(P.verify_lock(lock(self.entries),self.seed)['packages'],3)

    def test_lock_entry_version_checksum_edges_cannot_change(self):
        for i,key,value in [(0,'version','0.6.1'),(0,'dependencies',[]),(1,'checksum','0'*64),(1,'source','registry+other')]:
            entries=copy.deepcopy(self.entries);entries[i][key]=value
            with self.assertRaises(ValueError):P.verify_lock(lock(entries),self.seed)

    def test_unknown_local_or_registry_package_rejected(self):
        for entry in [{'name':'extra','version':'1.0.0'}, {'name':'extra','version':'1.0.0','source':REG,'checksum':'c'*64}]:
            with self.assertRaises(ValueError):P.verify_lock(lock(self.entries+[entry]),self.seed)

    def test_missing_required_or_duplicate_lock_package_rejected(self):
        for i in range(3):
            with self.assertRaises(ValueError):P.verify_lock(lock(self.entries[:i]+self.entries[i+1:]),self.seed)
        with self.assertRaises(ValueError):P.verify_lock(lock(self.entries+[self.entries[0]]),self.seed)

    def test_wrong_lock_seed_or_harness_dependencies_rejected(self):
        with self.assertRaises(ValueError):P.verify_lock(lock(self.entries),self.seed+b'\n')
        entries=copy.deepcopy(self.entries);entries[2]['dependencies']=[P.CORE]
        with self.assertRaises(ValueError):P.verify_lock(lock(entries),self.seed)

    def test_full_metadata_exact_two_local_sources(self):
        self.assertEqual(self.verify()['core_id'],'core-id')

    def test_metadata_local_source_version_target_redirect_rejected(self):
        for key,value in [('manifest_path','/other/Cargo.toml'),('version','9.0.0'),('targets',[{'kind':['lib'],'src_path':'/other.rs'}])]:
            m=metadata(self.root,self.mixins);m['packages'][0][key]=value
            with self.assertRaises(ValueError):self.verify(m)
        for key,value in [('name','other'),('crate_types',['cdylib'])]:
            m=metadata(self.root,self.mixins);m['packages'][0]['targets'][0][key]=value
            with self.assertRaises(ValueError):self.verify(m)
        m=metadata(self.root,self.mixins);m['packages'][2]['targets'].append({'kind':['custom-build']})
        with self.assertRaises(ValueError):self.verify(m)

    def test_workspace_member_and_default_must_be_only_harness(self):
        for key,value in [('workspace_root',str(self.mixins)),('workspace_members',['harness-id','core-id']),('workspace_default_members',[])]:
            m=metadata(self.root,self.mixins);m[key]=value
            with self.assertRaises(ValueError):self.verify(m)

    def test_duplicate_package_id_or_second_identity_rejected(self):
        for new_id in ['core-id','other-core-id']:
            m=metadata(self.root,self.mixins);p=copy.deepcopy(m['packages'][0]);p['id']=new_id;m['packages'].append(p)
            with self.assertRaises(ValueError):self.verify(m)

    def test_missing_or_unknown_resolved_graph_rejected(self):
        for mutation in ['missing','edge','root','features']:
            m=metadata(self.root,self.mixins)
            if mutation=='missing':m['resolve']['nodes'].pop()
            elif mutation=='edge':m['resolve']['nodes'][0]['dependencies'].append('unknown')
            elif mutation=='root':m['resolve']['root']='core-id'
            else:m['resolve']['nodes'][0]['features']=[1]
            with self.assertRaises(ValueError):self.verify(m)

    def test_three_result_controls_and_unknown_poll_are_separate(self):
        expected=result();actual=P.validate_result((json.dumps(expected)+'\n').encode());self.assertEqual(actual,expected)

    def test_result_control_or_target_or_file_drift_rejected(self):
        for key,value in [('function_fingerprint','f'*64),('target','other'),('file','other.rs'),('node',[0])]:
            x=result();x['targets'][0][key]=value
            with self.assertRaises(ValueError):P.validate_result((json.dumps(x)+'\n').encode())

    def test_result_node_types_bounds_and_resolved_ids_rejected(self):
        for node in [[True],[-1],[],[2**31],[1]*17]:
            x=result();x['targets'][2]['node']=node
            with self.assertRaises(ValueError):P.validate_result((json.dumps(x)+'\n').encode())
        x=result();x['targets'][2]['resolved_id']=x['targets'][0]['resolved_id']
        with self.assertRaises(ValueError):P.validate_result((json.dumps(x)+'\n').encode())

    def test_result_whole_stream_duplicate_fields_numbers_and_limit(self):
        raw=(json.dumps(result())+'\n').encode()
        for bad in [raw+b'\n',b'warning\n'+raw,raw[:-1],b' '*16385+b'\n',raw.replace(b'"schema": 1',b'"schema": 1, "schema": 1'),raw.replace(b'"schema": 1',b'"schema": NaN'),raw.replace(b'"schema": 1',b'"schema": 1e999')]:
            with self.assertRaises(ValueError):P.validate_result(bad)

    def test_result_missing_extra_or_reordered_rows_and_fields(self):
        for mutation in ['missing','extra','order','field','schema']:
            x=result()
            if mutation=='missing':x['targets'].pop()
            elif mutation=='extra':x['targets'].append(copy.deepcopy(x['targets'][0]))
            elif mutation=='order':x['targets'].reverse()
            elif mutation=='field':x['targets'][0]['extra']='ignored'
            else:x['schema']=True
            with self.assertRaises(ValueError):P.validate_result((json.dumps(x)+'\n').encode())


if __name__ == '__main__':unittest.main()
