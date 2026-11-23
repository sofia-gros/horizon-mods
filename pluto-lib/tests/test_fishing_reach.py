"""The fishing reach's shape: a mod rod casts through the game's own fishing, over a small woven
transport — an access module in the entry (the fishing module is private to the game), a game-side
module inside fishing.rs, and three reaches: the tick head that drains a queued use, the rod check
that keeps a combat hook alive, and the hit head that records the impact. No reach carries
`require`, only the rod check is cancellable, and the API side is the pure fishing module with its
provider installed by the hooks."""
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / 'mod'
PURE = BUNDLE / 'api' / 'src' / 'pure'


def manifest():
    return (BUNDLE / 'horizon-mixin.toml').read_text(encoding='utf-8')


def entry(text, entry_id):
    match = re.search(r'\[\[(?:module|injection)\]\]\nid = "%s"\n(.*?)(?=\n\[\[|\Z)' % re.escape(entry_id), text, re.S)
    return match.group(1) if match else None


class FishingReach(unittest.TestCase):
    def test_the_transport_modules_exist(self):
        text = manifest()
        access = entry(text, 'pluto-fishing-access')
        self.assertIsNotNone(access)
        self.assertIn('file = "crates/horizon-client/src/entry.rs"', access)
        self.assertIn('name = "pluto_fishing_access"', access)
        game = entry(text, 'pluto-fishing-game')
        self.assertIsNotNone(game)
        self.assertIn('file = "crates/horizon-client/src/game/fishing.rs"', game)
        self.assertIn('name = "pluto_fishing_game"', game)
        for source in ('pluto-fishing-access.rs', 'pluto-fishing-game.rs'):
            self.assertTrue((BUNDLE / source).is_file(), source)

    def test_the_three_reaches_have_the_woven_shape(self):
        text = manifest()
        tick = entry(text, 'pluto-fishing-tick-head')
        self.assertIsNotNone(tick)
        self.assertIn('target = "horizon_client::game::fishing::Game<\'_>::tick_fishing"', tick)
        self.assertIn('kind = "HEAD"', tick)
        self.assertNotIn('cancellable', tick)
        holds = entry(text, 'pluto-fishing-holds-rod-head')
        self.assertIsNotNone(holds)
        self.assertIn('target = "horizon_client::game::fishing::Game<\'_>::holds_rod"', holds)
        self.assertIn('cancellable = true', holds)
        hit = entry(text, 'pluto-fishing-hit-head')
        self.assertIsNotNone(hit)
        self.assertIn('target = "horizon_client::game::fishing::Hook::hit_entity"', hit)
        self.assertIn('kind = "HEAD"', hit)
        for reach in (tick, holds, hit):
            self.assertNotIn('require', reach, 'no fishing reach carries require')

    def test_the_helpers_are_single_plain_functions(self):
        for source in ('pluto-fishing-tick-head.rs', 'pluto-fishing-holds-rod-head.rs', 'pluto-fishing-hit-head.rs'):
            body = (BUNDLE / source).read_text(encoding='utf-8')
            self.assertEqual(len(re.findall(r'^fn ', body, re.M)), 1, source)
            self.assertNotIn('///', body, source)
            self.assertNotIn('#[', body, source)

    def test_the_api_side_is_pure_and_installed(self):
        pure = (PURE / 'fishing_api_v1.rs').read_text(encoding='utf-8')
        self.assertIn('pub struct FishingRegistry', pure)
        self.assertIn('pub struct Fishing', pure)
        self.assertIn('pub fn use_rod', pure)
        hooks = (BUNDLE / 'api' / 'src' / 'hooks.rs').read_text(encoding='utf-8')
        self.assertIn('pluto_fishing_access::set_provider', hooks)
        self.assertIn('native_fishing_take_use', hooks)


if __name__ == '__main__':
    unittest.main()
