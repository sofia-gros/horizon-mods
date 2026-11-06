"""Produce exact finite policy fixture sources in memory; never run Rust or Git."""
import hashlib
import json
from pathlib import Path
import re
HERE = Path(__file__).resolve().parent
MAIN = '08afa992d5091e03360f3473ea93fd17aa793987'
CANDIDATE = '68bddc1b7543c4822bf3c15f6312d4d4d35b76d8'
HORIZON = '41f822b659789780f58d32c63ddfb6887ea1b8bf'
MIXINS = '5a64af938ec86b00d53d16a408aabc0038fb3e57'
READ = b'crate::game::tconstruct_source_melter_store::read(&game.world,(pos[0],pos[1],pos[2]),"tconstruct:modifier_chest")'
FIXTURE_READ = b'metadata.read("tconstruct:modifier_chest")'
ARG = b"game: &crate::game::Game<'_>"
FIXTURE_ARG = b"metadata: &super::FixtureMetadata<'_>"
DECL = b'fn tconstruct_chest_pick('
FIXTURE_DECL = b'pub(crate) fn apply('
READER_START = b'pub(super) fn read(world: &World, pos: Pos, key: &str) -> Option<Tag> {\n'
READER_END = b'\n}\n\npub(super) fn write('
RAW_START = b'    if let Some(value)=raw.get(key)'
LIMIT = 128 * 1024
sha = lambda b: hashlib.sha256(b).hexdigest()

def config():
    c = json.loads((HERE/'inputs.json').read_text())
    if (c['schema'], c['main'], c['candidate'], c['horizon'], c['mixins']) != (1, MAIN, CANDIDATE, HORIZON, MIXINS):
        raise ValueError('Wrong fixed fixture inputs')
    if any(c[k] for k in ['native_executed', 'game_type_verified', 'installed_hook_verified']):
        raise ValueError('Source fixture cannot claim execution')
    if set(c['input_sha256']) != {'old_helper', 'metadata_store', 'native_pick'}:
        raise ValueError('Input source set changed')
    return c

def bytes_input(value, expected):
    if not isinstance(value, bytes) or len(value) > LIMIT or sha(value) != expected:
        raise ValueError('Source bytes/hash mismatch')
    return value

def reader_slice(store):
    if store.count(READER_START) != 1 or store.count(READER_END) != 1:
        raise ValueError('Ambiguous actual metadata reader')
    start = store.index(READER_START) + len(READER_START)
    end = store.index(READER_END, start)
    body = store[start:end]
    if body.count(RAW_START) != 1 or not body.endswith(b'    None'):
        raise ValueError('Reader suffix changed')
    prefix, suffix = body.split(RAW_START, 1)
    expected = (b'    let chunk = world.chunks.get(&(pos.0 >> 4, pos.2 >> 4))?;\n'
                b'    let saved = chunk.saved.as_deref()?;\n'
                b'    let raw=&saved.block_entity_sources.iter().find(|(at, _)| *at == pos)?.1;\n')
    if prefix != expected:
        raise ValueError('Native world/source acquisition changed')
    suffix = RAW_START + suffix
    return b'fn read_payload(raw: &Tag, key: &str) -> Option<Tag> {\n' + suffix + b'\n}\n', suffix

def helper_slice(source):
    replacements = [(DECL, FIXTURE_DECL), (ARG, FIXTURE_ARG), (READ, FIXTURE_READ)]
    out = source
    for before, after in replacements:
        if out.count(before) != 1 or after in out:
            raise ValueError('Ambiguous fixed helper substitution')
        out = out.replace(before, after, 1)
    inverse = out
    for before, after in reversed(replacements):
        if inverse.count(after) != 1:
            raise ValueError('Fixture helper inverse is ambiguous')
        inverse = inverse.replace(after, before, 1)
    if inverse != source or b'crate::game' in out or b'&game.world' in out:
        raise ValueError('Fixture helper inverse or Game isolation failed')
    return b'use horizon_items::items::ItemStack;\n' + out

