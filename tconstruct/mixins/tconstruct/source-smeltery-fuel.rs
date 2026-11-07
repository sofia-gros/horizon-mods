// Standard lava bucket exchange with a source seared tank. The caller commits
// the tank and inventory together and handles the player's creative mode.
use super::tconstruct_source_melter_state::Tank;

pub(super) fn capacity(name: &str) -> Option<u32> {
    match name {
        "tconstruct:seared_fuel_tank" | "tconstruct:seared_fuel_gauge" | "tconstruct:scorched_fuel_tank" | "tconstruct:scorched_fuel_gauge" => Some(4000),
        "tconstruct:seared_ingot_tank" | "tconstruct:seared_ingot_gauge" | "tconstruct:scorched_ingot_tank" | "tconstruct:scorched_ingot_gauge" => Some(48 * 90),
        _ => None,
    }
}

/// Portable tank items include lanterns, which are not multiblock fuel tanks.
pub(super) fn item_capacity(name: &str) -> Option<u32> {
    capacity(name).or_else(|| match name {
        "tconstruct:seared_lantern" | "tconstruct:scorched_lantern" => Some(50),
        "tconstruct:seared_casting_tank" => Some(4000),
        "tconstruct:seared_fluid_cannon" => Some(2000),
        _ => None,
    })
}

/// Copy a readable native tank payload without normalizing extension fields.
/// This is transport, not a debit or migration: no owned field is changed.
pub(super) fn copy_native_tank(raw: &horizon_world::nbt::Tag, capacity: u32) -> Option<horizon_world::nbt::Tag> {
    let tank = Tank::from_nbt(raw,capacity)?;
    (tank.to_nbt_preserving(raw).as_ref() == Some(raw)).then(|| raw.clone())
}

/// Read the first fluid tank from the machine's saved representation.
/// The casting tank and cannon nest their Tank beneath their inventory state;
/// ordinary portable tanks store their Tank directly.
pub(super) fn gauge_tank(block: &str, saved: &horizon_world::nbt::Tag) -> Option<Tank> {
    let capacity = item_capacity(block)?;
    let tank = match block {
        "tconstruct:seared_casting_tank" | "tconstruct:seared_fluid_cannon" => saved.get("tank")?,
        _ => saved,
    };
    Tank::from_nbt(tank, capacity)
}

/// PotionFluidType names its fluid from the potion, not from the generic fluid.
/// Resolve only known source-era vanilla potion IDs. Unknown/modded IDs retain
/// the existing generic display until their potion registries are available.
pub(super) fn potion_fluid_name(tag: Option<&horizon_world::nbt::Tag>) -> Option<String> {
    use horizon_world::nbt::Tag;
    let id = match tag {
        None => return Some("item.minecraft.potion.effect.empty".into()),
        Some(Tag::Compound(fields)) => match fields.get(&"Potion".into()) {
            None => return Some("item.minecraft.potion.effect.empty".into()),
            Some(Tag::String(id)) => id.to_string_lossy(),
            _ => return None,
        },
        _ => return None,
    };
    if id == "minecraft:empty" || id == "empty" { return Some("item.minecraft.potion.effect.empty".into()); }
    let bare = id.strip_prefix("minecraft:").unwrap_or(&id);
    let base = bare.strip_prefix("long_").or_else(|| bare.strip_prefix("strong_")).unwrap_or(bare);
    if !matches!(base, "water" | "mundane" | "thick" | "awkward" | "night_vision" | "invisibility" |
        "leaping" | "fire_resistance" | "swiftness" | "slowness" | "turtle_master" | "water_breathing" |
        "healing" | "harming" | "poison" | "regeneration" | "strength" | "weakness" | "luck" | "slow_falling") { return None; }
    let potion = horizon_client::pluto_potions::potion(&id)?;
    Some(format!("item.minecraft.potion.effect.{}", potion.name_suffix))
}

