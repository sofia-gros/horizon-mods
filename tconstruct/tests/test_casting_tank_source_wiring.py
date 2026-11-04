from pathlib import Path
import unittest
P=Path(__file__).resolve().parents[1]/'mixins/tconstruct/source-machine-runtime.rs'
class Wiring(unittest.TestCase):
    def test_scope_and_notification(self):
        s=P.read_text();start=s.index('enum CastingInput {');end=s.index('pub(super) fn activate_faucet(',start);b=s[start:end]
        self.assertIn('name == "tconstruct:seared_casting_tank"',b)
        self.assertIn('state.to_nbt() != raw',b)
        self.assertEqual(b.count('source_strength.is_some() && casting.tank.tag.is_some()'),2)
        self.assertEqual(b.count('notify_casting_comparator(game,source,before,after)'),2)
        self.assertNotIn('process_casting_tank(',b)
if __name__=='__main__':unittest.main()
