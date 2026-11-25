//! Connects the edible modifiers to PlutoLib's per-stack item use and finish event. A Tinkers tool that charges an interaction and carries the
//! edible module is eaten with a right click, for its eat duration, only while it is whole, its hunger stat is above 0 and the player is hungry
//! (`EdibleModule.onToolUse`); other tools, and a tool the use refuses, keep the game's own use. When the use is done the player is fed, the
//! eating sounds play and each modifier's edible effects run, wearing the tool in the hand (`EdibleModule.eat`).
use crate::{break_speed_hook::with_held_tool, edible, particles, tool_damage_hook, tool_slot, tool_stack, tool_stats::Stat};
use pluto_lib::{Hand, Identifier, ItemEvents, ItemUseRegistry, ParticleData, StackData, StackSlot, StackUse, UseAction, UseItemFinishCallback, UseItemFinishContext, World};

/// The tool items `EdibleModule.onToolUse` can start eating with: those tagged `tconstruct:modifiable/interactable/charge` (a tool that charges
/// its right click: the swasher, the right-click and dual tools, the shields and the bows). Armor is never eaten with a right click, which
/// equips it, and the stat's other valid tools are not interactable.
pub(crate) const USE_ITEMS: [&str; 29] = [
    "swasher", "pickaxe", "sledge_hammer", "vein_hammer", "mattock", "pickadze", "excavator", "hand_axe", "broad_axe", "minotaur_axe", "kama", "scythe", "dagger",
    "sword", "cleaver", "javelin", "flint_and_brick", "fishing_rod", "sky_staff", "earth_staff", "ichor_staff", "ender_staff", "melting_pan",
    "battlesign", "travelers_shield", "plate_shield", "longbow", "crossbow", "war_pick",
];

/// The hunger and eat duration stats of a whole Tinkers tool that carries the edible module; None for anything else, a broken tool included.
/// Shared with the blocking hook, whose registration for the shield items chooses per stack between the two uses.
pub(crate) fn eating(stack: &StackData) -> Option<(f32, f32)> {
    with_held_tool(stack, |env, tool| {
        tool_stack::edible(&tool.custom_data, env)?;
        Some((tool_stack::stat(&tool.custom_data, Stat::Hunger), tool_stack::stat(&tool.custom_data, Stat::EatDuration)))
    }).ok().flatten()
}

/// What eating a tool gives, from its stats and modifiers: the nutrition and saturation the conditional stats make of its stats, and the
/// effects its modifiers run afterwards. None when the tool is not edible or its hunger rounds to 0.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Meal { pub nutrition: i32, pub saturation: f32, pub effects: Vec<edible::Effect> }

pub(crate) fn meal(nbt: &pluto_lib::NbtCompound, env: &tool_stack::Environment) -> Option<Meal> {
    let effects = tool_stack::edible(nbt, env)?;
    // The eater's state tags are not known to the finish event, so a conditional stat that needs them does not apply.
    let no_state = |_: &Identifier| false;
    let no_entities = crate::entity_facts::Entities { attacker: None, target: None };
    let nutrition = edible::nutrition(tool_stack::conditional_stat(nbt, env, Stat::Hunger, &no_state, &no_entities));
    let saturation = tool_stack::conditional_stat(nbt, env, Stat::Saturation, &no_state, &no_entities);
    (nutrition > 0).then_some(Meal { nutrition, saturation, effects })
}

/// The tool in the hand once the meal's durability costs are paid, and whether paying them broke
/// it (EdibleConsumeDurabilityModule's directDamage answer): a creative eater pays nothing.
pub(crate) fn pay(meal: &Meal, hand: &pluto_lib::ItemSlot, env: &tool_stack::Environment, creative: bool) -> (pluto_lib::ItemSlot, bool) {
    let mut worn = hand.clone();
    let mut broke = false;
    for effect in &meal.effects {
        if let edible::Effect::ConsumeDurability(amount) = effect {
            match tool_slot::direct_damage_slot(&mut worn, env, *amount, !creative) {
                Ok(outcome) => broke |= outcome.broke,
                Err(reason) => eprintln!("Tinkers: eaten tool not worn: {reason}"),
            }
        }
    }
    (worn, broke)
}

/// The representative items' crumbs at `at`, `count` each (EdibleRepresentativeItemModule.eatEffects'
/// item particles, through the lazy crumb types in particles.rs: the port's particle spawn carries
/// no per-spawn item, so each representative item bursts through a type of its own).
fn crumbs(world: &World, items: &[Identifier], at: [f64; 3], count: i32) {
    for item in items {
        let crumb = particles::edible_crumb(item);
        world.send_particles(&crumb.to_string(), &ParticleData::Plain, at, [0.5, 0.5, 0.5], 0.0, count);
    }
}

