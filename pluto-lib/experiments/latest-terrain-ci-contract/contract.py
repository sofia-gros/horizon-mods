"""Excluded pure CI result contracts. No launcher, source writes or acceptance grant."""
import hashlib
import json
from pathlib import PurePosixPath
import re

HORIZON = 'f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd'
MIXINS = '5a64af938ec86b00d53d16a408aabc0038fb3e57'
PREFIX = 'mesher::pluto_terrain_material_cpu::tests::'
TESTS = {
    'no-provider': PREFIX + 'real_native_mesh_differential_and_timing',
    'registered': PREFIX + 'real_native_mesh_differential_and_timing',
    'profile': PREFIX + 'profile::same_output_callbacks_and_microkernels',
    'reload': PREFIX + 'reload::exact_resource_owner_reload_contract',
}
RELOAD = [
    'same_assets_new_render_tables_generation_collision',
    'exact_triple_and_clones_only',
    'unavailable_candidate_does_not_replace_old_owners',
    'abandoned_candidate_does_not_keep_resource_values_alive',
    'stale_registry_requires_new_owners',
    'same_owner_conflict_retains_original_snapshot',
]
PAIR_NAMES = {'geometry_original_vs_build', 'geometry_build_vs_clone', 'material_build_vs_clone'}
MICRO_NAMES = {'geometry_build', 'batch_clone', 'cell_local', 'uv_subset_accept', 'uv_subset_reject'}
MAX_LOG_BYTES = 2 * 1024 * 1024
U64_MAX = 2**64 - 1


def require(ok, message):
    if not ok:
        raise ValueError(message)


def strict_json(raw):
    def pairs(values):
        out = {}
        for key, value in values:
            require(key not in out, 'duplicate JSON key')
            out[key] = value
        return out
    def nonfinite(value):
        raise ValueError('nonfinite JSON constant: ' + value)
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=nonfinite)


def bounded(log):
    require(isinstance(log, str) and len(log.encode()) <= MAX_LOG_BYTES, 'log budget')
    return log


def records(log, marker):
    bounded(log)
    result = []
    for line in log.splitlines():
        if line.partition(' ')[0] != marker:
            continue
        head, sep, body = line.partition(' ')
        require(sep and body.strip(), 'empty structured receipt')
        result.append(strict_json(body))
    return result


def unsigned(value, positive=False):
    return type(value) is int and (1 if positive else 0) <= value <= U64_MAX


def samples(value):
    require(isinstance(value, list) and len(value) == 7 and all(unsigned(x) for x in value), 'invalid samples')
    return value


def label(opt):
    require(type(opt) is int and opt in (0, 3), 'unsupported profile')
    return 'opt' + str(opt) + '-assertions'


def schedule():
    # Each row requires its OWN process. Neither registry can be reset safely.
    return [{'opt_level': opt, 'profile_label': label(opt), 'mode': mode,
             'test': TESTS[mode], 'exact': True, 'test_threads': 1,
             'target_subdirectory': 'host-target-opt' + str(opt)}
            for opt in (0, 3) for mode in ('no-provider', 'registered', 'profile', 'reload')]


def validate_inventory(log):
    # Explicit future command: binary --list --format terse (no summary).
    names = re.findall(r'^(\S+): test$', bounded(log), re.M)
    require(names and len(names) == len(set(names)), 'empty/duplicate native inventory')
    require(log == ''.join(n + ': test\n' for n in sorted(names)), 'unexpected inventory bytes/order')
    require({n for n in names if n.startswith(PREFIX)} == set(TESTS.values()), 'unexpected fixture inventory')
    return names


def validate_result(log, mode, names):
    require(mode in TESTS and TESTS[mode] in names and len(names) == len(set(names)), 'unadmitted test')
    rows = re.findall(r'^test (\S+) \.\.\. (\w+)\s*$', bounded(log), re.M)
    require(rows == [(TESTS[mode], 'ok')], 'missing, extra, ignored or failed selected test')
    summary = re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;', log, re.M)
    require(summary == [('1', '0', '0', '0', str(len(names)-1))], 'result/filtered count mismatch')