/// Return the replacement container after a complete bucket transfer. The
/// caller must perform this on a copy until both inventory and NBT can commit.
pub(super) fn exchange(tank: &mut Tank, held: &str) -> Option<&'static str> {
    match held {
        "minecraft:lava_bucket" if tank.insert("minecraft:lava", 1000) => Some("minecraft:bucket"),
        "minecraft:bucket" if tank.fluid.as_deref() == Some("minecraft:lava") && tank.amount >= 1000 => {
            tank.take(1000)?; Some("minecraft:lava_bucket")
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn potion_fluid_names_use_base_potion_translation() {
        use horizon_world::nbt::Tag;
        let tag = |id: &str| Tag::compound([("Potion", Tag::String(id.into()))]);
        assert_eq!(potion_fluid_name(None).as_deref(), Some("item.minecraft.potion.effect.empty"));
        assert_eq!(potion_fluid_name(Some(&tag("minecraft:swiftness"))).as_deref(), Some("item.minecraft.potion.effect.swiftness"));
        assert_eq!(potion_fluid_name(Some(&tag("swiftness"))).as_deref(), Some("item.minecraft.potion.effect.swiftness"));
        assert_eq!(potion_fluid_name(Some(&tag("minecraft:long_swiftness"))).as_deref(), Some("item.minecraft.potion.effect.swiftness"));
        assert_eq!(potion_fluid_name(Some(&tag("minecraft:strong_healing"))).as_deref(), Some("item.minecraft.potion.effect.healing"));
        assert_eq!(potion_fluid_name(Some(&tag("minecraft:empty"))).as_deref(), Some("item.minecraft.potion.effect.empty"));
        assert_eq!(potion_fluid_name(Some(&tag("minecraft:strong_night_vision"))), None);
        assert_eq!(potion_fluid_name(Some(&tag("minecraft:wind_charged"))), None);
        assert_eq!(potion_fluid_name(Some(&tag("example:swiftness"))), None);
        assert_eq!(potion_fluid_name(Some(&Tag::Int(1))), None);
        assert_eq!(potion_fluid_name(Some(&Tag::compound([("Potion", Tag::Int(1))]))), None);
        let extra = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into())), ("CustomPotionColor", Tag::Int(99))]);
        assert_eq!(potion_fluid_name(Some(&extra)).as_deref(), Some("item.minecraft.potion.effect.swiftness"));
    }
    #[test]
    fn gauge_reads_nested_machine_tanks_without_mutation() {
        use horizon_world::nbt::Tag;
        for (block, capacity) in [("tconstruct:seared_casting_tank", 4000), ("tconstruct:seared_fluid_cannon", 2000)] {
            let mut tank = Tank::new(capacity);
            let empty = Tag::compound([("tank", tank.to_nbt()), ("unrelated", Tag::Int(7))]);
            assert_eq!(gauge_tank(block, &empty), Some(tank.clone()));
            assert!(tank.insert("minecraft:lava", 1000));
            let saved = Tag::compound([("tank", tank.to_nbt()), ("unrelated", Tag::Int(7))]);
            let before = saved.clone();
            assert_eq!(gauge_tank(block, &saved), Some(tank));
            assert_eq!(saved, before);
            assert!(gauge_tank(block, &Tag::compound([("tank", Tag::Int(1))])).is_none());
            assert!(gauge_tank(block, &Tag::compound(Vec::<(&str, Tag)>::new())).is_none());
            let too_full = Tag::compound([("tank", Tag::compound([("fluid", Tag::String("minecraft:water".into())), ("amount", Tag::Int((capacity + 1) as i32))]))]);
            assert!(gauge_tank(block, &too_full).is_none());
            let metadata = Tag::compound([("Potion", Tag::String("minecraft:swiftness".into()))]);
            let mut tagged = Tank::new(capacity);
            assert!(tagged.insert_tagged("tconstruct:potion", Some(&metadata), 250));
            let saved = Tag::compound([("tank", tagged.to_nbt())]);
            assert_eq!(gauge_tank(block, &saved), Some(tagged));
        }
        let mut lantern = Tank::new(50);
        assert!(lantern.insert("minecraft:water", 50));
        assert_eq!(gauge_tank("tconstruct:seared_lantern", &lantern.to_nbt()), Some(lantern));
        assert!(gauge_tank("minecraft:stone", &Tag::Int(1)).is_none());
    }
    #[test]
    fn bucket_exchange_is_all_or_nothing() {
        let mut tank = Tank::new(capacity("tconstruct:seared_fuel_tank").unwrap());
        assert_eq!(exchange(&mut tank, "minecraft:lava_bucket"), Some("minecraft:bucket"));
        assert_eq!(tank.amount, 1000);
        assert_eq!(exchange(&mut tank, "minecraft:bucket"), Some("minecraft:lava_bucket"));
        assert_eq!(tank.amount, 0);
        assert_eq!(exchange(&mut tank, "minecraft:bucket"), None);
        assert!(tank.insert("minecraft:lava", 3500));
        assert_eq!(exchange(&mut tank, "minecraft:lava_bucket"), None);
        assert_eq!(tank.amount, 3500);
    }
    #[test]
    fn source_tank_capacities_are_distinct() {
        assert_eq!(capacity("tconstruct:seared_fuel_gauge"), Some(4000));
        assert_eq!(capacity("tconstruct:seared_ingot_gauge"), Some(4320));
        assert_eq!(capacity("minecraft:glass"), None);
    }
}

