"""Capture actual game frames before and after a flint TConstruct pickaxe mines iron."""
import json
from pathlib import Path
import tempfile
import time
from types import SimpleNamespace
from probe_horizon_item_state import states
from probe_horizon_server_mods import Game, ROOT


def main():
    args = SimpleNamespace(game=ROOT.parent/'horizon/target/fast/horizon-client',
        ctl=ROOT.parent/'horizon/target/fast/horizon-ctl', resources=ROOT.parent/'horizon/.local/horizon-client/26.3',
        client_mod=ROOT/"Tinkers' onstruct/target/horizon-mod/tconstruct.hmod",
        server_mod=ROOT/"Tinkers' onstruct/server-tools/target/horizon-mod/tconstruct_tools.hmod",
        width=1280, height=720)
    parent = ROOT/'.local/tconstruct-captures'; parent.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='iron-', dir=parent)); report={'ok': False, 'images': []}; game=None
    def wait(label, fn, timeout=20):
        deadline=time.monotonic()+timeout
        while time.monotonic()<deadline:
            value=fn()
            if value: return value
            time.sleep(.2)
        raise AssertionError(label)
    def inspect(tag, damage):
        game.call('chat', f'/tconstruct_tool inspect {tag}')
        def response():
            for line in game.messages().get('chat', []):
                try: info=json.loads(line)
                except (ValueError, TypeError): continue
                if isinstance(info,dict) and info.get('request')==tag: return info
            return None
        info=wait('inspection missing',response)
        if info['damage']!=damage or info['hooks']!='mining': raise AssertionError(info)
        game.call('wait','5')
        path=run/f'{tag}.png'; game.call('screenshot',str(path),timeout=30)
        if not path.is_file() or path.read_bytes()[:8] != b'\x89PNG\r\n\x1a\n': raise AssertionError('screenshot failed')
        report['images'].append(str(path)); report[tag]=info
    try:
        print(f'RUN={run}',flush=True)
        game=Game(args,run,'Miner',in_frame=True); game.ready()
        for cmd in ('clear @s','fill -2 70 -2 2 70 3 minecraft:stone','tp @s 0.5 71 0.5 0 0'):
            game.call('command',cmd)
        game.call('chat','/tconstruct specimen flint wood wood 0')
        wait('specimen missing',lambda: len(states(game.call('inventory')))==1)
        game.call('select','1'); game.call('wait','3')
        game.call('chat','/tconstruct_tool activate')
        wait('profile not active',lambda: 'horizon:tool_control' in game.call('inventory')['held'].get('components',''))
        game.call('command','gamemode survival @s')
        game.call('command','setblock 0 71 2 minecraft:iron_ore')
        wait('iron ore missing',lambda: game.call('block','0','71','2').get('block')=='minecraft:iron_ore')
        inspect('before',0)
        result=game.call('break','0','71','2','--timeout','30',timeout=35)
        if result.get('drops',{}).get('minecraft:raw_iron',0)<1: raise AssertionError(f'iron drop missing: {result}')
        game.call('wait','5')
        wait('wear not observed',lambda: any(s['damage']==1 for s in states(game.call('inventory'))))
        inspect('after',1)
        report['mining_result']=result
        game.stop(); report['ok']=True
    except Exception as e:
        report['error']=f'{type(e).__name__}: {e}'
    finally:
        if game: game.cleanup()
        (run/'report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
        print(json.dumps({'ok':report['ok'],'report':str(run/'report.json'),'error':report.get('error')}),flush=True)
    return 0 if report['ok'] else 1

if __name__=='__main__': raise SystemExit(main())