def validate_replays(rows, count, resumes=0):
    require(len(rows) == count, 'replay count')
    for row in rows:
        require(set(row) == {'baseline_capacity_bytes', 'adapter_capacity_bytes',
            'native_replay_geometry_equal', 'valid_claim_resumed', 'allocation_capacity_equality_required'}, 'replay schema')
        require(unsigned(row['baseline_capacity_bytes']) and unsigned(row['adapter_capacity_bytes']), 'capacity observation')
        require(type(row['valid_claim_resumed']) is bool and
                row['native_replay_geometry_equal'] is (not row['valid_claim_resumed']) and
                row['allocation_capacity_equality_required'] is False, 'replay semantics')
    require(sum(r['valid_claim_resumed'] for r in rows) == resumes, 'resumed-claim count')


def validate_profile(log, opt, inventory):
    validate_result(log, 'profile', inventory)
    equal = records(log, 'PLUTO_PROFILE_EQUAL')
    expected = {(family, 'pluto_profile:' + right, smooth, pack)
        for family, variants in [('geometry', ['geometry_build', 'geometry_clone', 'geometry_check']),
                                 ('material', ['material_clone', 'material_check'])]
        for right in variants for smooth in (False, True) for pack in (False, True)}
    require(len(equal) == 20, 'same-output row count')
    observed = []
    for row in equal:
        require(set(row) == {'family', 'right', 'smooth', 'pack', 'replay_sections'} and
                type(row['smooth']) is bool and type(row['pack']) is bool and
                type(row['replay_sections']) is int and row['replay_sections'] == 5, 'same-output schema')
        observed.append((row['family'], row['right'], row['smooth'], row['pack']))
    require(set(observed) == expected, 'duplicate/missing same-output tuple')
    clocks = records(log, 'PLUTO_PROFILE_CLOCK')
    require(len(clocks) == 1, 'clock receipt count')
    clock = clocks[0]
    require(set(clock) == {'min_observed_empty_interval_ns', 'guaranteed_clock_resolution', 'debug_assertions'} and
            unsigned(clock['min_observed_empty_interval_ns'], True) and
            clock['guaranteed_clock_resolution'] is False and clock['debug_assertions'] is True, 'clock schema')
    floor = min(U64_MAX, clock['min_observed_empty_interval_ns'] * 100)
    pairs = records(log, 'PLUTO_PROFILE_PAIR')
    micro = records(log, 'PLUTO_PROFILE_MICRO')
    require(len(pairs) == 3 and {r.get('case') for r in pairs} == PAIR_NAMES, 'paired case set')
    require(len(micro) == 5 and {r.get('case') for r in micro} == MICRO_NAMES, 'micro case set')
    for row in pairs + micro:
        require(row.get('profile_label') == label(opt) and type(row.get('samples')) is int and row['samples'] == 7,
                'profile label/sample count')
    for row in pairs:
        require(set(row) == {'case', 'profile_label', 'warmup', 'iterations_per_sample', 'samples',
            'left_total_ns', 'right_total_ns', 'right_over_left_median', 'sample_order',
            'timing_inconclusive_short_samples', 'same_output_asserted', 'scope'}, 'paired schema')
        require(row['warmup'] == 8 and row['iterations_per_sample'] == 64 and
                row['same_output_asserted'] is True and row['sample_order'] ==
                'left-right on even rounds; right-left on odd rounds' and row['scope'] ==
                'constant fixture callbacks, adapter and native emission remain together', 'paired scope')
        left, right = samples(row['left_total_ns']), samples(row['right_total_ns'])
        median = sorted(left)[3]
        ratio = None if median == 0 else sorted(right)[3] / median
        require(row['right_over_left_median'] == ratio and
                (ratio is None or type(row['right_over_left_median']) in (int, float)), 'paired median')
        require(row['timing_inconclusive_short_samples'] is any(x < 1_000_000 or x < floor for x in left+right),
                'paired inconclusive flag')
    for row in micro:
        require(set(row) == {'case', 'profile_label', 'warmup', 'iterations_per_sample', 'samples',
            'total_ns', 'sample_order', 'timing_inconclusive_short_samples', 'scope'}, 'micro schema')
        require(row['warmup'] == 128 and row['iterations_per_sample'] == 4096 and
                row['sample_order'] == 'rotating five-kernel order by round' and
                row['scope'] == 'actual pure function only; not subtracted from mesher time', 'micro scope')
        times = samples(row['total_ns'])
        require(row['timing_inconclusive_short_samples'] is any(x < 1_000_000 or x < floor for x in times), 'micro inconclusive flag')
    validate_replays(records(log, 'PLUTO_TERRAIN_REPLAY'), 3)
    require(re.findall(r'^PLUTO_PROFILE_COMPLETE (.+)$', log, re.M) ==
            ['same-output callbacks and actual pure kernels; no GPU or shipping performance claim'], 'profile completion')
    markers = re.findall(r'^(PLUTO_PROFILE_\S+)', log, re.M)
    require(set(markers) == {'PLUTO_PROFILE_EQUAL', 'PLUTO_PROFILE_CLOCK', 'PLUTO_PROFILE_PAIR',
                            'PLUTO_PROFILE_MICRO', 'PLUTO_PROFILE_COMPLETE'}, 'unknown profile marker')
    return {'equal_rows': equal, 'clock': clock, 'pairs': pairs, 'micro': micro, 'replay_rows': 3,
            'scope': 'constant synthetic callbacks and actual kernels; no mesh speedup inference'}