/// Heater count changes must not invalidate retained network data or lose
/// item fields through the native save codec.
pub(super) fn heater_stack_save_safe(item: &horizon_items::items::ItemStack) -> bool {
    horizon_client::pluto_stack_storage::encode_item_lossless(item).is_some()
}

/// Consume one solid-fuel item on copies. The caller resolves burn duration,
/// whether the crafting remainder burns for more than three ticks, and the
/// source stack limit. Return the next Heater slot and an optional spill.
pub(super) fn plan_heater_fuel(fuel: &horizon_items::items::ItemStack,
    remainder: Option<horizon_items::items::ItemStack>, accepts_remainder: bool, limit: u32)
    -> Option<(Option<horizon_items::items::ItemStack>, Option<horizon_items::items::ItemStack>)> {
    if !heater_stack_save_safe(fuel) || remainder.as_ref().is_some_and(|item| item.count != 1 || !heater_stack_save_safe(item)) { return None; }
    let mut remaining = fuel.clone(); remaining.count -= 1;
    let mut next = (remaining.count > 0).then_some(remaining);
    let mut spill = None;
    if let Some(remainder) = remainder {
        if accepts_remainder {
            match &mut next {
                None => next = Some(remainder),
                Some(stack) if stack.same_item_same_components(&remainder) && stack.count < limit.min(64) => stack.count += 1,
                _ => spill = Some(remainder),
            }
        } else { spill = Some(remainder); }
    }
    Some((next, spill))
}

#[cfg(test)]
mod heater_tests {
    use super::*;
    use horizon_items::items::ItemStack;
    use horizon_world::nbt::Tag;
    #[test]
    fn heater_debit_preserves_fuel_components_and_original_stack() {
        for count in [1,3] {
            let mut fuel = ItemStack::new("minecraft:coal",count);fuel.damage=2;
            fuel.data_mut().set_other("minecraft:custom_name",Some(Tag::String("Kept".into())));
            let original=fuel.clone();let(next,spill)=plan_heater_fuel(&fuel,None,false,64).unwrap();
            assert_eq!(fuel,original);assert!(spill.is_none());
            if count==1{assert!(next.is_none());}else{let next=next.unwrap();assert_eq!(next.count,2);assert_eq!(next.damage,fuel.damage);assert!(next.same_item_same_components(&fuel));}
        }
    }
    #[test]
    fn heater_remainder_routing_matches_source_slot_rules() {
        for count in [1,2]{for accepted in [false,true]{for same in [false,true]{
            let fuel=ItemStack::new("minecraft:coal",count);let remainder=ItemStack::new(if same{"minecraft:coal"}else{"minecraft:bucket"},1);
            let(next,spill)=plan_heater_fuel(&fuel,Some(remainder.clone()),accepted,64).unwrap();
            if accepted && count==1{assert_eq!(next,Some(remainder));assert!(spill.is_none());}
            else if accepted && same{assert_eq!(next.unwrap().count,count);assert!(spill.is_none());}
            else{assert_eq!(next.as_ref().map(|item|item.count),(count>1).then_some(count-1));assert_eq!(spill,Some(remainder));}
        }}}
        let fuel=ItemStack::new("minecraft:coal",2);let(next,spill)=plan_heater_fuel(&fuel,Some(ItemStack::new("minecraft:coal",1)),true,1).unwrap();
        assert_eq!(next.unwrap().count,1);assert_eq!(spill.unwrap().count,1);
    }
    #[test]
    fn heater_debit_rejects_zero_and_retained_wire_without_a_plan() {
        for stale in [false,true]{for target in [false,true]{
            let mut fuel=ItemStack::new("minecraft:coal",2);let mut remainder=ItemStack::new("minecraft:bucket",1);
            let item=if target{&mut remainder}else{&mut fuel};
            item.data_mut().wire=horizon_items::item_data::Wire::new(item.clone(),std::sync::Arc::<[u8]>::from([1,2,3]));
            if stale{item.data_mut().set_other("minecraft:custom_name",Some(Tag::String("Changed".into())));}
            let before=(fuel.clone(),remainder.clone());assert!(plan_heater_fuel(&fuel,Some(remainder.clone()),true,64).is_none());assert_eq!((fuel,remainder),before);
        }}
        assert!(plan_heater_fuel(&ItemStack::new("minecraft:coal",0),None,false,64).is_none());
        assert!(plan_heater_fuel(&ItemStack::new("minecraft:coal",2),Some(ItemStack::new("minecraft:bucket",2)),false,64).is_none());
    }
}

