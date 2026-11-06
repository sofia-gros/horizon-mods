"""Synthetic parser negatives only. These strings are NOT compiler/test evidence."""
import ast
import hashlib
import importlib.util
import json
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('ci_contract', HERE/'contract.py')
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
NAMES = sorted(set(m.TESTS.values()) | {'unrelated::native_test'})


def result(mode):
    return f'test {m.TESTS[mode]} ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; {len(NAMES)-1} filtered out; finished in 0.1s\n'


def replay():
    return {'baseline_capacity_bytes': 1, 'adapter_capacity_bytes': 2,
        'native_replay_geometry_equal': True, 'valid_claim_resumed': False,
        'allocation_capacity_equality_required': False}


def profile(opt=0):
    rows = []
    def add(marker, value): rows.append(marker+' '+json.dumps(value))
    for family, variants in [('geometry', ['geometry_build', 'geometry_clone', 'geometry_check']),
                             ('material', ['material_clone', 'material_check'])]:
        for right in variants:
            for smooth in (False, True):
                for pack in (False, True):
                    add('PLUTO_PROFILE_EQUAL', {'family': family, 'right': 'pluto_profile:'+right,
                        'smooth': smooth, 'pack': pack, 'replay_sections': 5})
    for _ in range(3): add('PLUTO_TERRAIN_REPLAY', replay())
    add('PLUTO_PROFILE_CLOCK', {'min_observed_empty_interval_ns': 1,
        'guaranteed_clock_resolution': False, 'debug_assertions': True})
    for name in sorted(m.PAIR_NAMES):
        add('PLUTO_PROFILE_PAIR', {'case': name, 'profile_label': m.label(opt), 'warmup': 8,
            'iterations_per_sample': 64, 'samples': 7, 'left_total_ns': [2_000_000]*7,
            'right_total_ns': [3_000_000]*7, 'right_over_left_median': 1.5,
            'sample_order': 'left-right on even rounds; right-left on odd rounds',
            'timing_inconclusive_short_samples': False, 'same_output_asserted': True,
            'scope': 'constant fixture callbacks, adapter and native emission remain together'})
    for name in sorted(m.MICRO_NAMES):
        add('PLUTO_PROFILE_MICRO', {'case': name, 'profile_label': m.label(opt), 'warmup': 128,
            'iterations_per_sample': 4096, 'samples': 7, 'total_ns': [2_000_000]*7,
            'sample_order': 'rotating five-kernel order by round',
            'timing_inconclusive_short_samples': False,
            'scope': 'actual pure function only; not subtracted from mesher time'})
    rows.append('PLUTO_PROFILE_COMPLETE same-output callbacks and actual pure kernels; no GPU or shipping performance claim')
    return result('profile')+'\n'.join(rows)+'\n'


def legacy(mode='registered', opt=0):
    rows = []
    def add(marker, value): rows.append(marker+' '+json.dumps(value))
    for i in range(4 if mode == 'no-provider' else 51):
        row = replay()
        if mode == 'registered' and i == 18:
            row.update(valid_claim_resumed=True, native_replay_geometry_equal=False)
        add('PLUTO_TERRAIN_REPLAY', row)
    if mode == 'registered':
        for name in ['no-claim', 'claimed', 'invalid-cell', 'post-stale']:
            rows.append('PLUTO_TERRAIN_CASE '+name+' passed 4 lighting combinations')
        for smooth in (False, True):
            for pack in (False, True):
                for kind, group, variant, model, stale in [
                    ('ordinary','material',0,'flow',False), ('ordinary','material',2,None,False),
                    ('ordinary','geometry',1,'flow',False), ('ordinary','material',4,'flow',False),
                    ('ordinary','material',2,'flow',True), ('ordinary','material',2,'missing',False),
                    ('incompatible','material',2,'flow',False), ('opaque','material',2,'flow',False)]:
                    add('PLUTO_TERRAIN_FALLBACK', dict(kind=kind, group='pluto_fixture:'+group,
                        variant=variant, model=None if model is None else 'pluto_fixture:block/'+model,
                        stale=stale, smooth=smooth, pack=pack))
        rows.append('PLUTO_TERRAIN_CASE materials passed: layers, mixed faces, tint, Top, UV, fallback replay, Fusion full-layout smoke')
    for name in (['no-provider'] if mode == 'no-provider' else [
                 'registered-ordinary','provider-no-claim','claimed-mixed','secondary-material']):
        add('PLUTO_TERRAIN_CPU', dict(case=name, profile_label=m.label(opt), debug_assertions=True,
            iterations_per_sample=64, warmup=8, samples=7,
            baseline_ns_per_section=[2_000_000]*7, adapter_ns_per_section=[3_000_000]*7,
            median_ratio=1.5, scope='CPU mesher, original synthetic assets; no GPU or production performance threshold'))
    rows.append('PLUTO_TERRAIN_MODE '+mode+' complete; actual native CPU mesh, original synthetic resources only')
    return result(mode)+'\n'.join(rows)+'\n'