def validate_reload(log, inventory):
    validate_result(log, 'reload', inventory)
    rows = re.findall(r'^RELOAD_CASE (\S+) (\S+)$', log, re.M)
    require(rows == [(name, 'ok') for name in RELOAD], 'missing/duplicate/reordered reload case')
    require([line for line in log.splitlines() if line.startswith('RELOAD_CASE')] ==
            ['RELOAD_CASE '+name+' ok' for name in RELOAD], 'malformed extra reload receipt')
    return {'cases': RELOAD, 'scope': 'synthetic native owner values, not Game reload/streaming execution'}


def validate_compiler(args, cwd, stage, opt):
    # Invocation provenance and the executable bytes belong to the outer sealed
    # runner. This pure check must not treat a supplied argv as execution proof.
    label(opt)
    require(PurePosixPath(cwd) == PurePosixPath(stage)/'crates/horizon-client-render', 'wrong compile cwd')
    require(args.count('--crate-name') == 1 and args[args.index('--crate-name')+1] == 'horizon_client_render', 'wrong crate')
    require(args.count('--test') == 1, 'not a renderer test compile')
    sources = [s for s in args if s.endswith('.rs')]
    expected = str(PurePosixPath(stage) / 'crates/horizon-client-render/src/lib.rs')
    require(len(sources) == 1 and str(PurePosixPath(cwd) / sources[0]) == expected, 'wrong renderer source')
    codegen = {}
    at = 0
    while at < len(args):
        arg = args[at]
        if arg == '-C':
            at += 1; require(at < len(args), 'truncated codegen option'); item = args[at]
        elif arg.startswith('-C'):
            item = arg[2:]
        else:
            at += 1; continue
        key, sep, value = item.partition('=')
        require(sep and key not in codegen, 'duplicate/unsupported codegen option')
        codegen[key] = value; at += 1
    actual_opt = codegen.get('opt-level', '0')
    debug = codegen.get('debug-assertions', 'on' if actual_opt == '0' else 'off')
    overflow = codegen.get('overflow-checks', debug)
    require(actual_opt == str(opt) and debug == overflow == 'on', 'effective compiler profile')
    require(codegen.get('debuginfo', '0') == '0' and codegen.get('codegen-units') == '16' and
            codegen.get('lto', 'off') == 'off' and 'incremental' not in codegen, 'unexpected compiler costs')
    return {'opt_level': opt, 'debug_assertions': True, 'overflow_checks': True,
            'debug': 0, 'codegen_units': 16, 'lto': False, 'incremental': False,
            'argv_sha256': hashlib.sha256(json.dumps(args, separators=(',', ':')).encode()).hexdigest(),
            'source': expected, 'observed_execution': False}


def validate_artifact(value, stage, target, opt):
    label(opt)
    require(value.get('reason') == 'compiler-artifact' and value.get('fresh') is False, 'missing actual artifact')
    t, p = value.get('target', {}), value.get('profile', {})
    require(t.get('name') == 'horizon_client_render' and t.get('kind') == ['lib'] and
            t.get('src_path') == str(PurePosixPath(stage)/'crates/horizon-client-render/src/lib.rs'), 'artifact source')
    require(p.get('test') is True and p.get('opt_level') == str(opt) and p.get('debug_assertions') is True and
            p.get('overflow_checks') is True and type(p.get('debuginfo')) is int and p['debuginfo'] == 0, 'artifact profile')
    executable = PurePosixPath(value.get('executable') or '')
    require(executable.is_absolute() and '..' not in executable.parts and
            executable.parent == PurePosixPath(target)/'debug/deps', 'artifact destination')
    return str(executable)  # Outer runner must verify regular executable/hash.
