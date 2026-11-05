// First-Blazing-Blood bootstrap. Stage mob RNG/events and all machine writes;
// publish both only after the normal source metadata transaction succeeds.
use super::{tconstruct_source_melter_store as store, tconstruct_source_smeltery_state::Smeltery};
use super::tconstruct_source_entity_step::{apply, fill};
use horizon_world::nbt::Tag;
type Pos = (i32, i32, i32);

fn encoded(state: &Smeltery, saved: &Tag) -> Tag {
    let mut next = state.to_nbt();
    if let (Tag::Compound(fields), Some(codes)) = (&mut next, saved.get("heat_codes")) { fields.insert("heat_codes".into(), codes.clone()); }
    next
}

pub(super) fn melt_blazes(game: &mut super::Game<'_>, pos: Pos, min: [i32; 3], max: [i32; 3], original: &Tag) {
    if game.remote.is_some() || !super::tconstruct_source_entity_step::mutable_tank(original) { return; }
    let Some((recipe, sources)) = super::tconstruct_source_entity_world::blaze_context(game) else { return; };
    let Some(saved) = store::read(&game.world, pos, "tconstruct:smeltery") else { return; };
    let Some(state) = Smeltery::from_nbt(&saved) else { return; };
    use horizon_core::geometry::{Aabb, Vec3};
    let area = Aabb::new(Vec3::new(f64::from(min[0]+1), f64::from(min[1]+1), f64::from(min[2]+1)),
        Vec3::new(f64::from(max[0]), f64::from(max[1]+1), f64::from(max[2])));
    let candidates: Vec<_> = game.mobs.iter().enumerate().filter(|(_, mob)| {
        mob.kind.id == "minecraft:blaze" && mob.alive() && !mob.removed
            && sources[usize::from(mob.kind.flags.fire_immune)].is_some()
            && !mob.has_effect("minecraft:fire_resistance") && area.intersects(mob.bounds())
    }).map(|(index, _)| index).collect();
    if candidates.is_empty() { return; }
    // canMeltEntities simulates a fuel search first. Consumption happens only
    // if a native damage call accepted at least one hit.
    let mut fuel_update = None;
    let mut fueled = state.clone();
    if state.fuel == 0 {
        let mut positions: Vec<_> = game.world.chunks.__horizon_mixins_tconstruct_smeltery_owners().iter()
            .filter_map(|(at, owner)| (*owner == pos).then_some(*at)).collect();
        positions.sort_unstable();
        for at in positions {
            let name = &game.res.facts().get(game.world.block(at.0, at.1, at.2)).name;
            let Some(capacity) = super::tconstruct_source_smeltery_fuel::capacity(name) else { continue; };
            let Some(kind) = super::tconstruct_source_machine_runtime::kind(name) else { continue; };
            let Some(raw) = store::read(&game.world, at, kind) else { continue; };
            let Some(mut tank) = super::tconstruct_source_melter_state::Tank::from_nbt(&raw, capacity) else { continue; };
            let Some(fluid) = tank.fluid.as_deref() else { continue; };
            let Some(fuel) = super::tconstruct_source_melter_world::fluid_fuel(&game.world, fluid, tank.amount) else { continue; };
            if fuel.temperature == 0 || tank.take(fuel.fluid.amount).is_none() { continue; }
            let Some(next) = tank.to_nbt_preserving_fuel(&raw) else { continue; };
            fueled.fuel = fuel.duration; fueled.fuel_quality = fuel.duration;
            fueled.temperature = fuel.temperature; fueled.rate = fuel.rate;
            fueled.fuel_view.last = Some(at);
            fuel_update = Some((at, kind, next)); break;
        }
        if fuel_update.is_none() { return; }
    }
    // Prove the maximum possible tank mutation is writable before callbacks.
    // The native incoming-damage hook has no mutable Game/world argument.
    let mut maximum = fueled.clone();
    for _ in &candidates { fill(&mut maximum, &recipe); }
    let mut updates = vec![(pos, "tconstruct:smeltery", encoded(&maximum, &saved))];
    if let Some(update) = &fuel_update { updates.push(update.clone()); }
    if !store::can_write_many(&game.world, &updates) { return; }
    let committed = apply(&mut game.mobs, &candidates, &state, &fueled, &recipe, sources, |state, accepted| {
        updates[0].2 = encoded(state, &saved);
        if !accepted { updates.truncate(1); }
        store::write_many(&mut game.world, &updates)
    });
    if committed {
        for index in candidates {
            let mob = &game.mobs[index];
            game.world.modified.insert(((mob.position.x.floor() as i32)>>4, (mob.position.z.floor() as i32)>>4));
        }
    }
}
