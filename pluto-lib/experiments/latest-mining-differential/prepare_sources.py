"""Pure source renderer for a later real-native fixture, not a Cargo runner."""
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
MOD_ROOT = 'tconstruct/mixins/tconstruct/'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def single(source, fragment):
    if source.count(fragment) != 1:
        raise ValueError('Expected one exact source fragment: ' + fragment[:70])
    return fragment


def render(inputs, regression):
    """Return new Rust bytes only. Caller must independently verify official paths/lock."""
    spec = json.loads((HERE / 'inputs.json').read_text())
    if set(inputs) != set(spec['source_sha256']):
        raise ValueError('Input file-set drift')
    for path, expected in spec['source_sha256'].items():
        if not isinstance(inputs[path], bytes) or sha(inputs[path]) != expected:
            raise ValueError('Input byte drift: ' + path)
    if sha(regression) != spec['regression_sha256']:
        raise ValueError('Regression source drift')
    hand = inputs['official/crates/horizon-client/src/game/hand.rs'].decode()
    game = inputs['official/crates/horizon-client/src/game.rs'].decode()
    lan = inputs['official/crates/horizon-client/src/game/lan_actions.rs'].decode()
    division = single(hand, 'speed / hardness / if harvest { 30.0 } else { 100.0 }')
    tick = single(game, 'm.progress >= 1.0 && (self.lan_acting.is_none() || m.ticks == 1)')
    # The native operations around these predicates are NOT reproduced.
    single(lan, 'Some(p) if p < 1.0 => {}')
    single(lan, 'if done.is_some() {\n                    self.guest_destroy(host, conn, pos);')
    pure = inputs[MOD_ROOT + 'source-pure-rules.rs'].decode()
    start = pure.index('pub(super) fn boost(')
    end = pure.index('\n#[cfg(test)]', start)
    dwarven = pure[start:end].rstrip() + '\n'
    if dwarven.count('pub(super) fn boost(') != 1:
        raise ValueError('Dwarven fragment drift')
    fragments = f'''// Source-pinned arithmetic/predicate projections, not Game implementations.
pub(crate) fn progress(speed: f32, hardness: f32, harvest: bool) -> f32 {{
    {division}
}}
pub(crate) fn tick_threshold(progress: f32) -> bool {{
    // Only the numeric first predicate of: {tick}
    progress >= 1.0
}}
pub(crate) fn delayed_completion(progress: Option<f32>) -> bool {{
    // Project only the native match guard and done.is_some() branch decision.
    match progress {{ Some(p) if p < 1.0 => false, done => done.is_some() }}
}}
'''
    lib = '''#![allow(dead_code)]
pub use horizon_items::items::ItemStack;
#[path = "tool_rules.rs"] pub mod tool_rules;
mod tconstruct_tool_api {
    pub use horizon_items::item_data::canon_tag;
    pub use crate::tool_rules as rules;
}
#[path = "dwarven.rs"] mod tconstruct_source_dwarven;
#[path = "mining_math.rs"] mod tconstruct_source_mining_math;
#[path = "mining_rules.rs"] mod tconstruct_source_mining_rules;
mod official_fragments;
#[cfg(test)] mod regression_tests;
'''
    return {
        'lib.rs': lib.encode(),
        'tool_rules.rs': inputs['pluto-lib/mixins/pluto_tool_rules/rules.rs'],
        'dwarven.rs': dwarven.encode(),
        'mining_math.rs': inputs[MOD_ROOT + 'source-mining-math.rs'],
        'mining_rules.rs': inputs[MOD_ROOT + 'source-mining-rules.rs'],
        'official_fragments.rs': fragments.encode(),
        'regression_tests.rs': regression,
    }
