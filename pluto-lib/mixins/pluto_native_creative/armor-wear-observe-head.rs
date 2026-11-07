// Adapted from official horizon-rust/horizon 624c4f6265c02296fbe8a80f029dd0d987fb94c0.
// Copyright (c) 2026 Ludovic Marie and the Horizon contributors. MIT.
// See HORIZON-DAMAGE-LICENSE.txt. Regenerate/review on every native body change.
// MIT License
//
// Copyright (c) 2026 Ludovic Marie and the Horizon contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
fn pluto_armor_wear_observe(
    game: &mut crate::game::Game<'_>,
    source: horizon_world::damage_source::DamageTypeId,
    damage: f32,
) -> std::ops::ControlFlow<()> {
    // Match the native nonpositive gate before taking any observation.
    // NaN is intentionally not rejected here: preserve native comparison.
    if damage <= 0.0 {
        return std::ops::ControlFlow::Continue(());
    }
    let Some(observation) = crate::game::pluto_armor_wear::begin(game, source, damage) else {
        return std::ops::ControlFlow::Continue(());
    };
    fn original(
        game: &mut crate::game::Game<'_>,
        source: horizon_world::damage_source::DamageTypeId,
        damage: f32,
    ) {
        if damage <= 0.0 {
            return;
        }
        let amount = ((damage / 4.0) as u32).max(1);
        let name = game
            .health_resources
            .damage
            .definition(source)
            .map(|d| d.name.clone())
            .unwrap_or_default();
        let fire = game
            .health_resources
            .damage
            .in_tag(source, "minecraft:is_fire");
        let mut broke = false;
        let mut broken_items = Vec::new();
        let losses: Vec<Option<(u32, u32)>> = (0..4)
            .map(|i| {
                let stack = game.armor[i].clone()?;
                let info = world_items!(game).get(&stack.item)?;
                let component = |id| stack.data().and_then(|data| data.other(id));
                // ItemStack.isDamageableItem: component removal and
                // unbreakable are checked before enchantment RNG or hooks.
                if stack.count == 0
                    || !info.equip_damage_on_hurt
                    || component("minecraft:unbreakable").is_some()
                    || component("!minecraft:max_damage").is_some()
                    || component("!minecraft:damage").is_some()
                    || (fire && world_items!(game).resists_fire(&stack, &name))
                {
                    return None;
                }
                let max = match component("minecraft:max_damage") {
                    Some(horizon_world::nbt::Tag::Int(value)) => u32::try_from(*value).ok()?,
                    _ => info.max_damage,
                };
                if max == 0 {
                    return None;
                }
                let loss = game.wear(&stack, amount);
                (loss > 0).then_some((loss, max))
            })
            .collect();
        for (index, (slot, loss)) in game.armor.iter_mut().zip(losses).enumerate() {
            if let (Some(stack), Some((amount, max))) = (slot.as_mut(), loss) {
                stack.damage = stack.damage.saturating_add(amount).min(max);
                if stack.damage >= max {
                    broken_items.push((stack.clone(), index));
                    *slot = None;
                    broke = true;
                }
            }
        }
        if broke {
            // LivingEntity.onEquippedItemBroken for each armour piece that broke.
            for (stack, index) in &broken_items {
                game.player_equipment_broke(armor_slot(*index), stack);
            }
            game.apply_equipment();
        }
    }
    original(game, source, damage);
    observation.finish(game);
    std::ops::ControlFlow::Break(())
}
