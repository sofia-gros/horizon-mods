#!/usr/bin/env python3
"""Prepare source-only legacy inspection fixture; no apply, build or save mode."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
REVISION = '624c4f6265c02296fbe8a80f029dd0d987fb94c0'
SOURCES = {
    'crates/horizon-client-render/src/effects.rs': '5269c3c35f38c4c658d4409b0f7badae4f0ddca0b68df36c5197a5dd51f735dd',
    'crates/horizon-world/src/nbt.rs': '6cbce272aa37242d50a7f0baa9ce6fd76f694e4d7cb8aac8c84951e4f778500c',
    'crates/horizon-client-core/src/nbt_value.rs': 'c289ce9eba52dea6d0f2682b2620a4d955b2a75dde08f8e34437c7e81dacce82',
    'crates/horizon-client/src/save.rs': '5d4cda583f246f987494f90490b10b39fb0641ce9377ae607d6cd75353bf559b',
    'crates/horizon-client/src/lan/playerdata.rs': '6f798f8564e49570d8b2719f3e3e07a39e6e082c9a74cb5a0c262306b2fd9886',
    'crates/horizon-client/src/game.rs': 'cdfc9247cce139c00e5a36266812c7f7f42ccbb565d67705a896520b4b62b22c',
    'crates/horizon-client/src/game/lan_actions.rs': 'a2ea07508ddc14b34fa638e0d11af31d86b9e18cd2e8a7d286e33616c4327a57',
    'crates/horizon-client/src/motion.rs': '32caf71a62f87dda437f650e271922a9e5add6dec7056a67cf0d82471b56a4ae',
    'crates/horizon-client/src/game/potions.rs': '9d504ba0534f79a5a040d85e40649b14943847cb9691a2621c9dfe37ef45f460',
}
START = '        self.active_effects = '
END = '\n        // ServerPlayer.saveParentVehicle:'

def digest(data): return hashlib.sha256(data).hexdigest()

def read_sources(official):
    official = official.resolve()
    if subprocess.check_output(['git', '-C', str(official), 'rev-parse', 'HEAD'], text=True).strip() != REVISION:
        raise ValueError('wrong approved official revision')
    if subprocess.check_output(['git', '-C', str(official), 'status', '--porcelain', '--untracked-files=no'], text=True):
        raise ValueError('official tracked sources must be clean')
    result = {}
    for name, expected in SOURCES.items():
        data = (official / name).read_bytes()
        if digest(data) != expected: raise ValueError('official source changed: ' + name)
        result[name] = data
    return result

def oracle(save):
    save = save.split('    pub fn read_player(&mut self, p: &Tag) {', 1)[1].split('    pub fn player_tag(&self) -> Tag {', 1)[0]
    if save.count(START) != 1 or save.count(END) != 1:
        raise ValueError('native tuple span is not unique')
    body = save.split(START, 1)[1].split(END, 1)[0]
    if not body.endswith(';'): raise ValueError('native assignment terminator changed')
    expression = body[:-1]
    return ('use horizon_client_core::nbt_value::{number, text};\nuse horizon_world::nbt::Tag;\n'
            'pub fn decode(p: &Tag) -> Vec<(String, i32, i32, bool, bool)> {\n' + expression + '\n}\n')

def generated(sources):
    return {
        'official-nbt.rs': sources['crates/horizon-world/src/nbt.rs'],
        'official-nbt-value.rs': sources['crates/horizon-client-core/src/nbt_value.rs'],
        'legacy_oracle.rs': oracle(sources['crates/horizon-client/src/save.rs'].decode()).encode(),
        'legacy_view.rs': (HERE / 'legacy_view.rs').read_bytes(),
        'legacy_tests.rs': (HERE / 'legacy_tests.rs').read_bytes(),
        'fixture.rs': b'''extern crate self as horizon_world;
extern crate self as horizon_client_core;
#[path = "official-nbt.rs"] pub mod nbt;
#[path = "official-nbt-value.rs"] pub mod nbt_value;
mod legacy_view;
mod legacy_oracle;
#[cfg(test)] mod legacy_tests;
''',
    }

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--official', type=Path, required=True)
    p.add_argument('--output', type=Path)
    args = p.parse_args()
    sources = read_sources(args.official)
    files = generated(sources)
    if args.output:
        output = args.output.resolve()
        if not output.is_relative_to((REPO / '.work').resolve()): raise ValueError('output outside MOD .work')
        if output.exists(): raise ValueError('output must not exist')
        output.mkdir(parents=True)
        for name, data in files.items(): (output / name).write_bytes(data)
        evidence = {
            'status': 'UNAPPLIED borrowed legacy inspection, not native migration or contact activation',
            'official_revision': REVISION, 'source_sha256': SOURCES,
            'prepared_files': {name: digest(data) for name, data in files.items()},
            'oracle': 'Exact Level::read_player active_effects RHS; only enclosing assignment and semicolon removed, wrapped in standalone function',
            'native_types': 'Complete unchanged official NBT and nbt_value modules',
            'excluded': ['Full Level or Game execution', 'Registry resolution or effect merge', 'Local/LAN file migration', 'Actor swap or contact authority', 'Native NBT own tests unless explicitly selected'],
        }
        (output / 'evidence.json').write_text(json.dumps(evidence, indent=2) + '\n')
    if read_sources(args.official) != sources: raise ValueError('official source changed during preparation')
    print('Exact legacy expression and official dependencies verified; no compilation or official writes.')

if __name__ == '__main__': main()