/// GUI-only snapshot of the source fuel module's selected fluid tanks.
#[derive(Clone, Debug, Default)]
pub(super) struct FuelView {
    pub last: Option<(i32, i32, i32)>,
    pub has_tank: bool,
    pub fluid: Option<String>,
    pub amount: u32,
    pub capacity: u32,
    pub temperature: u32,
}
impl FuelView {
    pub(super) fn collect(tanks: &[((i32, i32, i32), Tank)], last: Option<(i32, i32, i32)>, temperature: impl Fn(&str) -> u32) -> Self {
        let primary = last.and_then(|pos| tanks.iter().find(|(at, tank)| *at == pos && tank.fluid.is_some()))
            .or_else(|| tanks.iter().find(|(_, tank)| tank.fluid.is_some()));
        let Some((pos, tank)) = primary else { return Self { has_tank: !tanks.is_empty(), ..Self::default() }; };
        let Some(fluid) = tank.fluid.clone() else { return Self::default(); };
        let mut view = Self { last: Some(*pos), has_tank: true, temperature: temperature(&fluid), fluid: Some(fluid.clone()), amount: 0, capacity: 0 };
        for (_, tank) in tanks {
            if tank.fluid.is_none() || tank.fluid.as_deref() == Some(fluid.as_str()) {
                view.amount = view.amount.saturating_add(tank.amount);
                view.capacity = view.capacity.saturating_add(tank.capacity);
            }
        }
        view
    }
    pub(super) fn to_nbt(&self) -> horizon_world::nbt::Tag {
        use horizon_world::nbt::Tag;
        let mut fields = horizon_world::nbt::Compound::new();
        fields.insert("has_tank".into(), Tag::Byte(i8::from(self.has_tank)));
        if let Some((x,y,z)) = self.last { fields.insert("last".into(), Tag::IntArray(vec![x,y,z])); }
        if let Some(id) = &self.fluid { fields.insert("fluid".into(), Tag::String(id.as_str().into())); }
        for (name, value) in [("amount", self.amount), ("capacity", self.capacity), ("temperature", self.temperature)] {
            fields.insert(name.into(), Tag::Long(i64::from(value)));
        }
        Tag::Compound(fields)
    }
    pub(super) fn from_nbt(tag: &horizon_world::nbt::Tag) -> Option<Self> {
        use horizon_world::nbt::Tag;
        let number = |name| match tag.get(name)? { Tag::Long(value) => u32::try_from(*value).ok(), _ => None };
        let last = match tag.get("last") { Some(Tag::IntArray(v)) if v.len() == 3 => Some((v[0],v[1],v[2])), _ => None };
        let fluid = match tag.get("fluid") { Some(Tag::String(id)) => Some(id.to_string_lossy()), _ => None };
        Some(Self { last, has_tank: matches!(tag.get("has_tank"), Some(Tag::Byte(1))), fluid,
            amount: number("amount")?, capacity: number("capacity")?, temperature: number("temperature")? })
    }
}

#[cfg(test)]
mod view_tests {
    use super::*;
    #[test] fn source_view_prefers_last_nonempty_and_combines_same_or_empty_tanks() {
        let mut a = Tank::new(4000); a.insert("minecraft:lava", 1000);
        let mut b = Tank::new(4000); b.insert("minecraft:water", 1000);
        let empty = Tank::new(4320);
        let tanks = vec![((0,0,0),a), ((1,0,0),b), ((2,0,0),empty)];
        let view = FuelView::collect(&tanks, Some((1,0,0)), |_| 500);
        assert_eq!((view.fluid.as_deref(), view.amount, view.capacity, view.temperature), (Some("minecraft:water"), 1000, 8320, 500));
        let restored = FuelView::from_nbt(&view.to_nbt()).unwrap();
        assert_eq!((restored.last, restored.fluid, restored.capacity), (Some((1,0,0)), Some("minecraft:water".into()), 8320));
    }
}
