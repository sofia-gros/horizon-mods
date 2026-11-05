use super::{Game, Hand, tconstruct_offhand_attack as rule};
use horizon_client::items::ItemStack;
use horizon_world::nbt::Tag;
use std::cell::Cell;

#[derive(Clone, Copy)]
struct Attack {
    actor: Option<u32>,
    damage: f32,
    knockback: f32,
    charge: f32,
    target: u32,
    hurt_delay: i32,
    depth: usize,
    nested: bool,
}
thread_local! { static ATTACK: Cell<Option<Attack>> = const { Cell::new(None) }; }
struct Scope(Option<Attack>);
impl Drop for Scope {
    fn drop(&mut self) {
        ATTACK.with(|a| a.set(self.0));
    }
}
pub(super) fn enter(game: &Game<'_>) {
    ATTACK.with(|cell| {
        if let Some(mut a) = cell.get() {
            if a.actor == game.lan_acting {
                a.depth += 1;
                a.nested |= a.depth > 1;
                cell.set(Some(a));
            }
        }
    });
}
pub(super) fn leave(game: &Game<'_>) {
    ATTACK.with(|cell| {
        if let Some(mut a) = cell.get() {
            if a.actor == game.lan_acting {
                a.depth = a.depth.saturating_sub(1);
                cell.set(Some(a));
            }
        }
    });
}
fn active(game: &Game<'_>) -> Option<Attack> {
    ATTACK.with(|a| {
        a.get()
            .filter(|a| a.actor == game.lan_acting && a.depth == 1)
    })
}
pub(super) fn damage(game: &Game<'_>) -> Option<f32> {
    active(game).map(|a| a.damage)
}
pub(super) fn charge(game: &Game<'_>) -> Option<f32> {
    active(game).map(|a| a.charge)
}
pub(super) fn knockback(game: &Game<'_>) -> Option<f32> {
    active(game).map(|a| a.knockback)
}

