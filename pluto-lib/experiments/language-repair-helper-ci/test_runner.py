"""Pure controller regressions; no Git/Cargo/Rust or native execution."""
import ast,copy,importlib.util,json,os,tempfile,tomllib,unittest
from pathlib import Path
from unittest.mock import patch
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('helper_controller',HERE/'run.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)

def graph(host,out):
    packages=[{'name':m.prepare.NAME,'version':'0.0.0','source':None,'id':'own','manifest_path':str(out/'Cargo.toml'),
               'targets':[{'kind':['lib'],'src_path':str(out/'fixture.rs')}] }]
    for name in ['serde','serde_json']:
        packages.append({'name':name,'version':'1.0.0','source':'registry+https://github.com/rust-lang/crates.io-index',
                         'id':name,'manifest_path':str(host/name/'Cargo.toml'),'targets':[{'kind':['lib'],'src_path':str(host/name/'src/lib.rs')}]})
    lock={'package':[{'name':p['name'],'version':p['version'],**({'source':p['source'],'checksum':'a'*64} if p['source'] else {})} for p in packages]}
    return {'packages':packages,'workspace_members':['own']},lock
def lock_text(lock):
    return ''.join('[[package]]\n'+''.join(k+'='+json.dumps(v)+'\n' for k,v in p.items()) for p in lock['package'])
def compiler_log(meta):
    lines=[]
    for p in meta['packages']:
        name=p['name'].replace('-','_');flag=' --test' if p['id']=='own' else ''
        lines.append('Running `rustc --crate-name '+name+' '+p['targets'][0]['src_path']+flag+'`')
    return '\n'.join(lines)

class RunnerTests(unittest.TestCase):
    def test_exact_current_frozen_inputs(self):m.validate(m.config())
    def test_wrong_official_url_revision_and_lock_reject(self):
        for key in ['url','revision','lock_sha256']:
            c=copy.deepcopy(m.config());c['official'][key]='wrong'
            with self.assertRaises(ValueError):m.validate(c)
    def test_wrong_source_or_branch_reject(self):
        for key in ['repository','branch','source']:
            c=copy.deepcopy(m.config());c[key]='wrong'
            with self.assertRaises(ValueError):m.validate(c)
    def test_wrong_source_file_set_and_tests_reject(self):
        c=copy.deepcopy(m.config());c['source_sha256'].pop(next(iter(c['source_sha256'])))
        with self.assertRaises(ValueError):m.validate(c)
        c=copy.deepcopy(m.config());c['tests']=c['tests'][:-1]
        with self.assertRaises(ValueError):m.validate(c)
    def test_dedicated_public_push_only(self):
        env={'GITHUB_ACTIONS':'true','GITHUB_REPOSITORY':m.REPOSITORY,'GITHUB_EVENT_NAME':'push','GITHUB_REF':'refs/heads/'+m.BRANCH}
        event={'repository':{'private':False}}
        self.assertTrue(m.event_allowed(event,env))
        for key in env:
            changed=dict(env);changed[key]='wrong';self.assertFalse(m.event_allowed(event,changed))
        self.assertFalse(m.event_allowed({'repository':{'private':True}},env))
    def test_actual_serde_and_fixture_compiler_paths(self):
        meta,_=graph(Path('/official'),Path('/fixture'));rows=m.compilers(compiler_log(meta),meta,Path('/fixture'))
        self.assertEqual(len(rows),3);self.assertEqual(sum(r['test'] for r in rows),1)
    def test_missing_duplicate_and_wrong_compile_source_reject(self):
        meta,_=graph(Path('/official'),Path('/fixture'));log=compiler_log(meta)
        for changed in ['',log+'\n'+log,log.replace('/fixture/fixture.rs','/other.rs'),log.replace(' --test',''),log+'\nRunning `rustc --crate-name horizon_world /x.rs`']:
            with self.assertRaises(ValueError):m.compilers(changed,meta,Path('/fixture'))
    def test_exact_complete_fourteen_names(self):
        names=m.config()['tests'];log='\n'.join('test '+n+' ... ok' for n in names)+'\ntest result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n'
        self.assertEqual(set(m.base.validate_suite(log,14,names)),set(names))
        for changed in [log.replace('0 filtered','1 filtered'),log.replace(' ... ok',' ... ignored',1),log+'test '+names[0]+' ... ok\n']:
            with self.assertRaises(ValueError):m.base.validate_suite(changed,14,names)
    def test_official_lock_subset_and_only_fixture_local(self):
        with tempfile.TemporaryDirectory() as tmp:
            host=Path(tmp);out=host/'fixture';meta,lock=graph(host,out)
            (host/'Cargo.lock').write_text(lock_text({'package':lock['package'][1:]}))
            m.prepare.validate_dependencies(host,out,meta,lock)
            altered=copy.deepcopy(lock);altered['package'][1]['version']='9.9.9'
            with self.assertRaises(ValueError):m.prepare.validate_dependencies(host,out,meta,altered)
    def test_unknown_or_redirected_local_package_rejects(self):
        with tempfile.TemporaryDirectory() as tmp:
            host=Path(tmp);out=host/'fixture';meta,lock=graph(host,out)
            (host/'Cargo.lock').write_text(lock_text({'package':lock['package'][1:]}))
            changed=copy.deepcopy(meta);changed['packages'][0]['manifest_path']=str(host/'bad/Cargo.toml')
            with self.assertRaises(ValueError):m.prepare.validate_dependencies(host,out,changed,lock)
            changed=copy.deepcopy(meta);extra=copy.deepcopy(changed['packages'][0]);extra['name']='unknown';extra['id']='extra';changed['packages'].append(extra)
            with self.assertRaises(ValueError):m.prepare.validate_dependencies(host,out,changed,lock)
            changed=copy.deepcopy(meta);changed['packages'][1]['source']=None
            with self.assertRaises(ValueError):m.prepare.validate_dependencies(host,out,changed,lock)
    def test_wrong_registry_and_workspace_rejects(self):
        with tempfile.TemporaryDirectory() as tmp:
            host=Path(tmp);out=host/'fixture';meta,lock=graph(host,out)
            (host/'Cargo.lock').write_text(lock_text({'package':lock['package'][1:]}))
            for field in ['registry','workspace']:
                changed=copy.deepcopy(meta)
                if field=='registry':changed['packages'][1]['source']='alternate'
                else:changed['workspace_members']=['other']
                with self.assertRaises(ValueError):m.prepare.validate_dependencies(host,out,changed,lock)
    def test_preparer_requires_outer_group_for_subprocesses(self):
        with patch.dict(os.environ,{},clear=True),patch.object(m.prepare.subprocess,'check_output') as mocked:
            with self.assertRaises(ValueError):m.prepare.git(Path('/official'),'rev-parse','HEAD')
            mocked.assert_not_called()
    def test_no_nested_process_session_escape(self):
        source=(HERE/'prepare.py').read_text();tree=ast.parse(source)
        calls=[n for n in ast.walk(tree) if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute) and n.func.attr=='check_output']
        self.assertEqual(len(calls),2)
        self.assertTrue(all(not any(k.arg=='start_new_session' and ast.literal_eval(k.value) for k in c.keywords) for c in calls))
    def test_no_apply_plan_or_host_build_in_controller(self):
        source=(HERE/'run.py').read_text()
        self.assertNotIn("'apply'",source);self.assertNotIn("'plan'",source);self.assertNotIn("'horizon-mixins'",source)
        self.assertIn("'--locked','--offline'",source);self.assertIn("'--no-run'",source)
    def test_compile_precedes_sealed_execution_and_final_reverification(self):
        source=(HERE/'run.py').read_text();a=source.index("runner.run('helper-compile'");b=source.index('generated=base.out_dirs');c=source.index('base.test_after_compile');d=source.index("runner.run('verify-after'")
        self.assertLess(a,b);self.assertLess(b,c);self.assertLess(c,d)
        self.assertIn('base.assert_seal(host,host_seal)',source);self.assertIn('base.regular_seal(fixture)!=fixture_seal',source)
    def test_verified_alone_is_not_final_success(self):
        source=(HERE/'run.py').read_text()
        self.assertIn("if set(report['gates'])!=REQUIRED",source)
        self.assertIn("report['passed']=report['error'] is None and report['verified']",source)
        self.assertIn("if not report['passed']:raise SystemExit(1)",source)

if __name__=='__main__':unittest.main()
