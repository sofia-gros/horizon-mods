"""Exact official inputs for an UNAPPLIED contact refactor design; no writes."""
from pathlib import Path
import hashlib
import subprocess
REVISION = '624c4f6265c02296fbe8a80f029dd0d987fb94c0'
FILES = {
 'crates/horizon-client/src/block_contact.rs': '359d9c95134d07e86e3ba92bcdc9fc23afbe59799bf5fc5e46c20e43c1c9d143',
 'crates/horizon-client/src/motion.rs': '32caf71a62f87dda437f650e271922a9e5add6dec7056a67cf0d82471b56a4ae',
 'crates/horizon-client/src/mobs.rs': '425f605cf6daeaa0f5e76b8bed5bcfee5110daef3a741ea34fc66133a04a6028',
 'crates/horizon-client/src/mobs/ai.rs': '81ecf5da5119d8711de9c3ca5689c78758202ad315268b7e08ccb93ffe4b8056',
 'crates/horizon-client/src/mobs/riders.rs': 'ca22f78d4b5b741f212778f36d695929a5f5b010366bf1d9c137e1924c449d44',
 'crates/horizon-client/src/game/creatures.rs': 'b5a2e3f52ac802182855251d2b67dec975b228550aaaaf3a0582560dee8ba90b',
 'crates/horizon-client/src/game/lan_actions.rs': 'a2ea07508ddc14b34fa638e0d11af31d86b9e18cd2e8a7d286e33616c4327a57',
 'crates/horizon-client/src/game/potions.rs': '9d504ba0534f79a5a040d85e40649b14943847cb9691a2621c9dfe37ef45f460',
 'crates/horizon-client/src/game/native_hooks.rs': '195b19d3647e21caf33757aa690b9df8cc5b562e2943a36404f7a26c6e4a60e1',
 'crates/horizon-client/src/game/prediction.rs': 'bb58dc9294cd549a0a2e3fc9e7de7e5dd733d7333e052594dc90c080e834a717',
 'crates/horizon-client/src/game/remote.rs': '331d12df620e605fda197e96465435be4f41e85db1e336b633eda37444f8fe71',
 'crates/horizon-client/src/game/levels.rs': '839eec483b745eef8cfdf8ac2502160e46517e1dbd62534b69e612d2828b7ba7',
 'crates/horizon-client-render/src/effects.rs': '5269c3c35f38c4c658d4409b0f7badae4f0ddca0b68df36c5197a5dd51f735dd',
}

def read(official: Path):
    if subprocess.check_output(['git','-C',str(official),'rev-parse','HEAD'],text=True).strip() != REVISION:
        raise ValueError('wrong approved official revision')
    if subprocess.check_output(['git','-C',str(official),'status','--porcelain','--untracked-files=no'],text=True):
        raise ValueError('official tracked sources are not clean')
    result = {}
    for path, expected in FILES.items():
        data = (official / path).read_bytes()
        if hashlib.sha256(data).hexdigest() != expected: raise ValueError('source changed: '+path)
        result[path] = data.decode()
    return result

def span(source, start, end):
    if source.count(start) != 1: raise ValueError('nonunique start: '+start)
    tail = source.split(start,1)[1]
    if end not in tail: raise ValueError('missing end: '+end)
    return start + tail.split(end,1)[0]
