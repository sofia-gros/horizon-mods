"""Source checks only; actual Rust API/example execution is a separate gate."""
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[1]
class Ergonomics(unittest.TestCase):
 def test_builder_retains_direct_public_fields_and_moves_vectors(self):
  s=(ROOT/'mixins/pluto_native_creative/inventory-api/src/recipe_book/presentation.rs').read_text()
  self.assertIn('pub entry: Address',s);self.assertIn('pub alternatives: Vec<(usize, Vec<T>)>',s)
  builder=s[s.index('impl<T> Replacement<T>'):s.index('#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum Error')]
  self.assertIn('pub fn for_entry(entry: EntryFacts',builder);self.assertIn('pub fn with_ingredient(mut self',builder)
  self.assertIn('self.alternatives.push((index, alternatives));',builder)
  for bad in ['.clone()', 'Clone>', 'register(', 'static ', 'unsafe ', '.collect(']:self.assertNotIn(bad,builder)
  self.assertEqual(s.count('#[test]'),12);self.assertNotIn('#[ignore]',s)
 def test_actual_fixture_exercises_fluent_nonclone_path(self):
  s=(ROOT/'tests/native/recipe_presentation.rs').read_text()
  self.assertIn('Replacement::for_entry(entry)',s);self.assertIn('.with_ingredient(1, vec![shown])',s)
  self.assertIn('as *const _ as usize, pointer',s)
 def test_examples_use_real_crate_and_do_not_install(self):
  for name in ['presentation.rs','matching_registration.rs']:
   s=(ROOT/'examples/recipes'/name).read_text()
   self.assertIn('use pluto_inventory_api::',s);self.assertIn('fn main() {}',s)
   self.assertNotIn('struct ItemStack',s);self.assertNotIn('struct Book',s)
 def test_docs_keep_status_and_phase_boundaries(self):
  s=(ROOT/'docs/recipe-api-usage.md').read_text()
  for expected in ['have not run','not Fabric', 'Furnace remains unsupported','capturing closures do not','No old TConstruct','not by the builder']:
   self.assertIn(expected,s)
if __name__=='__main__':unittest.main()
