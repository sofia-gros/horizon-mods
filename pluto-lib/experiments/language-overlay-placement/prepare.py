"""Source-only payload/plan inspection. Does not call Mixins, Cargo or apply."""
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import tomllib

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2] if len(HERE.parents) > 2 else HERE.parent
REPAIR = ROOT / 'pluto-lib/experiments/language-overlay-repair'
TARGET = 'crates/horizon-client/src/mod_content.rs'
MOD = 'pluto_language_repair_probe'
HORIZON = 'f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd'
MIXINS = '5a64af938ec86b00d53d16a408aabc0038fb3e57'


def sha(raw): return hashlib.sha256(raw).hexdigest()
def require(value, message):
    if not value: raise ValueError(message)


def native_write(native):
    start = b'    fn write(&self, rel: &str, bytes: &[u8]) -> Result<()> {'
    require(native.count(start) == 1, 'native write signature changed')
    at = native.index(start)
    end = native.index(b'\n    }', at) + len(b'\n    }')
    return native[at:end]


def expected_head(native):
    method = native_write(native)
    body = method[method.index(b'{') + 1:method.rindex(b'}')]
    require(body.count(b'fs::write(path, bytes)?;') == 1, 'native writer call changed')
    body = body.replace(b'self.', b'overlay.').replace(b'fs::write(path, bytes)?;', b'fs::write(path, &merged)?;')
    prefix = b'''fn write_if_eligible(overlay: &Overlay<'_>, rel: &str, bytes: &[u8]) -> std::ops::ControlFlow<Result<()>> {
    let std::borrow::Cow::Owned(merged) = pluto_language_overlay::prepare(overlay.base, rel, bytes) else {
        return std::ops::ControlFlow::Continue(());
    };
    let result = (|| -> Result<()> {'''
    suffix = b'''})();
    std::ops::ControlFlow::Break(result)
}
'''
    result = prefix + body + suffix
    # Exact small copied body only; no recursive Overlay::write or broad overwrite.
    restored = result[len(prefix):-len(suffix)].replace(b'overlay.', b'self.').replace(b'fs::write(path, &merged)?;', b'fs::write(path, bytes)?;')
    require(restored == method[method.index(b'{') + 1:method.rindex(b'}')], 'native writer inverse changed')
    return result


def payload(selected):
    config = json.loads((HERE/'inputs.json').read_bytes())
    require((config['horizon'], config['mixins']) == (HORIZON, MIXINS), 'wrong official inputs')
    spec = importlib.util.spec_from_file_location('excluded_merge_contract', REPAIR/'source.py')
    shared = importlib.util.module_from_spec(spec); spec.loader.exec_module(shared)
    shared.compose(selected)  # Includes exact admission and writer source checks.
    native = (Path(selected)/TARGET).read_bytes()
    helper = (REPAIR/'merge.rs').read_bytes()
    require(sha(helper) == config['merge_sha256'], 'merge helper changed')
    policy = re.findall(rb'pub\(super\) const POLICY_ID: &str = "([^"]+)";', helper)
    require(policy == [config['policy_id'].encode()], 'policy identity changed without source contract')
    files = {name:(HERE/name).read_bytes() for name in ['horizon-mixin.toml','write-head.rs','stamp-store.rs']}
    require({p:sha(b) for p,b in files.items()} == config['payload_sha256'], 'payload changed')
    require(files['write-head.rs'] == expected_head(native), 'small writer copy differs from native')
    require(sha(native_write(native)) == config['native_write_sha256'], 'native write hash changed')
    manifest = tomllib.loads(files['horizon-mixin.toml'].decode())
    require(set(manifest) == {'mod','compatibility','module','injection'} and
            manifest['mod'] == {'id':MOD,'version':'0.0.0','priority':100} and
            manifest['compatibility'] == {'horizon_revision':HORIZON}, 'probe identity changed')
    require(len(manifest['module']) == 1 and len(manifest['injection']) == 2, 'scope changed')
    files['merge.rs'] = helper
    return files


def inspect_plan(plan, files, native_inputs):
    # This checks the tiny probe only. It is NOT an all-installed composition guard.
    require(plan.get('schema') == 6, 'unexpected plan schema')
    require(all(not plan.get(k) for k in ['bridges','states','crates','data','post_build']), 'unexpected additional plan entries')
    for path, digest in native_inputs.items():
        require(plan.get('inputs',{}).get(path) == digest, 'official plan input mismatch: '+path)
    manifest = tomllib.loads(files['horizon-mixin.toml'].decode())
    modules, injections = plan.get('modules', []), plan.get('injections', [])
    require(len(modules) == 1 and len(injections) == 2, 'expected one module and two injections')
    module = modules[0]
    require((module.get('mod_id'),module.get('mod_version'),module.get('module_id'),module.get('file'),module.get('name')) ==
            (MOD,'0.0.0','bounded-merge',TARGET,'pluto_language_overlay'), 'wrong module owner')
    require(module.get('visibility', 'pub(crate)') == 'pub(crate)' and module.get('source','').encode() == files['merge.rs'], 'module visibility or payload drift')
    observed = {p.get('injection_id'):p for p in injections}
    require(len(observed) == 2, 'duplicate injection id')
    witnesses = []
    for expected in manifest['injection']:
        row = observed.get(expected['id'], {})
        for key in ['target','kind','bindings']:
            require(row.get(key) == expected[key], 'wrong planned '+key)
        require((row.get('mod_id'),row.get('mod_version'),row.get('priority'),row.get('file')) == (MOD,'0.0.0',100,TARGET), 'wrong injection owner')
        require(row.get('helper_source','').encode() == files[expected['helper']], 'helper payload mismatch')
        require(row.get('cancellable') == expected.get('cancellable',False), 'wrong cancellation contract')
        require(row.get('node') and re.fullmatch('[0-9a-f]{64}',row.get('function_fingerprint','')), 'missing actual function witness')
        contract = row.get('contract', {})
        require(all(contract.get(k) is None for k in ['operation','bundle','result']), 'unexpected operation contract')
        output = re.sub(r'\s+', '', contract.get('output','')).removeprefix('::')
        if expected['kind'] == 'HEAD':
            require(row.get('ordinal') is None and row.get('require') is None and row.get('variable') is None, 'unexpected HEAD site')
            require(output in ['std::ops::ControlFlow<Result<()>>', 'core::ops::ControlFlow<Result<()>>'] and
                    contract.get('value') is None, 'wrong HEAD result contract')
        else:
            require(row.get('ordinal') == 0 and row.get('require') == 1 and row.get('variable',{}).get('at') == 'STORE', 'wrong variable site')
            local = row.get('variable',{}).get('local',{})
            require((local.get('name'),local.get('ordinal'),local.get('parameter'),local.get('mutable'),local.get('supported')) == ('parts',0,False,True,True), 'wrong actual parts binding')
            require(isinstance(row.get('variable_site'),int) and row['variable_site'] == row.get('expression_site') and
                    re.fullmatch('[0-9a-f]{64}',row.get('site_fingerprint','')), 'missing actual STORE witness')
            require(output == 'Vec<String>' and re.sub(r'\s+', '', contract.get('value','')) == 'Vec<String>', 'wrong STORE value contract')
        witnesses.append({k:v for k,v in row.items() if k != 'helper_source'})
    return {'status':'tiny_probe_plan_accepted_only','schema':6,'transformer':plan.get('transformer'),
            'injections':witnesses,'module':{k:v for k,v in module.items() if k != 'source'},
            'helper_sha256':{p:sha(b) for p,b in files.items()},'native_apply':False,
            'native_type_check':False,'all_installed_composition':False,'game_ui':False}