/// The finish event: feeds the eater, plays the sounds, runs the edible effects and hands back the worn tool.
fn finished(done: &UseItemFinishContext) {
    // The hand must still hold the tool that was eaten.
    let Some(hand) = done.slot.as_ref().filter(|hand| hand.item.to_string() == done.item && hand.custom_data == done.stack.custom_data) else { return };
    let Some(status) = done.player.food() else { return };
    if !edible::can_eat(done.creative, status.level) { return; }
    let Ok(Some((meal, worn, broke, presented))) = with_held_tool(&done.stack, |env, tool| meal(&tool.custom_data, env).map(|meal| { let (worn, broke) = pay(&meal, hand, env, done.creative); let presented = tool_stack::representative_items(&tool.custom_data, env); (meal, worn, broke, presented) })) else { return };
    done.player.add_food(meal.nutrition, meal.saturation);
    let world = World::new(&done.dimension);
    // The completion burst: sixteen crumbs of each representative item at the eater.
    crumbs(&world, &presented, done.position, 16);
    for (sound, volume, pitch) in edible::sounds(&mut tool_damage_hook::random) { world.play_sound(sound, done.position, volume, pitch); }
    for effect in &meal.effects {
        match effect {
            edible::Effect::ConsumeDurability(_) => {}
            edible::Effect::CureRandomEffect => {
                let held: Vec<String> = done.player.effects().into_iter().map(|effect| effect.id).collect();
                if let Some(id) = edible::pick_effect(&held, tool_damage_hook::random()) { done.player.remove_effect(id); }
            }
            edible::Effect::RemoveEffect(effect) => { done.player.remove_effect(&effect.to_string()); }
        }
    }
    if worn != *hand {
        // EdibleConsumeDurabilityModule pays with a direct damage and broadcasts the break by
        // hand when it broke: a meal that broke the tool announces it, at the hand that ate. A
        // break always changes the stack, so it is announced exactly when the hand-back is set.
        if broke {
            done.player.broadcast_item_break(match done.hand { Hand::Main => StackSlot::MainHand, Hand::Off => StackSlot::OffHand }, &crate::brush_hook::stack_of(&worn));
        }
        *done.replacement.borrow_mut() = Some(worn);
    }
}

/// Registers the use and the finish callback. Called once from the product's hooks.
pub(crate) fn register() {
    for path in USE_ITEMS {
        let Ok(item) = Identifier::new("tconstruct", path) else { continue };
        ItemUseRegistry::register_stack(item, StackUse::new(UseAction::Eat)
            .can_use(|stack| eating(stack).is_some_and(|(hunger, _)| edible::can_start(hunger, false)))
            .use_time(|stack| eating(stack).map_or(0, |(_, duration)| edible::use_ticks(duration)))
            .requires_hunger());
    }
    UseItemFinishCallback::EVENT.register(finished);
    // EdibleRepresentativeItemModule.onUsingTick: while an edible tool is being eaten, every
    // fourth tick of the use plays the eating sound of its representative item and bursts five
    // crumbs of each representative item at the eater's eyes. The completion burst of the
    // original (sixteen crumbs) rides the finish above, whose own sounds play.
    ItemEvents::USE_TICK.register(|tick| {
        if tick.ticks_used >= tick.duration || tick.ticks_used % 4 != 0 { return; }
        let Some(status) = tick.player.food() else { return; };
        if !edible::can_eat(tick.creative, status.level) { return; }
        let Ok(presented) = with_held_tool(&tick.stack, |env, tool| {
            if tool_stack::is_broken(&tool.custom_data) || tool_stack::stat(&tool.custom_data, Stat::Hunger) as i32 <= 0 { return Vec::new(); }
            tool_stack::representative_items(&tool.custom_data, env)
        }) else { return; };
        if presented.is_empty() { return; }
        let world = World::new(&tick.dimension);
        let (sound, volume, pitch) = edible::eat_sound(&mut tool_damage_hook::random);
        world.play_sound(sound, tick.position, volume, pitch);
        crumbs(&world, &presented, tick.eye, 5);
    });
    // EdibleRepresentativeItemModule.beforeReleaseUsing: a use released at its full duration
    // bursts five crumbs of each representative item. A use that ran its whole time out is
    // finished, not released, so this never doubles the finish's burst.
    ItemEvents::RELEASE_USING.register(|release| {
        let Some((_, duration)) = eating(&release.stack) else { return; };
        if release.ticks_used as i32 != edible::use_ticks(duration) { return; }
        let Some(status) = release.player.food() else { return; };
        if !edible::can_eat(release.creative, status.level) { return; }
        let Ok(presented) = with_held_tool(&release.stack, |env, tool| tool_stack::representative_items(&tool.custom_data, env)) else { return; };
        if presented.is_empty() { return; }
        crumbs(&release.world(), &presented, release.eye, 5);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::world_api_v1::{with_world, MemoryWorldOps};

    #[test]
    fn crumbs_burst_each_representative_item_by_the_count() {
        let items = vec![
            Identifier::new("tconstruct", "cheese_ingot").expect("valid id"),
            Identifier::new("minecraft", "apple").expect("valid id"),
        ];
        let mut ops = MemoryWorldOps::default();
        with_world(&mut ops, || {
            let world = World::new("minecraft:overworld");
            crumbs(&world, &items, [1.0, 64.0, 2.0], 5);
        });
        assert_eq!(ops.particles.len(), 2);
        assert_eq!(ops.particles[0].particle, "tconstruct:edible_crumb_tconstruct_cheese_ingot");
        assert_eq!(ops.particles[0].count, 5);
        assert_eq!(ops.particles[0].at, [1.0, 64.0, 2.0]);
        assert_eq!(ops.particles[1].particle, "tconstruct:edible_crumb_minecraft_apple");
        assert_eq!(ops.particles[1].count, 5);
    }
}
