use super::{Game,tconstruct_gold_guard as source};
use horizon_items::items::ItemStack;

fn definition<'a>(game: &'a Game<'_>, id: &str) -> Option<&'a [u8]> {
    let raw=game.world.chunks.__horizon_mixins_tconstruct_modifier_resource_snapshot().as_ref()?.as_ref().ok()?;
    super::tconstruct_modifier_snapshot::highest_definition(raw,id)
}
/// Return the Gold Guard level and the tool's source piglin-neutral flag.
/// Brokenness does not disable volatile flags or EquipmentChange hooks.
fn tool(game: &Game<'_>, stack: &ItemStack) -> Option<(i32,bool)> {
    let tags=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    if !tags.get("tconstruct:modifiable/armor/worn")?.contains(stack.item.as_ref()) {return None;}
    let traits=super::tconstruct_source_material_traits::levels(&game.world,stack)?;
    let guard=super::tconstruct_source_material_traits::level_with_upgrade(stack,&traits,source::ID)?;
    let golden=super::tconstruct_source_material_traits::level_with_upgrade(stack,&traits,"tconstruct:golden")?;
    if guard>0 && !source::known_guard(definition(game,"tconstruct:tinkering/modifiers/gold_guard")) {return None;}
    if golden>0 && !source::known_golden(definition(game,"tconstruct:tinkering/modifiers/golden")) {return None;}
    Some((guard,source::neutral(golden,guard)?))
}
pub(super) fn neutral(game:&Game<'_>, stack:&ItemStack) -> bool {
    tool(game,stack).is_some_and(|(_,neutral)|neutral)
}
pub(super) fn bonus(game:&Game<'_>) -> Option<f64> {
    let native=game.res.items().item_tags.get("minecraft:piglin_safe_armor");
    let mut slots=Vec::new();
    for (index, stack) in game.armor.iter().enumerate().filter_map(|(i,s)|s.as_ref().map(|s|(i,s))) {
        slots.push(if stack.item.starts_with("tconstruct:") {
            let (level,neutral)=tool(game,stack)?;
            // Source EquipmentChangeWatcher requires the armor item's class slot.
            let slot=match stack.item.as_ref() {
                "tconstruct:travelers_helmet"|"tconstruct:plate_helmet"|"tconstruct:slime_helmet"=>Some(0),
                "tconstruct:travelers_chestplate"|"tconstruct:plate_chestplate"|"tconstruct:slimy_chestplate"|"tconstruct:slime_wings"=>Some(1),
                "tconstruct:travelers_leggings"|"tconstruct:plate_leggings"|"tconstruct:slime_leggings"=>Some(2),
                "tconstruct:travelers_boots"|"tconstruct:plate_boots"|"tconstruct:slime_boots"=>Some(3),
                _=>None,
            };
            (if slot==Some(index){level}else{0},neutral)
        }
            else {(0,native.is_some_and(|tag|tag.contains(stack.item.as_ref())))});
    }
    source::bonus(&slots)
}

/// Worn attribute hooks apply only to unbroken armor in its class slot.
/// Only resolved material traits are supported: saved upgrades lack a normal
/// acquisition path here, and must not silently acquire hand health semantics.
fn revitalizing(game:&Game<'_>) -> Option<f64> {
    if !source::known_revitalizing(definition(game,"tconstruct:tinkering/modifiers/revitalizing")) {return None;}
    let tags=game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    let worn=tags.get("tconstruct:modifiable/armor/worn")?;
    let mut total=0.0f64;
    for (index, stack) in game.armor.iter().enumerate().filter_map(|(i,s)|s.as_ref().map(|s|(i,s))) {
        let slot=match stack.item.as_ref() {
            "tconstruct:travelers_helmet"|"tconstruct:plate_helmet"|"tconstruct:slime_helmet"=>Some(0),
            "tconstruct:travelers_chestplate"|"tconstruct:plate_chestplate"|"tconstruct:slimy_chestplate"|"tconstruct:slime_wings"=>Some(1),
            "tconstruct:travelers_leggings"|"tconstruct:plate_leggings"|"tconstruct:slime_leggings"=>Some(2),
            "tconstruct:travelers_boots"|"tconstruct:plate_boots"|"tconstruct:slime_boots"=>Some(3),
            _=>None,
        };
        if slot!=Some(index) || !worn.contains(stack.item.as_ref()) || super::tconstruct_source_ranged_wear::is_broken(stack) {continue;}
        let traits=super::tconstruct_source_material_traits::levels(&game.world,stack)?;
        let level=traits.levels.get(source::REVITALIZING).copied().unwrap_or(0);
        if level<0 || super::tconstruct_source_material_traits::level_with_upgrade(stack,&traits,source::REVITALIZING)?!=level {return None;}
        total+=f64::from(0.0f32+2.0f32*level as f32);
    }
    (total>0.0 && total.is_finite()).then_some(total)
}

pub(super) fn refresh(game: &mut crate::game::Game<'_>) {
    if game.remote.is_some() {return;}
    let wanted=[(source::MODIFIER,bonus(game)),(source::REVITALIZING,revitalizing(game))];
    let Some(player)=game.player.as_mut() else {return;};
    let mut changed=false;
    // Compute both projections first and update before clamping. A same-refresh
    // swap between health traits must not clamp against an intermediate maximum.
    for (id,wanted) in wanted {
        let key=player.attributes.key(source::ATTRIBUTE);
        let Ok(present)=player.attributes.has_modifier(key.clone(),id) else {continue;};
        if !present && wanted.is_none() {continue;}
        if present && wanted.is_some() && player.attributes.modifier_value(key,id).ok()==wanted {continue;}
        if player.set_modifier(source::ATTRIBUTE,id,wanted.unwrap_or(0.0),"add_value",wanted.is_some()).is_ok() {changed=true;}
    }
    // Worn UpdateHealth has bonus zero: never heal, only clamp on change.
    if changed {
        if let Ok(maximum)=player.attribute(source::ATTRIBUTE) {
            let maximum=maximum as f32;
            if player.health.health()>maximum {let _=player.set_health(maximum,false);}
        }
    }
}
