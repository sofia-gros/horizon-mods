"""Prepare PlutoLib-owned Horizon source overrides and the matching full bundle.

The output is an overlay, not a compiled game: copy horizon-overlay onto an
isolated pinned Horizon build tree before applying the emitted bundle.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import tomllib

from direct_host_rules import RULES, apply_rule

ROOT = Path(__file__).resolve().parents[2]
# SHA-256 of Git blob bytes at the manifest's pinned revision (LF).
ITEMS_SOURCE = 'crates/horizon-items/src/items.rs'
SOURCE_HASHES = {
    ITEMS_SOURCE: 'd9c283ac47a8ed7689e0b9ae72d7d20f872c3d8bd0d7515b47d12d8120d7c7aa',
    'crates/horizon-client/src/game/native_block_entity_draw.rs': 'f2ba152dde79b3ee0610c179eee9c7dfa88974bfda492a9721fa78b1791a66e7',
    'crates/horizon-client/src/game/combat.rs': '2d7a2a593644ea96f61878f9c1ab1d3eb1c4e61961ce4c31d3822c90d95f4ae0',
}
# Contract of each declaration transferred out of the Mixins projection. The
# direct rule supplies its call arguments itself, so changes to bindings,
# selectors, ownership or constructor counts must never be silently discarded.
DECLARATION_HASHES = {
    'pluto-menu-actions': 'cd23fcf0ee3ce87136dc5bf65c652ad9749a60576fb8d44a9d0926b5e5b31bfe',
    'pluto-menu-actions-local': 'd3b459212bf6c3739a790c8b98cd21d177c153183b0edfa5d9f5e500ed4f998c',
    'pluto-menu-actions-guest': 'e2740c640a147743d0928abba53e137396133014c4c15a6aa701edcc7cf7bac0',
    'pluto-item-prototypes': '1779bc4e5081db24d6399d4b3aa33ec4cd6827e9b0ee91d3164de2f550f1d8d1',
    'pluto-item-after-fall': '7aff6853a19eec5c537ec66645b1ac3513e788ef16bf9fa9294429337513c027',
    'pluto-mob-after-fall': '9563f8a83d542bb2ec055a472c7af82e64eb780ff72941c49bebb00bc600355a',
    'pluto-block-inside-motion': '509835fabf0486db2d6f12d54c36e80e3c5c6fb2ad9c0d7b155d9626ad31145d',
    'pluto-entity-spawn-spawner-add': 'e29c99b5793ec084c345842d9d2a4ddd84799c7c4dfb02b5b9232f8a1707e458',
    'pluto-living-attributes-reload': '38155a91220e3ca973a24557836bf789ee85e620b702c1b83840a970a26ebd18',
    'pluto-skull-special-decode': 'c5b954e90c69d3d501c8b21388bda3d7d6a7344e3b3dda0e92b1a2dba9463f23',
    'pluto-item-placement-root-path': '035d99a20cc0d46d713ba538ac0c6edc9f007f2438ce0064bfa363df02e113b3',
    'pluto-entity-particle-host-event': 'd0f3e61a927bf45ae54474ac6d8b2e5ec17035d7f6e711753a481c0b1de835cb',
    'pluto-armor-worn-layers': 'a2a6fecfd54161df86b15481e0e373d4ea9c8948a24943e361dd8968188309b8',
    'pluto-armor-stand-worn-layers': 'b05d7e67e819cea384b368d61af4ba336f9aba941a19d20b39338c6e4944818f',
    'pluto-block-entity-cache-generation': 'f8e21e6c9424b94864b9bbb22d15332eb4f4070157617cc0031c664f2007d06f',
    'pluto-living-drop-state': 'e8204433c51ef8d9a66931bc851816aacedbf7db00153a2fcf10de7ebf824551',
    'pluto-block-entity-container-reach': 'fe4212c7ab75aec4802abb750e29e4a81ad61a37a1a9145e247fd6d2333ebd2c',
    'pluto-item-menu-open': 'cfdc76a6d58b054f17969aa409dc9c50cc7bb313e87670f91b7f0832f4cdf894',
}


def validate_transferred_declarations(parsed):
    if set(DECLARATION_HASHES) != set(STATES) | {rule['id'] for rule in RULES} | {'pluto-item-prototypes'}:
        raise ValueError('direct attachment lacks a full declaration contract')
    for site, expected in DECLARATION_HASHES.items():
        entries = [entry for kind in ('injection', 'state')
                   for entry in parsed.get(kind, []) if entry.get('id') == site]
        if len(entries) != 1:
            raise ValueError(f'{site}: expected exactly one transferred declaration')
        encoded = json.dumps(entries[0], sort_keys=True, separators=(',', ':')).encode('utf-8')
        if hashlib.sha256(encoded).hexdigest() != expected:
            raise ValueError(f'{site}: full declaration changed; review direct attachment')


# Files the build tree does not have: helper fragments for the generated `Host` (source in this directory, never an official file).
ADDED_FILES = {
    'tools/native-hook-fragments/pluto_container_reach.json': 'pluto-container-reach-fragment.json',
    'tools/native-hook-fragments/pluto_item_menu_open.json': 'pluto-item-menu-open-fragment.json',
}


STATES = {
    'pluto-block-entity-cache-generation': (
        'crates/horizon-client/src/game/native_block_entity_draw.rs',
        'KeptEntry', 'u64',
        'crate::game::pluto_block_entity_cache::frame_generation()',
    ),
    'pluto-living-drop-state': (
        'crates/horizon-client/src/game/combat.rs',
        'CombatTracker', 'crate::game::combat::pluto_living_drop_state::HeadDeaths',
        '::core::default::Default::default()',
    ),
}


def transform(text, site, owner, ty, initial):
    """Keep the existing helper ABI while supplying its storage directly."""
    name = '__horizon_mixins_pluto_lib_' + site.replace('-', '_')
    declaration = f'struct {owner} {{'
    if text.count(declaration) != 1:
        raise ValueError(f'{site}: expected exactly one owner declaration')
    text = text.replace(declaration, declaration + f'\n    {name}: {ty},', 1)
    if owner == 'KeptEntry':
        literal = '            KeptEntry {\n'
        if text.count(literal) != 1:
            raise ValueError(f'{site}: expected one native constructor')
        text = text.replace(literal, literal + f'                {name}: {initial},\n', 1)
    # CombatTracker uses derived Default and a struct-update literal; both
    # naturally initialize the added field without rewriting closure bodies.
    text += (f'\nimpl {owner} {{\n'
             f'    pub(crate) fn {name}(&self) -> &{ty} {{\n'
             f'        &self.{name}\n    }}\n}}\n')
    return text


def attach_item_prototypes(text, helper):
    # Use the exact Arc still held by load_with, at the final owned map transfer.
    # This preserves all fallible loads and their original order.
    anchor = '        Ok(Self {\n            items,\n            mob_visibility,'
    if text.count(anchor) != 1:
        raise ValueError('item prototypes: expected unique final items transfer')
    text = text.replace(anchor, '        Ok(Self {\n            items: pluto_item_prototypes(items, &raw),\n            mob_visibility,', 1)
    return text + '\n' + helper


def prepare(source, output):
    source, output = source.resolve(), output.resolve()
    if output.exists():
        raise ValueError('output must be new; existing build inputs are never overwritten')
    if output.is_relative_to(source) or source.is_relative_to(output):
        raise ValueError('output must be separate from reference sources')
    bundle = ROOT / 'pluto-lib/mod'
    manifest = (bundle / 'sunrise-mod.toml').read_text(encoding='utf-8')
    parsed = tomllib.loads(manifest)
    if parsed['compatibility']['horizon_revision'] != '441f49d57e344bd56b90fd4af5c4e443ff3937c9':
        raise ValueError('Horizon revision changed; review direct attachments')
    validate_transferred_declarations(parsed)
    overrides, receipts = {}, []
    for site, (relative, owner, ty, initial) in STATES.items():
        declarations = [s for s in parsed['state'] if s['id'] == site]
        if len(declarations) != 1 or declarations[0]['ty'] != ty or declarations[0]['initial'] != initial:
            raise ValueError(f'{site}: declaration changed; review direct attachment')
        data = (source / relative).read_bytes().replace(b'\r\n', b'\n')
        digest = hashlib.sha256(data).hexdigest()
        if digest != SOURCE_HASHES[relative]:
            raise ValueError(f'{relative}: official source fingerprint mismatch')
        changed = transform(data.decode('utf-8'), site, owner, ty, initial).encode('utf-8')
        overrides[relative] = changed
        receipts.append({'id': site, 'file': relative, 'before': digest,
                         'after': hashlib.sha256(changed).hexdigest()})
        pattern = r'(?m)^\[\[state\]\]\n' + r'id = "' + re.escape(site) + r'"\n.*?(?=^\[\[(?!state\.constructor)|\Z)'
        manifest, count = re.subn(pattern, '', manifest, flags=re.S)
        if count != 1:
            raise ValueError(f'{site}: cannot transfer state ownership')
    site = 'pluto-item-prototypes'
    declarations = [s for s in parsed['injection'] if s['id'] == site]
    if len(declarations) != 1 or declarations[0] != {
        'id': site, 'target': 'sunrise_items::items::Items::load_with',
        'kind': 'MODIFY_VARIABLE', 'helper': 'pluto-item-prototypes.rs',
        'variable': {'name': 'items', 'ordinal': 0, 'at': 'LOAD'},
        'require': 3, 'ordinal': 2,
        'bindings': [
            {'name': 'value', 'ty': 'HashMap<Arc<str>, ItemInfo>', 'value': '$value'},
            {'name': 'raw', 'ty': '&Arc<sunrise_world::item_defaults::Defaults>', 'value': '$local_ref:raw:0'},
        ],
    }:
        raise ValueError('item prototypes: declaration changed; review attachment')
    data = (source / ITEMS_SOURCE).read_bytes().replace(b'\r\n', b'\n')
    digest = hashlib.sha256(data).hexdigest()
    if digest != SOURCE_HASHES[ITEMS_SOURCE]:
        raise ValueError(f'{ITEMS_SOURCE}: official source fingerprint mismatch')
    helper = (bundle / 'pluto-item-prototypes.rs').read_text(encoding='utf-8')
    changed = attach_item_prototypes(data.decode('utf-8'), helper).encode('utf-8')
    overrides[ITEMS_SOURCE] = changed
    receipts.append({'id': site, 'file': ITEMS_SOURCE, 'before': digest,
                     'after': hashlib.sha256(changed).hexdigest(),
                     'helper_sha256': hashlib.sha256(helper.encode('utf-8')).hexdigest()})
    manifest, count = re.subn(r'(?m)^\[\[injection\]\]\nid = "pluto-item-prototypes"\n.*?(?=^\[\[|\Z)', '', manifest, flags=re.S)
    if count != 1:
        raise ValueError('item prototypes: cannot transfer attachment ownership')
    for rule in RULES:
        site, relative = rule['id'], rule['file']
        declarations = [s for s in parsed['injection'] if s['id'] == site]
        if len(declarations) != 1 or any(declarations[0].get(k) != rule[k] for k in ('target', 'kind', 'helper')):
            raise ValueError(f'{site}: declaration changed; review direct attachment')
        data = (source / relative).read_bytes().replace(b'\r\n', b'\n')
        digest = hashlib.sha256(data).hexdigest()
        if digest != rule['sha256']:
            raise ValueError(f'{relative}: official source fingerprint mismatch')
        helper = (bundle / rule['helper']).read_text(encoding='utf-8')
        before = overrides.get(relative, data)
        changed = apply_rule(before.decode('utf-8'), rule, helper).encode('utf-8')
        overrides[relative] = changed
        receipts.append({'id': site, 'file': relative, 'before': hashlib.sha256(before).hexdigest(),
                         'after': hashlib.sha256(changed).hexdigest(),
                         'helper_sha256': hashlib.sha256(helper.encode('utf-8')).hexdigest()})
        pattern = r'(?m)^\[\[injection\]\]\nid = "' + re.escape(site) + r'"\n.*?(?=^\[\[|\Z)'
        manifest, count = re.subn(pattern, '', manifest, flags=re.S)
        if count != 1:
            raise ValueError(f'{site}: cannot transfer attachment ownership')
    added = {}
    for relative, name in ADDED_FILES.items():
        if (source / relative).exists():
            raise ValueError(f'{relative}: an added file must not replace an official file')
        data = (Path(__file__).resolve().parent / name).read_bytes().replace(b'\r\n', b'\n')
        overrides[relative] = data
        added[relative] = hashlib.sha256(data).hexdigest()
    generated = tomllib.loads(manifest)
    if any(s['id'] in STATES for s in generated.get('state', [])):
        raise ValueError('duplicate state ownership in generated bundle')
    transferred = {r['id'] for r in receipts} - set(STATES)
    for kind, removed in [('injection', transferred), ('state', set(STATES)), ('module', set()), ('bridge', set())]:
        expected = [entry for entry in parsed.get(kind, []) if entry['id'] not in removed]
        if generated.get(kind, []) != expected:
            raise ValueError(f'{kind}: generated bundle changed unrelated attachments')
    # Validate every input before creating output. Preserve all other sites.
    output.mkdir(parents=True)
    shutil.copytree(bundle, output / 'bundle')
    (output / 'bundle/sunrise-mod.toml').write_text(manifest, encoding='utf-8', newline='\n')
    for relative, data in overrides.items():
        path = output / 'horizon-overlay' / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    bundle_digest = hashlib.sha256()
    for path in sorted((output / 'bundle').rglob('*')):
        if path.is_file():
            bundle_digest.update(path.relative_to(output / 'bundle').as_posix().encode('utf-8') + b'\0')
            bundle_digest.update(hashlib.sha256(path.read_bytes()).digest())
    receipt = {'horizon_revision': parsed['compatibility']['horizon_revision'],
               'bundle_sha256': bundle_digest.hexdigest(),
               'attachments': receipts, 'added_files': added, 'native_verified': False}
    (output / 'direct-host.json').write_text(newline='\n', data=json.dumps(receipt, indent=2) + '\n', encoding='utf-8')
    return receipt


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(prepare(args.source, args.output), indent=2))
