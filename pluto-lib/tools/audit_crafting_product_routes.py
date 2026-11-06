#!/usr/bin/env python3
"""Read-only source audit of the pinned official automatic crafting routes.

This proves source shape/order only. It does not install hooks or execute any
Menu, Game, network, output publication, or consumption operation.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

PIN = '624c4f6265c02296fbe8a80f029dd0d987fb94c0'
ORIGIN = 'https://github.com/horizon-rust/horizon.git'
PATHS = {
    'menu': 'crates/horizon-client/src/menu.rs',
    'bundle': 'crates/horizon-client/src/menu/bundle.rs',
    'game': 'crates/horizon-client/src/game.rs',
    'lan': 'crates/horizon-client/src/game/lan_crafting.rs',
    'crafter': 'crates/horizon-client/src/game/machines/crafter.rs',
    'crafting': 'crates/horizon-client/src/crafting.rs',
    'remote': 'crates/horizon-client/src/game/remote.rs',
}
def digest(value):
    return hashlib.sha256(value).hexdigest()

def audit(host):
    host = host.resolve()
    def git(*args):
        return subprocess.check_output(['git', '-C', str(host), *args], text=True).strip()
    if git('remote', 'get-url', 'origin') != ORIGIN or git('rev-parse', 'HEAD') != PIN or git('status', '--porcelain', '--untracked-files=all'):
        raise ValueError('Require exact clean official checkout; no fork or host edits')
    sources = {key: (host / path).read_text() for key, path in PATHS.items()}
    fragments = {}
    checks = []
    def body(source, name):
        matches = list(re.finditer(r'^    (?:pub(?:\([^)]*\))? )?fn ' + re.escape(name) + r'(?:<[^\n]+>)?\(', sources[source], re.M))
        if len(matches) != 1:
            raise ValueError('Ambiguous function: ' + name)
        start = matches[0].start()
        tail = sources[source][matches[0].end():]
        next_fn = re.search(r'^    (?:pub(?:\([^)]*\))? )?fn ', tail, re.M)
        end = matches[0].end() + next_fn.start() if next_fn else len(sources[source])
        text = sources[source][start:end]
        fragments[source + '::' + name] = {'file': PATHS[source], 'line': sources[source][:start].count('\n')+1, 'sha256': digest(text.encode())}
        return text
    def ordered(label, text, *markers):
        positions = [text.index(marker) for marker in markers]
        if positions != sorted(positions) or len(set(positions)) != len(positions):
            raise ValueError('Ordering changed: ' + label)
        checks.append(label)
    ops = body('game', 'menu_ops')
    ordered('local non-QuickMove result bypasses Menu', ops, 'if g == 200 && input != Input::QuickMove', 'return None;', 'menu.click(env, menu_slot, button, input)')
    local = body('game', 'click_inventory')
    ordered('local authority excludes remote before result branch', local, 'if self.remote.is_some()', 'return self.remote_click(right);', 'if slot == 200 {')
    result = local[local.index('if slot == 200 {'):]
    ordered('local result/remainder derive before output and consumption', result, 'while let Some((recipe, result)) = self.craft_recipe()', 'let remainders = world_recipes!(self).remainders(', 'self.give(result)', 'for cell in self.craft_grid.iter_mut()', 'for (i, rest) in remainders.into_iter().enumerate()', 'self.native_item_crafted(')
    session = body('game', 'menu_session')
    ordered('local menu changes commit only after successful callback', session, 'let outcome = run(&mut menu, &env, &to_menu);', 'Some(false) => return true', 'Some(true) => {}', 'let predicting = self.remote.is_some();', 'let after: Vec<Option<ItemStack>>', 'self.carried = carried;')
    click = body('menu', 'click')
    ordered('QuickMove recomputes/retries within one click', click, 'let mut clicked = self.quick_move(env, s);', 'while let Some(c) = &clicked', '                    clicked = self.quick_move(env, s);')
    ordered('safe_take has already removed output before on_take', body('menu', 'safe_take'), 'self.try_remove(env, s, amount, max_amount)?', 'self.on_take(env, s, &result)')
    ordered('pickup has already published cursor before on_take', body('menu', 'pickup_inner'), 'self.try_remove(env, s, amount, u32::MAX)', 'self.carried = Some(taken.clone());', 'self.on_take(env, s, &taken)')
    ordered('quick_move has already moved items before on_take', body('menu', 'quick_move'), 'self.move_to(env, &mut stack', 'self.on_take(env, index, &clicked)')
    ordered('swap has already published player slot before on_take', body('menu', 'swap'), 'self.containers[PLAYER][inv_slot] = Some(taken.clone());', 'self.set_by_player(env, target, None);', 'self.on_take(env, target, &taken);')
    take = body('menu', 'on_take')
    tail = take[take.index('let grid = self.containers[GRID].clone();'):]
    if tail.count('.find(&grid, n)') != 2:
        raise ValueError('Menu find count changed')
    ordered('on_take recalculates matches/remainders before decrement', tail, '.find(&grid, n)', '.remainders(id, &grid, n)', 'stack.count -= 1;', 'self.recompute(env);')
    if 'self.safe_take(env, s, other.count, room)' not in body('bundle', 'carried_bundle_click'):
        raise ValueError('Bundle extraction route changed')
    checks.append('carried bundle extraction also uses safe_take')
    pickup_all = body('menu', 'can_take_for_pick_all')
    if 'self.slots[t].container != RESULT' not in pickup_all or 'Kind::Loom | Kind::Grindstone | Kind::Anvil' not in pickup_all:
        raise ValueError('Crafting pickup-all exclusion changed')
    checks.append('crafting result excluded from pickup-all')
    recompute = body('menu', 'recompute')
    ordered('menu result generation applies limited crafting before storing', recompute, '.find(&self.containers[GRID], self.grid_size)', 'if env.limited_crafting', 'let is_special', 'let known', 'self.containers[RESULT][0] = result;')
    lan = body('lan', 'guest_menu_op')
    ordered('LAN guest authority copies input and output around same Menu op', lan, 'load_player(menu,', 'let result = op(menu, &env);', 'store_player(menu,', 'self.drop_guest_items(host, gi, dropped);')
    inv = body('lan', 'lan_inventory_click')
    ordered('LAN inventory prepare hook can replace result after op', inv, 'self.guest_menu_op(', 'menu.click(env,', 'self.native_prepare_item_craft(', 'menu.set_slot(&env, 0, result);')
    table = body('lan', 'lan_crafting_click')
    ordered('LAN table validates authority then can replace result after op', table, '!host.hub.guests[gi].spectator()', 'self.lan_table_valid(', 'self.guest_menu_op(', 'menu.click(env,', 'self.native_prepare_item_craft(', 'menu.set_slot(&env, 0, result);')
    remove = body('menu', 'remove')
    ordered('ResultContainer removes the whole result, not a partial split', remove, 'if slot.container == RESULT && slot.role != Role::LoomResult', 'return self.containers[RESULT][0].take();', '.map(|c| split(c, amount))')
    crafter = body('crafter', 'crafter_dispense')
    ordered('crafter postprocesses output before deriving remainder then publishes before decrement', crafter, 'let grid: Vec<Option<ItemStack>>', '.find_with(&grid, 3,', 'Menu::map_crafted(', '.remainders(&recipe, &grid, 3)', 'self.crafter_eject(key, facing, result', 'for rest in remainders.into_iter().flatten()', 's.count -= 1;', 'self.native_crafter_craft(')
    remote = body('remote', 'remote_menu_ops')
    ordered('remote predicts on client copy then sends clicks, not authority', remote, 'self.menu_session(', 'predict_clicks(m, env, &clicks)', 'self.send_clicks(container, sent);')
    send = body('remote', 'send_clicks')
    ordered('remote sends changed-slot hashes and server state', send, 'let state = self.remote.as_ref()', 'let carried = hashed(self, &carried);', 'self.send(S::ContainerClick(')
    callsites = []
    for path in sorted((host/'crates/horizon-client/src').rglob('*.rs')):
        if path.name.endswith('_tests.rs'):
            continue
        for number,line in enumerate(path.read_text().splitlines(),1):
            if re.search(r'\.remainders\s*\(',line):
                callsites.append({'file':str(path.relative_to(host)), 'line':number})
    if {row['file'] for row in callsites} != {PATHS['game'],PATHS['menu'],PATHS['crafter']} or len(callsites)!=3:
        raise ValueError('Syntactic production remainder call-site inventory changed')
    checks.append('three syntactic production remainder calls belong to audited Game/Menu/Crafter bodies')
    new = body('menu', 'new')
    if new.count('\n        Self {') != 1 or '.with_initial_data()' not in new:
        raise ValueError('Menu constructor shape changed')
    checks.append('Menu has one selected Self literal, planner feasibility still separate')
    return {'official_horizon': PIN, 'source_sha256': {PATHS[k]: digest(v.encode()) for k,v in sources.items()}, 'fragments': fragments, 'source_assertions': checks, 'source_assertion_count': len(checks), 'production_remainder_call_sites': callsites, 'runtime_validation': False, 'automatic_hooks_installed': False}

if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--horizon-source', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    result = audit(args.horizon_source)
    args.output.write_text(json.dumps(result, indent=2)+'\n')
    print(f"Passed {result['source_assertion_count']} pinned official source-order assertions; no runtime or hook claim.")
