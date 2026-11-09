"""Execute the actual contact bridge with a small registry dependency fixture."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class BlockContactTests(unittest.TestCase):
    @unittest.skipUnless(shutil.which('rustc'), 'Rust toolchain is required')
    def test_registry_refresh_properties_unknown_ids_and_unlocked_dispatch(self):
        source = (ROOT / 'mod/pluto-block.rs').as_posix()
        fixture = r'''extern crate self as horizon_world;
pub mod registry {
    pub struct BlockStateId(pub u32);
    pub struct State { pub name: String, pub properties: std::collections::BTreeMap<String, String> }
    pub struct BlockRegistry(pub State);
    impl BlockRegistry {
        pub fn state(&self, id: BlockStateId) -> Option<&State> { (id.0 == 0).then_some(&self.0) }
    }
}
#[allow(dead_code)]
#[path = "SOURCE"]
mod bridge;
use std::sync::{LazyLock, Mutex};
fn registry(name: &str, facing: &str) -> registry::BlockRegistry {
    registry::BlockRegistry(registry::State {
        name: name.into(), properties: [("facing".into(), facing.into())].into(),
    })
}
static FIRST: LazyLock<registry::BlockRegistry> = LazyLock::new(|| registry("example:first", "west"));
static SECOND: LazyLock<registry::BlockRegistry> = LazyLock::new(|| registry("example:second", "east"));
static SEEN: Mutex<Vec<(String, bridge::StateInfo, [i32; 3], i32, String)>> = Mutex::new(Vec::new());
fn sink() -> bridge::Sink {
    bridge::Sink {
        has: |_| true, survives: |_, _, _| None,
        placement: |_, _| bridge::PlacementAnswer::Default, update_shape: |_, _| None,
        entity_inside: |dimension, state, pos, entity, kind| {
            // A callback may re-enter setup: neither bridge mutex may be held here.
            bridge::set_contact_registry(&SECOND);
            bridge::set_sink(sink());
            SEEN.lock().unwrap().push((dimension.into(), state.clone(), pos, entity, kind.into()));
        },
        random_tick: |_, _, _| false, scheduled_tick: |_, _, _| false,
        bonemeal_valid: |_, _, _| None, bonemeal_success: |_, _, _| None,
        bonemeal_perform: |_, _, _| {}, may_place_on: |_, _, _, _| None,
    }
}
fn contact(id: u32) {
    bridge::entity_inside("minecraft:the_nether", 41, "minecraft:zombie", registry::BlockStateId(id), [-2, 65, 7]);
}
#[test]
fn registry_is_current_and_callbacks_are_unlocked() {
    bridge::set_sink(sink());
    contact(0);
    assert!(SEEN.lock().unwrap().is_empty(), "no registry: do not guess");
    bridge::set_contact_registry(&FIRST);
    contact(999);
    assert!(SEEN.lock().unwrap().is_empty(), "unknown id: do not guess");
    contact(0);
    contact(0);
    let seen = SEEN.lock().unwrap();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0], ("minecraft:the_nether".into(), bridge::StateInfo {
        block: "example:first".into(), properties: [("facing".into(), "west".into())].into(),
    }, [-2, 65, 7], 41, "minecraft:zombie".into()));
    assert_eq!(seen[1].1.block, "example:second");
    assert_eq!(seen[1].1.properties["facing"], "east");
}
'''.replace('SOURCE', source)
        with tempfile.TemporaryDirectory(prefix='pluto-contact-') as temp:
            temp = Path(temp)
            test = temp / 'test.rs'
            binary = temp / ('test.exe' if os.name == 'nt' else 'test')
            test.write_text(fixture, encoding='utf-8', newline='\n')
            subprocess.run(['rustc', '--edition=2024', '--test', str(test), '-o', str(binary)], check=True, timeout=60)
            subprocess.run([str(binary)], check=True, timeout=30)


if __name__ == '__main__':
    unittest.main()