/// ToolAttackUtil.getToolAttribute: remove main-hand modifiers on a clone,
/// add the offhand tool stat to the base, then evaluate retained multipliers.
fn projected(game: &Game<'_>, attribute: &str, amount: f32) -> Option<f32> {
    let mut player = game.player.clone()?;
    if let Some(main) = game.inventory.get(game.selected)?.as_ref() {
        let info = game.items().get(&main.item)?;
        for (attr, id, _, _, group) in &info.modifiers {
            if attr == attribute && super::equipment_modifier_applies(group, "mainhand") {
                player
                    .set_modifier(attr, id, 0.0, "add_value", false)
                    .ok()?;
            }
        }
    }
    // The source adds toolValue to the base before multiply_base. An additive
    // modifier would miss that interaction with armor/command modifiers.
    let key = player.attributes.key(attribute)?;
    let handle = player.attributes.get_instance(Some(key)).ok()??;
    player
        .attributes
        .edit(&handle, |instance, observer| {
            instance.set_base_value(instance.base_value() + f64::from(amount), observer)
        })
        .ok()?;
    let value = player.attribute(attribute).ok()? as f32;
    (value.is_finite() && value >= 0.0).then_some(value)
}
struct Config {
    damage: f32,
    speed: f32,
    knockback: f32,
    hurt_delay: i32,
}
fn config(game: &Game<'_>, stack: &ItemStack) -> Option<Config> {
    if game.remote.is_some()
        || game.hand != Hand::Off
        || game.spectator()
        || stack.item.as_ref() != "tconstruct:dagger"
        || stack.count != 1
        || stack.wire_patch().is_some()
        || game.offhand.as_ref() != Some(stack)
        || !stack.enchantments().is_empty()
    {
        return None;
    }
    super::tconstruct_source_melee_shape::intact(stack)?;
    let player = game.player.as_ref()?;
    if player.health.health() <= 0.0 || player.auto_spin_attack_dmg > 0.0 {
        return None;
    }
    let main = game.inventory.get(game.selected)?.as_ref();
    if game.attack_item.as_deref() != main.map(|s| s.item.as_ref()) {
        return None;
    }
    // Existing dynamic melee modules read selected mainhand. Do not give their
    // effects to this offhand hit. The mace also selects its mainhand directly.
    if main.is_some_and(|s| {
        !s.item.starts_with("minecraft:")
            || s.item.as_ref() == "minecraft:mace"
            || s.wire_patch().is_some()
            || s.data().is_some()
            || !s.enchantments().is_empty()
    }) {
        return None;
    }
    if [
        "minecraft:attribute_modifiers",
        "minecraft:consumable",
        "minecraft:use_cooldown",
        "minecraft:attack_range",
    ]
    .into_iter()
    .any(|id| stack.data().and_then(|d| d.other(id)).is_some())
    {
        return None;
    }
    let item = game.items().get(&stack.item)?;
    if item.consumable.is_some() || item.use_cooldown.is_some() {
        return None;
    }
    let tags = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_resolved_item_tags()
        .as_ref()?
        .as_ref()
        .ok()?;
    if ![
        "tconstruct:modifiable/melee",
        "tconstruct:modifiable/melee/weapon",
        "tconstruct:modifiable/durability",
    ]
    .into_iter()
    .all(|tag| {
        tags.get(tag)
            .is_some_and(|set| set.contains(stack.item.as_ref()))
    }) {
        return None;
    }
    let raw = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_modifier_resource_snapshot()
        .as_ref()?
        .as_ref()
        .ok()?;
    let definition = |id| super::tconstruct_modifier_snapshot::highest_definition(raw, id);
    // Offhand Attack is the pinned static Java modifier, not a JSON module.
    if definition("tconstruct:tinkering/modifiers/offhand_attack").is_some() {
        return None;
    }
    let materials = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_material_snapshot()
        .as_ref()?;
    if !rule::known(
        "tool",
        materials
            .get("tconstruct:tinkering/tool_definitions/dagger")?
            .last()
            .map(Vec::as_slice),
    ) || !super::tconstruct_padded::known(definition("tconstruct:tinkering/modifiers/padded"))
    {
        return None;
    }
    let traits = super::tconstruct_source_material_traits::levels(&game.world, stack)?;
    let level =
        |id| super::tconstruct_source_material_traits::level_with_upgrade(stack, &traits, id);
    if level("tconstruct:offhand_attack")? != 1
        || level("tconstruct:padded")? <= 0
        || level("tconstruct:silky")? > 0
    {
        return None;
    }
    for id in super::tconstruct_source_material_effects::ordered_modifier_ids(stack, &traits.levels)
    {
        if level(&id)? == 0
            || matches!(
                id.as_str(),
                "tconstruct:offhand_attack" | "tconstruct:padded"
            )
        {
            continue;
        }
        let name = id.strip_prefix("tconstruct:")?;
        if !rule::known(
            name,
            definition(&format!("tconstruct:tinkering/modifiers/{name}")),
        ) {
            return None;
        }
    }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    let stat = |key| match custom.get(key)? {
        Tag::Float(bits) => {
            let n = f32::from_bits(*bits);
            (n.is_finite() && n >= 0.0).then_some(n)
        }
        _ => None,
    };
    Some(Config {
        damage: projected(game, "minecraft:attack_damage", stat("tic_attack_damage")?)?,
        speed: projected(
            game,
            "minecraft:attack_speed",
            stat("tic_attack_speed")? - 4.0,
        )?,
        knockback: projected(game, "minecraft:attack_knockback", 0.0)?,
        hurt_delay: rule::cooldown(stat("tic_attack_speed")?)?,
    })
}
/// Consume entity use or a miss, leaving normal main-hand use precedence intact.
pub(super) fn use_tool(game: &mut Game<'_>, stack: &ItemStack, target: Option<i32>) -> bool {
    if ATTACK.with(|a| a.get().is_some()) {
        return false;
    }
    let Some(config) = config(game, stack) else {
        return false;
    };
    let index = match target {
        Some(id) => {
            let Some(index) = game.mobs.iter().position(|m| m.id as i32 == id) else {
                return false;
            };
            let mob = &game.mobs[index];
            // Other entity families and multipart bosses have their own entry
            // points. This slice claims only ordinary live mob interactions.
            if mob.removed
                || !mob.alive()
                || !super::tconstruct_source_spilling::eligible_mob(mob)
                || Some(mob.id) == game.riding
                || !mob.attrs.knockback_resistance.is_finite()
                || !(0.0..=1.0).contains(&mob.attrs.knockback_resistance)
            {
                return false;
            }
            Some(index)
        }
        None => None,
    };
    let now = game.player.as_ref().unwrap().health.tick_count;
    let uuid = game.acting_player_uuid();
    let last = game
        .world
        .chunks
        .__horizon_mixins_tconstruct_offhand_cooldown()
        .get(&uuid)
        .copied();
    let Some(charge) = rule::charge(now, last) else {
        return false;
    };
    let Some(duration) = rule::cooldown(config.speed) else {
        return false;
    };
    // Reserve before dispatch so nested use cannot spend the same tick twice.
    game.world
        .chunks
        .__horizon_mixins_tconstruct_offhand_cooldown_mut()
        .insert(uuid, (now, duration));
    if let Some(index) = index.filter(|_| config.damage > 0.0) {
        let old_ticks = game.attack_ticks;
        let scope = Scope(ATTACK.with(|a| {
            a.replace(Some(Attack {
                actor: game.lan_acting,
                damage: config.damage,
                knockback: config.knockback,
                charge,
                target: game.mobs[index].id,
                hurt_delay: config.hurt_delay,
                depth: 0,
                nested: false,
            }))
        }));
        game.attack_mob(index);
        // attack_mob resets mainhand directly. Do not erase a nested ordinary
        // attack's legitimate reset if another mod performed one synchronously.
        let nested = ATTACK.with(|a| a.get().is_some_and(|a| a.nested));
        drop(scope);
        if !nested {
            game.attack_ticks = old_ticks;
        }
    }
    // Native callers swing the acting hand exactly once on claimed=true.
    true
}

/// The source speeds up a fast weapon's target hurt timer after a landed hit,
/// before ordinary hurtEnemy and retained tool wear.
pub(super) fn hit(game: &mut Game<'_>, index: usize) {
    let Some(attack) = active(game) else {
        return;
    };
    let Some(mob) = game.mobs.get_mut(index).filter(|m| m.id == attack.target) else {
        return;
    };
    mob.invulnerable = rule::hurt_delay(mob.invulnerable, attack.hurt_delay);
    // The registration uses a flint template; native weapon_hit only awards
    // ITEM_USED for a native weapon component. Never award it twice.
    if game
        .res
        .items()
        .get("tconstruct:dagger")
        .is_some_and(|i| i.weapon_damage.is_none())
    {
        game.award_stat("used", "tconstruct:dagger", 1);
    }
}
