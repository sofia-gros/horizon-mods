"""Particle-type registration surface checks, not native emission proof.

The host has no registry a mod can add a particle type to, so PlutoLib
keeps the declarations itself: a pure registry whose item-crumb factories
store through the item-factory registry (the transports the host already
consults), sprite-set and fluid-sprite declarations over the mod's own
particle assets, and a world send whose host adapter resolves vanilla
names to the host's types and sends any other namespaced name by name
through the level bridge. These checks pin that wiring so it cannot
silently fall away.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
API = ROOT / 'mod' / 'api' / 'src'


class ParticleSurfaceTests(unittest.TestCase):
    def pure(self, name):
        return (API / 'pure' / name).read_text(encoding='utf-8')

    def test_the_pure_module_is_declared_and_exported(self):
        self.assertIn('pub mod particle_api_v1;', self.pure('mod.rs'))
        exports = (API / 'exports.rs').read_text(encoding='utf-8')
        self.assertIn('pub use crate::pure::particle_api_v1::{self, ParticleData, ParticleFactory, ParticleTypeRegistry};', exports)
        self.assertIn('ParticleRecord', exports)

    def test_crumb_factories_and_entity_overrides_delegate_to_the_factory_registry(self):
        source = self.pure('particle_api_v1.rs')
        self.assertIn('ParticleFactoryRegistry::register_item(particle.clone(), stack.clone())', source)
        self.assertIn('ParticleFactoryRegistry::item(particle).map(ParticleFactory::ItemCrumb)', source)
        self.assertIn('ParticleFactoryRegistry::register_entity_override(entity, inherited, replacement)', source)

    def test_the_world_send_is_validated_recorded_and_defaulted(self):
        source = self.pure('world_api_v1.rs')
        self.assertIn('pub fn send_particles(&self, particle: &str, data: &ParticleData', source)
        self.assertIn('fn send_particles(&mut self, _dimension: &str, _particle: &str, _data: &ParticleData', source)
        self.assertIn('pub struct ParticleRecord', source)
        self.assertIn('pub particles: Vec<ParticleRecord>', source)

    def test_the_host_adapter_sends_named_particles_through_the_level_bridge(self):
        hooks = (API / 'hooks.rs').read_text(encoding='utf-8')
        self.assertIn('ParticleOptions::named(particle)', hooks)
        self.assertIn('ParticleType::from_name(particle)', hooks)
        self.assertIn('self.0.get().send_particles(&options, None, at, spread, speed, count)', hooks)

    def test_the_support_matrix_and_docs_cover_the_surface(self):
        support = self.pure('base/support.rs')
        self.assertIn('particle_api_v1::ParticleTypeRegistry::{register,factory,registered,register_entity_particle}', support)
        self.assertIn('world_api_v1::World::send_particles / WorldOps::send_particles', support)
        page = ROOT / 'docs' / 'api' / 'particles.md'
        self.assertTrue(page.exists())
        self.assertIn('[particles](particles.md)', (ROOT / 'docs' / 'api' / 'index.md').read_text(encoding='utf-8'))
        pure_lib = (ROOT / 'tests' / 'pure' / 'src' / 'lib.rs').read_text(encoding='utf-8')
        self.assertIn('page!(Particles, "particles.md");', pure_lib)


if __name__ == '__main__':
    unittest.main()