def change(log, marker, key, value):
    out = []
    changed = False
    for line in log.splitlines():
        if not changed and line.startswith(marker+' '):
            row = json.loads(line[len(marker)+1:]); row[key] = value
            line = marker+' '+json.dumps(row); changed = True
        out.append(line)
    assert changed
    return '\n'.join(out)+'\n'


def args(opt=0):
    return ['rustc', '--crate-name', 'horizon_client_render', 'src/lib.rs', '--test',
        '-C', 'opt-level='+str(opt), '-C', 'debug-assertions=on', '-C', 'overflow-checks=on',
        '-C', 'codegen-units=16', '-C', 'debuginfo=0']


def artifact(opt=0):
    return {'reason': 'compiler-artifact', 'fresh': False,
        'target': {'name': 'horizon_client_render', 'kind': ['lib'], 'src_path': '/stage/crates/horizon-client-render/src/lib.rs'},
        'profile': {'test': True, 'opt_level': str(opt), 'debug_assertions': True, 'overflow_checks': True, 'debuginfo': 0},
        'executable': '/target/debug/deps/horizon_client_render-example'}


class Contracts(unittest.TestCase):
    def test_frozen_source_inputs_and_missing_admission_are_explicit(self):
        c = json.loads((HERE/'inputs.json').read_text())
        self.assertEqual((c['horizon'], c['mixins']), (m.HORIZON, m.MIXINS))
        self.assertEqual(set(c['admission']), {'latest_production_bundle_sha256', 'latest_reviewed_guard_cutoff', 'accepted_complete_plan_sha256'})
        self.assertTrue(all(v is None for v in c['admission'].values()))
        self.assertFalse(any(c['execution'].values()))
        for base, field in [(HERE.parents[2], 'source_sha256'), (HERE, 'contract_sha256')]:
            for p, digest in c[field].items():
                self.assertEqual(hashlib.sha256((base/p).read_bytes()).hexdigest(), digest, p)

    def test_eight_processes_keep_two_compiler_profiles_separate(self):
        s = m.schedule(); self.assertEqual(len(s), 8)
        self.assertEqual([(r['opt_level'], r['mode']) for r in s],
                         [(p,k) for p in (0,3) for k in ('no-provider','registered','profile','reload')])
        self.assertEqual(len({r['target_subdirectory'] for r in s}), 2)
        self.assertTrue(all(r['exact'] and r['test_threads'] == 1 for r in s))
        for value in (True, 1, 2, '3'):
            with self.assertRaises(ValueError): m.label(value)

    def test_exact_terse_inventory_and_fixture_subset(self):
        good = ''.join(n+': test\n' for n in NAMES)
        self.assertEqual(m.validate_inventory(good), NAMES)
        for bad in ['', good+'\n', good+'4 tests, 0 benchmarks\n', good+NAMES[0]+': test\n',
                    good.replace(NAMES[0], 'foreign::test', 1), good.replace(': test', ': benchmark', 1),
                    good.replace(': test\n', ': test\r\n'), ''.join(n+': test\n' for n in reversed(NAMES))]:
            with self.assertRaises(ValueError): m.validate_inventory(bad)

    def test_each_test_result_requires_exact_name_and_dynamic_filtered_count(self):
        for mode in m.TESTS:
            good = result(mode); m.validate_result(good, mode, NAMES)
            for bad in [good+good, good.replace(' ... ok', ' ... ignored'), good.replace('1 passed', '0 passed'),
                        good.replace('3 filtered', '2 filtered'), good.replace(m.TESTS[mode], 'foreign')]:
                with self.assertRaises(ValueError): m.validate_result(bad, mode, NAMES)

    def test_profile_has_all_twenty_three_five_and_replay_receipts(self):
        for opt in (0,3):
            r = m.validate_profile(profile(opt), opt, NAMES)
            self.assertEqual((len(r['equal_rows']),len(r['pairs']),len(r['micro']),r['replay_rows']), (20,3,5,3))

    def test_both_original_modes_require_all_replays_cases_and_timings(self):
        for opt in (0,3):
            for mode, n, timings, fallbacks in [('no-provider',4,1,0),('registered',51,4,32)]:
                r = m.validate_legacy(legacy(mode,opt),mode,opt,NAMES)
                self.assertEqual((r['replay_rows'],len(r['timings']),r['material_fallback_rows']), (n,timings,fallbacks))

    def test_original_replay_exact_count_position_and_semantics(self):
        good = legacy(); lines = good.splitlines(); first = next(s for s in lines if s.startswith('PLUTO_TERRAIN_REPLAY'))
        resumed = next(s for s in lines if s.startswith('PLUTO_TERRAIN_REPLAY') and '"valid_claim_resumed": true' in s)
        for bad in [good.replace(first+'\n','',1),good+first+'\n',
                    good.replace(resumed,first),good.replace(first,resumed,1),
                    good.replace(resumed+'\n','',1)+resumed+'\n',
                    change(good,'PLUTO_TERRAIN_REPLAY','allocation_capacity_equality_required',True)]:
            with self.assertRaises(ValueError): m.validate_legacy(bad,'registered',0,NAMES)

    def test_material_fallback_input_tuples_are_exact_not_just_counted(self):
        good=legacy(); row=next(s for s in good.splitlines() if s.startswith('PLUTO_TERRAIN_FALLBACK'))
        for bad in [good.replace(row+'\n','',1),good+row+'\n',
                    change(good,'PLUTO_TERRAIN_FALLBACK','model','pluto_fixture:block/unreviewed'),
                    change(good,'PLUTO_TERRAIN_FALLBACK','group','pluto_fixture:cell'),
                    change(good,'PLUTO_TERRAIN_FALLBACK','variant',False),
                    change(good,'PLUTO_TERRAIN_FALLBACK','smooth',0)]:
            with self.assertRaises(ValueError): m.validate_legacy(bad,'registered',0,NAMES)

    def test_original_timing_schema_samples_and_ratio_remain_strict(self):
        for key,value in [('median_ratio',True),('median_ratio',1.6),('samples',True),
                          ('warmup',9),('profile_label','old-unadmitted-label'),
                          ('baseline_ns_per_section',[0]*7),('adapter_ns_per_section',[True]*7),
                          ('adapter_ns_per_section',[2**64]*7),('debug_assertions',False)]:
            with self.assertRaises(ValueError): m.validate_legacy(change(legacy(),'PLUTO_TERRAIN_CPU',key,value),'registered',0,NAMES)
        good=legacy(); row=next(s for s in good.splitlines() if s.startswith('PLUTO_TERRAIN_CPU'))
        for bad in [good.replace(row+'\n','',1),good+row+'\n',good.replace('"median_ratio": 1.5','"median_ratio": NaN',1)]:
            with self.assertRaises(ValueError): m.validate_legacy(bad,'registered',0,NAMES)

    def test_original_markers_reject_missing_malformed_and_cross_mode_receipts(self):
        good=legacy(); case='PLUTO_TERRAIN_CASE claimed passed 4 lighting combinations\n'
        for bad in [good.replace(case,''),good+case,good+'PLUTO_TERRAIN_CASE\n',
                    good+'PLUTO_TERRAIN_CPU \n',good+'PLUTO_PROFILE_FOREIGN {}\n',
                    good.replace('PLUTO_TERRAIN_MODE registered','PLUTO_TERRAIN_MODE no-provider'),
                    good.replace('PLUTO_TERRAIN_MODE','INCOMPLETE')]:
            with self.assertRaises(ValueError): m.validate_legacy(bad,'registered',0,NAMES)
        with self.assertRaises(ValueError): m.validate_legacy(legacy('no-provider')+case,'no-provider',0,NAMES)

    def test_original_group_names_and_replay_structure_are_source_pinned(self):
        source=(HERE.parent/'terrain-material-cpu/fixture.rs').read_text()
        self.assertIn('const GROUP: &str = "pluto_fixture:geometry";',source)
        self.assertIn('const MATERIAL_GROUP: &str = "pluto_fixture:material";',source)
        self.assertIn('for kind in ["ordinary", "empty", "single"]',source)
        self.assertIn('let claimed_replay = resume_valid_claim && kind == "single";',source)
        self.assertIn('for smooth in [false, true]',source)
        self.assertIn('for pack in [false, true]',source)

    def test_missing_duplicate_or_typed_equality_rows_reject(self):
        good = profile(); first = next(x for x in good.splitlines() if x.startswith('PLUTO_PROFILE_EQUAL'))
        for bad in [good.replace(first+'\n','',1),good+first+'\n',
                    change(good,'PLUTO_PROFILE_EQUAL','smooth',0), change(good,'PLUTO_PROFILE_EQUAL','right','foreign')]:
            with self.assertRaises(ValueError): m.validate_profile(bad,0,NAMES)

    def test_ratio_recalculated_not_trusted(self):
        for ratio in (0.5, True, '1.5', None):
            with self.assertRaises(ValueError): m.validate_profile(change(profile(),'PLUTO_PROFILE_PAIR','right_over_left_median',ratio),0,NAMES)
        with self.assertRaises(ValueError): m.validate_profile(profile().replace('1.5','NaN',1),0,NAMES)

    def test_profile_ratio_matches_native_u64_cast_before_division(self):
        left,right=2**53+1,2**53+2
        good=change(profile(),'PLUTO_PROFILE_PAIR','left_total_ns',[left]*7)
        good=change(good,'PLUTO_PROFILE_PAIR','right_total_ns',[right]*7)
        native=float(right)/float(left)
        self.assertNotEqual(native,right/left)
        good=change(good,'PLUTO_PROFILE_PAIR','right_over_left_median',native)
        m.validate_profile(good,0,NAMES)
        with self.assertRaises(ValueError):
            m.validate_profile(change(good,'PLUTO_PROFILE_PAIR','right_over_left_median',right/left),0,NAMES)

    def test_short_samples_must_be_marked_inconclusive(self):
        bad = change(profile(),'PLUTO_PROFILE_MICRO','total_ns',[0]*7)
        with self.assertRaises(ValueError): m.validate_profile(bad,0,NAMES)
        fixed = change(bad,'PLUTO_PROFILE_MICRO','timing_inconclusive_short_samples',True)
        m.validate_profile(fixed,0,NAMES)

    def test_samples_cannot_be_negative_boolean_oversized_or_wrong_count(self):
        for value in [[True]*7,[-1]*7,[1]*6,[2**64]*7,'seven']:
            with self.assertRaises(ValueError): m.validate_profile(change(profile(),'PLUTO_PROFILE_MICRO','total_ns',value),0,NAMES)

    def test_wrong_scope_clock_label_or_completion_rejects(self):
        for bad in [change(profile(),'PLUTO_PROFILE_PAIR','same_output_asserted',False),
                    change(profile(),'PLUTO_PROFILE_MICRO','scope','isolated emission cost'),
                    change(profile(),'PLUTO_PROFILE_CLOCK','guaranteed_clock_resolution',True),
                    profile().replace('opt0-assertions','opt3-assertions'), profile().replace('PLUTO_PROFILE_COMPLETE','INCOMPLETE'),
                    profile()+'PLUTO_PROFILE_FOREIGN {}\n']:
            with self.assertRaises(ValueError): m.validate_profile(bad,0,NAMES)

    def test_replay_semantics_not_just_present(self):
        for key,value in [('valid_claim_resumed',True),('native_replay_geometry_equal',False),
                          ('allocation_capacity_equality_required',True),('adapter_capacity_bytes',-1)]:
            with self.assertRaises(ValueError): m.validate_profile(change(profile(),'PLUTO_TERRAIN_REPLAY',key,value),0,NAMES)

    def test_reload_exact_six_ordered_cases_only(self):
        good = result('reload')+''.join('RELOAD_CASE '+n+' ok\n' for n in m.RELOAD)
        self.assertEqual(m.validate_reload(good,NAMES)['cases'], m.RELOAD)
        for bad in [good+good,good.replace(m.RELOAD[0],'foreign'),good.replace(' ok\n',' ignored\n',1),good.replace('RELOAD_CASE '+m.RELOAD[0]+' ok\n','')]:
            with self.assertRaises(ValueError): m.validate_reload(bad,NAMES)

    def test_json_duplicate_keys_nonfinite_and_budget_reject(self):
        for value in ['{"x":1,"x":2}','{"x":NaN}','{"x":Infinity}']:
            with self.assertRaises(ValueError): m.strict_json(value)
        with self.assertRaises(ValueError): m.records('x'*(m.MAX_LOG_BYTES+1),'X')

    def test_malformed_empty_structured_extra_receipts_reject(self):
        for suffix in ['PLUTO_PROFILE_EQUAL\n','PLUTO_PROFILE_MICRO \n']:
            with self.assertRaises(ValueError): m.validate_profile(profile()+suffix,0,NAMES)
        good = result('reload')+''.join('RELOAD_CASE '+n+' ok\n' for n in m.RELOAD)
        with self.assertRaises(ValueError): m.validate_reload(good+'RELOAD_CASE malformed\n',NAMES)

    def test_actual_argv_shape_and_artifact_profiles_pair(self):
        for opt in (0,3):
            r = m.validate_compiler(args(opt),'/stage/crates/horizon-client-render','/stage',opt)
            self.assertFalse(r['observed_execution'])
            self.assertEqual(r['opt_level'],opt)
            self.assertEqual(m.validate_artifact(artifact(opt),'/stage','/target',opt),artifact(opt)['executable'])

    def test_no_fresh_other_source_or_different_profile_artifact(self):
        for bad in [dict(artifact(),fresh=True),dict(artifact(),executable='/foreign/binary')]:
            with self.assertRaises(ValueError): m.validate_artifact(bad,'/stage','/target',0)
        for field,value in [('opt_level','3'),('debug_assertions',False),('overflow_checks',False),('debuginfo',2),('test',False)]:
            bad=artifact();bad['profile'][field]=value
            with self.assertRaises(ValueError): m.validate_artifact(bad,'/stage','/target',0)

    def test_compiler_effective_defaults_duplicates_and_cost_flags(self):
        plain = ['rustc','--crate-name','horizon_client_render','src/lib.rs','--test','-C','codegen-units=16']
        m.validate_compiler(plain,'/stage/crates/horizon-client-render','/stage',0)
        for bad in [plain+['-C','opt-level=3'],args()+['-C','incremental=/target/cache'],
                    args()+['-C','opt-level=0'],args()+['-C','lto=thin'],[]]:
            with self.assertRaises((ValueError,IndexError)): m.validate_compiler(bad,'/stage/crates/horizon-client-render','/stage',0)
        with self.assertRaises(ValueError): m.validate_compiler(args(),'/foreign','/stage',0)

    def test_this_module_cannot_fetch_compile_apply_or_update_guards(self):
        tree=ast.parse((HERE/'contract.py').read_text())
        imports={n.module for n in ast.walk(tree) if isinstance(n,ast.ImportFrom)}
        imports|={a.name for n in ast.walk(tree) if isinstance(n,ast.Import) for a in n.names}
        self.assertFalse(imports & {'os','subprocess','shutil','requests'})
        calls={n.func.attr for n in ast.walk(tree) if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute)}
        self.assertFalse(calls & {'write_text','write_bytes','mkdir','unlink','open','read_bytes','read_text'})


if __name__ == '__main__': unittest.main()
