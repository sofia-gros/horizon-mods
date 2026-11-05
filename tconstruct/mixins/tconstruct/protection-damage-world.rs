use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;

#[derive(Clone,Copy)]
struct Rule {
    id: &'static str,
    source_tag: &'static str,
    source_each: f32,
    direct_only: bool,
    vanilla: Option<(&'static str,f32)>,
}
const RULES: [Rule;5] = [
    Rule { id:"tconstruct:fire_protection", source_tag:"tconstruct:protection/fire", source_each:3.0, direct_only:false, vanilla:Some(("minecraft:is_fire",2.0)) },
    Rule { id:"tconstruct:blast_protection", source_tag:"tconstruct:protection/blast", source_each:3.0, direct_only:false, vanilla:Some(("minecraft:is_explosion",2.0)) },
    Rule { id:"tconstruct:magic_protection", source_tag:"tconstruct:protection/magic", source_each:3.0, direct_only:false, vanilla:None },
    Rule { id:"tconstruct:projectile_protection", source_tag:"tconstruct:protection/projectile", source_each:2.0, direct_only:false, vanilla:None },
    Rule { id:"tconstruct:melee_protection", source_tag:"tconstruct:protection/melee", source_each:2.0, direct_only:true, vanilla:None },
];

fn piece(game: &super::Game<'_>, stack: &ItemStack, worn: bool, id: &str) -> Option<(f32,bool,bool)> {
    if stack.count != 1 || stack.wire_patch().is_some() { return None; }
    let custom = stack.data()?.other("minecraft:custom_data")?;
    let maximum = match custom.get("tic_max_damage") { Some(Tag::Int(n)) if *n > 0 => *n as u32, _ => return None };
    let flag = match custom.get("tic_broken") { None | Some(Tag::Byte(0)) => false, Some(Tag::Byte(1)) => true, _ => return None };
    let raw = match custom.get("tic_upgrades") {
        None => 0,
        Some(upgrades @ Tag::Compound(_)) => match upgrades.get(id) { None => 0, Some(Tag::Int(raw)) => *raw, _ => return None },
        _ => return None,
    };
    let traits = super::tconstruct_source_material_traits::levels(&game.world,stack)?;
    let level = super::tconstruct_source_protection::effective_level(traits.levels.get(id).copied().unwrap_or(0),raw)?;
    Some((level, flag || stack.damage >= maximum, worn))
}

type Table = Vec<(String,f32)>;

fn wanted(game: &super::Game<'_>) -> Option<(Table,Table)> {
    let damage_types = game.world.chunks.__horizon_mixins_tconstruct_protection_damage_types().as_ref()?.as_ref().ok()?;
    let tags = game.world.chunks.__horizon_mixins_tconstruct_resolved_item_tags().as_ref()?.as_ref().ok()?;
    let belongs = |stack:&ItemStack, tag:&str| tags.get(tag).is_some_and(|set|set.contains(stack.item.as_ref()));
    let mut equipment = Vec::new();
    // Source EquipmentSlot order: hands first, then feet through head.
    for stack in [game.inventory.get(game.selected).and_then(Option::as_ref), game.offhand.as_ref()].into_iter().flatten() {
        if matches!(stack.item.as_ref(), "tconstruct:travelers_shield" | "tconstruct:plate_shield" | "tconstruct:battlesign")
            && belongs(stack,"tconstruct:modifiable/held") { equipment.push((stack,false)); }
    }
    for (slot, stack) in game.armor.iter().enumerate().rev().filter_map(|(slot,s)|s.as_ref().map(|s|(slot,s))) {
        if super::tconstruct_fire_duration::armor_slot(&stack.item) == Some(slot)
            && belongs(stack,"tconstruct:modifiable/armor/worn") { equipment.push((stack,true)); }
    }
    let mut table = std::collections::BTreeMap::new();
    let mut direct_table = std::collections::BTreeMap::new();
    let empty = std::collections::HashSet::new();
    for rule in RULES {
        if !super::tconstruct_source_protection::supported_definition(&game.world,rule.id) { continue; }
        let Some(source_types) = damage_types.get(rule.source_tag) else { continue; };
        let (vanilla_types, vanilla_each) = match rule.vanilla {
            Some((tag,each)) => { let Some(types) = damage_types.get(tag) else { continue; }; (types,each) },
            None => (&empty,0.0),
        };
        let Some(pieces) = equipment.iter().map(|(stack,worn)|piece(game,stack,*worn,rule.id)).collect::<Option<Vec<_>>>() else { continue; };
        let Some(points) = super::tconstruct_protection_damage::points(&pieces,rule.source_each,vanilla_each) else { continue; };
        let target = if rule.direct_only { &mut direct_table } else { &mut table };
        super::tconstruct_protection_damage::add_points(target,source_types,vanilla_types,points);
    }
    Some((table.into_iter().collect(),direct_table.into_iter().collect()))
}

pub(super) fn refresh(game: &mut super::Game<'_>) {
    let (table,direct_table) = if game.remote.is_none() { wanted(game).unwrap_or_default() } else { Default::default() };
    if let Some(player) = game.player.as_mut() {
        *player.status.__horizon_mixins_tconstruct_protection_points_mut() = table;
        *player.status.__horizon_mixins_tconstruct_melee_protection_points_mut() = direct_table;
    }
}