def render(old_helper, metadata_store, new_helper, native_pick, tests):
    c = config()
    for name, raw in [('old_helper', old_helper), ('metadata_store', metadata_store), ('native_pick', native_pick)]:
        bytes_input(raw, c['input_sha256'][name])
    bytes_input(new_helper, c['new_helper_sha256']); bytes_input(tests, c['regression_sha256'])
    # Independently enforce only the two runtime header/binding changes.
    expected = old_helper.replace(b'mut value: Option<ItemStack>', b'mut value: ItemStack', 1)
    expected = expected.replace(b'-> Option<ItemStack>', b'-> ItemStack', 1)
    expected = expected.replace(b'    let Some(stack)=value.as_mut() else {return value;};', b'    let stack=&mut value;', 1)
    if expected != new_helper:
        raise ValueError('Candidate differs beyond owned-result bridge')
    reader, suffix = reader_slice(metadata_store)
    prefix = b'''// Actual native types; only world metadata acquisition is supplied as input.
use std::cell::Cell;
use horizon_world::nbt::{Compound, Tag};
pub mod items { pub use horizon_items::items::ItemStack; }
struct FixtureMetadata<'a> { raw: Option<&'a Tag>, reads: Cell<usize> }
impl<'a> FixtureMetadata<'a> {
    fn new(raw: Option<&'a Tag>) -> Self { Self { raw, reads: Cell::new(0) } }
    fn read(&self, key: &str) -> Option<Tag> {
        self.reads.set(self.reads.get() + 1);
        read_payload(self.raw?, key)
    }
}
mod old_policy;
mod new_policy;
pub mod native_pick;
#[cfg(test)] mod regression_tests;
'''
    names = {
        'original3': sorted('native_pick::tests::'+n for n in re.findall(rb'#\[test\]\s*fn\s+(\w+)\(', native_pick.decode().encode()).__iter__())
    } if False else {
        'original3': sorted('native_pick::tests::'+n for n in re.findall(r'#\[test\]\s*fn\s+(\w+)\(', native_pick.decode())),
        'additional14': sorted('regression_tests::'+n for n in re.findall(r'#\[test\]\s*fn\s+(\w+)\(', tests.decode()))}
    if names != c['expected_tests'] or len(names['original3']) != 3 or len(names['additional14']) != 14 or len(set(sum(names.values(), []))) != 17:
        raise ValueError('Wrong full17 test inventory')
    result = {'lib.rs': prefix+reader, 'old_policy.rs': helper_slice(old_helper), 'new_policy.rs': helper_slice(new_helper),
              'native_pick.rs': native_pick, 'regression_tests.rs': tests}
    if any(len(b)>LIMIT or b'#[ignore]' in b for b in result.values()):
        raise ValueError('Oversized or skipped fixture source')
    return result, {'status':'source_composed_only', 'main':MAIN, 'candidate':CANDIDATE, 'horizon':HORIZON,
                    'files':{n:{'bytes':len(b),'sha256':sha(b)} for n,b in result.items()},
                    'reader_suffix_sha256':sha(suffix), 'test_names':names, 'native_executed':False,
                    'scope':'Finite policy/value tests, exact native pick_item only; no Game, world lookup, actor, hook IR or installation proof.'}

def frozen_inputs():
    c = config(); raw = (HERE/'oracles.json').read_bytes(); bytes_input(raw, c['oracles_sha256'])
    o = json.loads(raw)
    if set(o) != set(c['input_sha256']):raise ValueError('Frozen source oracle set changed')
    return {name:value.encode() for name,value in o.items()}

def compose():
    source = frozen_inputs()
    new = (HERE.parent/'latest-chest-pick/source-chest-pick.rs').read_bytes()
    tests = (HERE/'regression_tests.rs').read_bytes()
    return render(source['old_helper'],source['metadata_store'],new,source['native_pick'],tests)

if __name__ == '__main__':
    _, receipt = compose()
    print(json.dumps(receipt,sort_keys=True))
