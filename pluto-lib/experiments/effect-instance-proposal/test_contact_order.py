"""Source-pinned ordering constraints, not native actor/borrow execution tests."""
import importlib.util
import os
from pathlib import Path
import unittest
HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('contact_sources',HERE/'contact_sources.py')
p=importlib.util.module_from_spec(spec);spec.loader.exec_module(p)

class NativeContactOrder(unittest.TestCase):
    @classmethod
    def setUpClass(cls): cls.sources=p.read(Path(os.environ['PLUTO_APPROVED_OFFICIAL']))
    def src(self,name): return self.sources['crates/horizon-client/src/'+name]
    def ordered(self,body,*anchors):
        cursor=0
        for anchor in anchors:
            index=body.find(anchor,cursor)
            self.assertNotEqual(index,-1,anchor)
            cursor=index+len(anchor)
    def player(self): return p.span(self.src('motion.rs'),'    fn check_inside_blocks(', '    fn honey_block_inside(')
    def mob(self): return p.span(self.src('mobs/ai.rs'),'fn effects_from_blocks(', '#[cfg(test)]')
    def test_player_snapshot_then_visited_then_emit_then_native_action(self):
        body=self.player()
        self.ordered(body,'let alive = !self.player.health.dead_or_dying();','sweep_intersected_blocks(',
            'hits.push((iteration, p, state));','for (step, p, state) in hits {',
            'if inside && visited.insert(p.pack()) {','Event::Inside(',
            'match name {','if damage_only {','continue;','MotionEvent::EnterBlock { state }')
        self.assertIn('let name = definition.name.as_str();',body)
        self.assertIn('Ok(iterations + 1)',body)
    def test_mob_emit_precedes_visited_and_live_name_read(self):
        body=self.mob()
        self.ordered(body,'let steps = ((longest / 0.2).ceil() as i32).clamp(1, 100);',
            'for step in 0..=steps {','if !m.alive() {','for x in ', 'for y in ', 'for z in ',
            'if visited.contains(&p) {','Event::Inside(', 'visited.push(p);','match w.block_name(x, y, z) {')
        self.assertNotIn('sweep_intersected_blocks',body)
        self.assertNotIn('VoxelShape',body)
    def test_player_shape_and_air_rules_are_not_mob_rules(self):
        self.assertIn('"minecraft:cave_air"',self.player())
        self.assertIn('"minecraft:end_portal" | "minecraft:tripwire"',self.player())
        self.assertIn('collision_shape_with_context(',self.player())
        self.assertNotIn('"minecraft:cave_air"',self.mob())
        self.assertNotIn('"minecraft:end_portal" | "minecraft:tripwire"',self.mob())
    def test_player_outer_budget_and_powder_groups_survive(self):
        body=p.span(self.src('motion.rs'),'    fn effects_from_blocks(', '    pub fn passenger_block_effects_tick(')
        self.ordered(body,'self.player.history.clear();','if self.player.spectator {',
            'Event::StepOn(', 'let mut visited =', 'let mut entered =','for movement in &movements {',
            'let mut budget = 16;', 'Axis::step_order(original)', 'if budget <= 0 {', 'let mut groups = 0;',
            'for _ in 0..groups {','if self.player.health.dead_or_dying() {')
        self.assertIn('groups == 1 && !damage_only',body)
    def test_guest_surrounding_hazards_and_swaps_remain_ordered(self):
        source=self.src('game/lan_actions.rs')
        wrapper=p.span(source,'    pub(super) fn as_guest_with_host', '    pub(super) fn as_guest_messaged')
        self.ordered(wrapper,'self.enter_guest(', 'block_contact::in_level(', 'f(self, host)', 'self.leave_guest(')
        body=p.span(source,'    fn guest_server_tick(', '    pub(super) fn ')
        self.ordered(body,'self.as_guest(host, conn, |game| {','let mut w = MotionWorld {',
            'w.fluid_base_tick()', 'w.server_block_damage(start)', 'w.update_soul_speed_block()',
            'w.tick_environment_damage()', 'w.apply_fire_contact()', 'w.server_hazards_tick(in_wall)')
        self.assertEqual(source.count('self.potions.swap_player(&mut g.effects, &mut play.potions);'),2)
    def test_complete_player_add_needs_mutable_game_callbacks(self):
        source=self.src('game/potions.rs')
        body=p.span(source,'    pub(super) fn add_player_effect(', '    pub(super) fn mod_effect_added(')
        self.ordered(body,'let new =', 'self.potions.effects.add(effect)', 'self.apply_effect_status()',
            'if id == "absorption"', 'p.set_health(', 'self.mod_effect_added(')
        hooks=self.src('game/native_hooks.rs')
        for name in ['native_on_effect_added','native_mob_effect_added']:
            self.assertIn('fn '+name+'(\n        &mut self,',hooks)
    def test_mob_live_owner_is_detached_from_game_during_tick(self):
        source=self.src('game/creatures.rs')
        self.ordered(source,'let mut mobs = std::mem::take(&mut self.mobs);',
            'let mut base = self.mob_world_base(&[]);', 'for i in 0..mobs.len() {',
            'horizon_client::mobs::riders::tick(&mut mobs, i, &w, &mut scratch, &mut events)',
            'if self.mobs.is_empty() {', 'self.mobs = mobs;', 'self.mobs.append(&mut mobs);')
        self.assertIn('mod_effects: if self.remote.is_none() {\n                Some(self)',source)
    def test_mob_passenger_navigation_loan_spans_inner_tick(self):
        body=p.span(self.src('mobs/riders.rs'),'pub fn tick(', 'pub fn keeps_alive(')
        self.ordered(body,'mobs.split_at_mut(', 'std::mem::swap(&mut rider.ai.nav, &mut vehicle.ai.nav);',
            'rider.ai.navigation_redirect = Some(', 'let kept = rider.tick(w, random, events);',
            '.expect("navigation loan lasts for the tick")', 'vehicle.ai.nav =')
    def test_mob_visibility_ids_are_not_player_hidden_instances(self):
        mob=self.src('mobs.rs')
        self.assertIn("pub effects: Vec<(&'static str, i32, i32)>",mob)
        self.assertIn('The effects above given with `hideParticles`',mob)
        self.assertIn("pub hidden_effects: Vec<&'static str>",mob)
        self.assertIn('a weaker one is\n            // ignored (vanilla keeps it hidden behind the active one)',mob)
        player=self.sources['crates/horizon-client-render/src/effects.rs']
        self.assertIn('pub hidden: Option<Box<EffectInstance>>',player)
    def test_existing_dispatch_and_client_motion_do_not_lend_game_authority(self):
        hook=self.src('block_contact.rs')
        self.assertIn("OnceLock<fn(&str, Event<'_>, &mut f32)>",hook)
        self.assertIn('if contact.block.starts_with("minecraft:")',hook)
        self.assertIn('MotionWorld {',self.src('game/prediction.rs'))
        self.assertIn('passenger.passenger_block_effects_tick()',self.src('game/remote.rs'))
        self.assertIn('(self.serving.is_some() || self.options.dedicated.is_some()).then_some(dim.id())',self.src('game/levels.rs'))

if __name__=='__main__': unittest.main()
