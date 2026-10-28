"""Exercise Station input, preview, take and save/reload in an isolated game."""
import argparse
import json
from pathlib import Path
import tempfile
import time

from probe_horizon_item_state import states
from probe_horizon_server_mods import Game, ROOT


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--game', type=Path, default=ROOT.parent / 'horizon/target/fast/horizon-client')
    parser.add_argument('--ctl', type=Path, default=ROOT.parent / 'horizon/target/fast/horizon-ctl')
    parser.add_argument('--resources', type=Path, default=ROOT.parent / 'horizon/.local/horizon-client/26.3')
    parser.add_argument('--client-mod', type=Path, default=ROOT / "Tinkers' onstruct/target/horizon-mod/tconstruct.hmod")
    parser.add_argument('--server-mod', type=Path, default=ROOT / "Tinkers' onstruct/server-tools/target/horizon-mod/tconstruct_tools.hmod")
    parser.add_argument('--in-frame', action='store_true')
    args = parser.parse_args()
    for key in ('game', 'ctl', 'resources', 'client_mod', 'server_mod'):
        setattr(args, key, getattr(args, key).resolve())
    parent = ROOT / '.local/station-repair-tests'
    parent.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='station-', dir=parent))
    report = {'ok': False, 'checks': [], 'mode': 'in-frame' if args.in_frame else 'integrated'}
    game = None

    def wait(label, predicate):
        deadline = time.monotonic() + 25
        while time.monotonic() < deadline:
            if predicate():
                report['checks'].append(label)
                return
            time.sleep(.2)
        raise AssertionError(label)

    def menu():
        return {s['slot']: s['stack'] for s in game.call('menu')['slots'] if s.get('stack')}

    def slot_damage(slot):
        item = menu().get(slot)
        return item.get('damage') if item else None

    def inventory(damage, ingredient, count):
        inv = game.call('inventory')
        return [s['damage'] for s in states(inv)] == [damage] and inv['counts'].get(ingredient, 0) == count

    def click(slot, *args):
        game.call('click-slot', str(slot), *args)
        game.call('wait', '5')
        report.setdefault('clicks', []).append({'slot': slot, 'menu': game.call('menu')})

    def hotbar(item):
        return next(s['slot'] for s in game.call('inventory')['hotbar'] if s.get('item') == item)

    try:
        game = Game(args, run, 'Miner', in_frame=args.in_frame)
        game.ready()
        game.call('command', 'fill -2 70 -2 2 70 3 minecraft:stone')
        game.call('command', 'tp @s 0.5 71 0.5')
        game.call('command', 'setblock 0 71 2 tconstruct:tinker_station')
        wait('station block available', lambda: game.call('block', '0', '71', '2').get('block') == 'tconstruct:tinker_station')
        for material, damage, ingredient, before, after in [
            ('flint', 85, 'minecraft:flint', 3, 1),
            ('wood', 60, 'minecraft:oak_planks', 3, 1),
        ]:
            game.call('command', 'gamemode creative @s')
            game.call('command', 'clear @s')
            if material == 'flint':
                state = json.dumps({'schema': 1, 'tool': 'tconstruct:pickaxe', 'materials': ['tconstruct:flint', 'tconstruct:wood', 'tconstruct:wood'], 'damage': damage, 'broken': True}, separators=(',', ':'))
                game.call('command', f'''give @s minecraft:wooden_pickaxe[custom_data={{tconstruct:{{state_json:'{state}'}},other_mod:{{marker:7}}}}] 1''')
            else:
                game.call('chat', f'/tconstruct specimen {material} wood wood {damage}')
            wait(f'{material} specimen', lambda: inventory(damage, ingredient, 0))
            game.call('select', '1')
            game.call('chat', '/tconstruct_tool activate')
            wait(f'{material} profile activated', lambda: game.call('inventory')['held'].get('damage') == damage)
            game.call('command', f'item replace entity @s hotbar.1 with {ingredient} {before}')
            wait(f'{material} ingredient', lambda: inventory(damage, ingredient, before))
            game.call('command', 'gamemode survival @s')
            game.call('interact', '0', '71', '2')
            game.call('wait-for', 'screen', 'station', '--timeout', '15')
            game.call('click-gui', '90', '73')
            game.call('wait', '5')
            click(hotbar('minecraft:wooden_pickaxe'))
            click(600)
            click(hotbar(ingredient))
            click(601)
            wait(f'{material} preview', lambda: slot_damage(600) == damage and slot_damage(650) == 0 and menu().get(601, {}).get('count') == before)
            game.call('screenshot', str(run / f'{material}-preview.png'), timeout=30)
            if material == 'flint':
                game.call('close')
                wait('closing retains both inputs in station', lambda: not any(s.get('item') == 'minecraft:wooden_pickaxe' for s in game.call('inventory')['hotbar']))
                game.call('interact', '0', '71', '2')
                game.call('wait-for', 'screen', 'station', '--timeout', '15')
                wait('flint preview after reopening', lambda: slot_damage(600) == damage and slot_damage(650) == 0 and menu().get(601, {}).get('count') == before)
            click(650, 'shift')
            wait(f'{material} output consumes inputs once', lambda: not menu().get(600) and not menu().get(650) and menu().get(601, {}).get('count') == after)
            click(601, 'shift')
            game.call('close')
            wait(f'{material} repaired inventory', lambda: inventory(0, ingredient, after))
            if material == 'flint':
                def foreign_data():
                    inv = game.call('inventory')
                    return any('other_mod:{marker:7}' in s.get('components', '') for s in inv['hotbar'] + inv['main'])
                wait('foreign custom data preserved', foreign_data)
            game.call('save')
            game.stop()
            game = Game(args, run, 'Miner', in_frame=args.in_frame, resume=True)
            game.ready()
            wait(f'{material} persists after reload', lambda: inventory(0, ingredient, after))
            if material == 'flint':
                wait('foreign custom data persists after reload', foreign_data)
        game.call('command', 'gamemode creative @s')
        game.call('command', 'clear @s')
        game.call('chat', '/tconstruct specimen flint wood wood 85')
        wait('failure case specimen', lambda: inventory(85, 'minecraft:stone', 0))
        game.call('select', '1')
        game.call('chat', '/tconstruct_tool activate')
        wait('failure case profile', lambda: game.call('inventory')['held'].get('damage') == 85)
        game.call('command', 'item replace entity @s hotbar.1 with minecraft:stone 1')
        wait('failure case ingredient', lambda: inventory(85, 'minecraft:stone', 1))
        game.call('command', 'gamemode survival @s')
        game.call('interact', '0', '71', '2')
        game.call('wait-for', 'screen', 'station', '--timeout', '15')
        game.call('click-gui', '90', '73')
        game.call('wait', '5')
        click(hotbar('minecraft:wooden_pickaxe'))
        click(600)
        click(hotbar('minecraft:stone'))
        click(601)
        wait('invalid ingredient has no result', lambda: slot_damage(600) == 85 and menu().get(601, {}).get('item') == 'minecraft:stone' and not menu().get(650))
        click(650)
        wait('invalid take leaves inputs unchanged', lambda: slot_damage(600) == 85 and menu().get(601, {}).get('count') == 1 and not menu().get(650))
        click(600, 'shift')
        click(601, 'shift')
        game.call('close')
        wait('invalid inputs returned', lambda: inventory(85, 'minecraft:stone', 1))
        payload = '{"kind":"Head","material":"tconstruct:wood"}'
        game.call('command', f"give @s tconstruct:pick_head[custom_data={{tconstruct:{{state_json:'{payload}'}}}}] 1")
        wait('replacement part available', lambda: any(s.get('item') == 'tconstruct:pick_head' for s in game.call('inventory')['hotbar']))
        game.call('interact', '0', '71', '2')
        game.call('wait-for', 'screen', 'station', '--timeout', '15')
        game.call('click-gui', '109', '73'); game.call('wait', '5')
        click(next(i for i, stack in menu().items() if i < 36 and stack.get('item') == 'minecraft:wooden_pickaxe')); click(600)
        click(hotbar('tconstruct:pick_head')); click(601)
        wait('replacement preview', lambda: menu().get(650, {}).get('item') == 'minecraft:wooden_pickaxe')
        game.call('screenshot', str(run / 'replacement-preview.png'), timeout=30)
        click(650, 'shift')
        wait('replacement consumed once', lambda: not menu().get(600) and not menu().get(601))
        game.call('close')
        wait('replacement material applied', lambda: any(s['materials'][0] == 'tconstruct:wood' for s in states(game.call('inventory'))))
        game.stop()
        report['ok'] = True
    except Exception as error:
        report['error'] = f'{type(error).__name__}: {error}'
        if game:
            try:
                report['menu_on_failure'] = game.call('menu')
                report['inventory_on_failure'] = game.call('inventory')
                report['messages_on_failure'] = game.messages()
            except Exception as diagnostic_error:
                report['diagnostic_error'] = str(diagnostic_error)
    finally:
        if game:
            game.cleanup()
        (run / 'report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
        print(json.dumps({'report': str(run / 'report.json'), **report}), flush=True)
    return 0 if report['ok'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
