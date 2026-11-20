//! MaterialVariantId, MaterialNBT and material-part state from SlimeKnights (MIT; see LICENSE).
//! Original item NBT lives inside the host's custom_data component; other stack data is retained.
use crate::materials::Definitions;
use pluto_lib::{Identifier, ItemSlot, Nbt, NbtCompound};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MaterialVariant {
    // Preserve Java's valid empty resource path in saved data. Such IDs cannot
    // address a PlutoLib registry entry, but must not be silently rewritten on save.
    material: String,
    variant: String,
}
impl MaterialVariant {
    pub fn parse(text: &str) -> Option<Self> {
        let (base, variant) = text.split_once('#').unwrap_or((text, ""));
        let valid_path = |value: &str| value.bytes().all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'/' | b'-'));
        if !valid_path(variant) { return None; }
        let (namespace, path) = base.split_once(':').unwrap_or(("minecraft", base));
        let namespace = if namespace.is_empty() { "minecraft" } else { namespace };
        if !namespace.bytes().all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'-')) || !valid_path(path) { return None; }
        Some(Self { material: format!("{namespace}:{path}"), variant: variant.into() })
    }
    pub fn unknown() -> Self { Self::parse("tconstruct:unknown").unwrap() }
    pub fn material_id(&self) -> Option<Identifier> { Identifier::parse(&self.material).ok() }
    /// Recipe matching is directional: no variant matches all, #default only empty/default.
    pub fn matches(&self, other: &Self) -> bool {
        self.material == other.material && (self.variant.is_empty()
            || self.variant == other.variant || self.variant == "default" && other.variant.is_empty())
    }
    pub fn normalized(&self) -> Self {
        let mut value = self.clone();
        if value.variant == "default" { value.variant.clear(); }
        value
    }
    /// Explicit migration only. Reading a part does not implicitly resolve aliases.
    pub fn resolved(&self, registry: &Definitions) -> Self {
        let mut value = self.clone();
        if let Some(id) = self.material_id() {
            value.material = registry.resolve_redirect(&id).to_string();
        }
        value
    }
}
impl fmt::Display for MaterialVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.material)?;
        if !self.variant.is_empty() { write!(f, "#{}", self.variant)?; }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MaterialList(Vec<MaterialVariant>);
impl MaterialList {
    pub fn from_nbt(value: Option<&Nbt>) -> Self {
        let Some(Nbt::List(list)) = value else { return Self::default(); };
        // Minecraft ListTag is homogeneous; a list with the wrong type is empty,
        // whereas a malformed string occupies its original position as UNKNOWN.
        if !list.iter().all(|value| matches!(value, Nbt::String(_))) { return Self::default(); }
        Self(list.iter().map(|value| MaterialVariant::parse(value.as_str().unwrap())
            .unwrap_or_else(MaterialVariant::unknown)).collect())
    }
    pub fn to_nbt(&self) -> Nbt { Nbt::List(self.0.iter().map(|value| Nbt::String(value.to_string())).collect()) }
    pub fn get(&self, index: i32) -> MaterialVariant {
        usize::try_from(index).ok().and_then(|index| self.0.get(index)).cloned().unwrap_or_else(MaterialVariant::unknown)
    }
    pub fn from_variants(variants: Vec<MaterialVariant>) -> Self { Self(variants) }
    pub fn as_slice(&self) -> &[MaterialVariant] { &self.0 }
    pub fn replaced(&self, index: i32, replacement: MaterialVariant) -> Result<Self, String> {
        let index = usize::try_from(index).map_err(|_| "Material index is out of bounds")?;
        let mut result = self.clone();
        if index >= result.0.len() {
            result.0.try_reserve(index + 1 - result.0.len()).map_err(|_| "Material list allocation failed")?;
            result.0.resize_with(index + 1, MaterialVariant::unknown);
        }
        result.0[index] = replacement;
        Ok(result)
    }
}

pub(crate) fn part_material(data: &NbtCompound) -> MaterialVariant {
    data.get("Material").and_then(Nbt::as_str).filter(|text| !text.is_empty())
        .and_then(MaterialVariant::parse).unwrap_or_else(MaterialVariant::unknown)
}
pub(crate) fn set_part_material(stack: &mut ItemSlot, material: &MaterialVariant) {
    stack.custom_data.insert("Material".into(), Nbt::String(material.to_string()));
    sync_model(stack);
}

/// The sprite key of one material slot for the item definitions' `select` on `custom_model_data`: the material id path without
/// its namespace (`wood`, `rose_gold`); empty for an unknown material, which has no sprite of its own.
fn model_key(material: &MaterialVariant) -> String {
    let unknown = MaterialVariant::unknown().material_id();
    match material.material_id() {
        Some(id) if Some(&id) != unknown.as_ref() => id.path().to_owned(),
        _ => String::new(),
    }
}
/// What a stack's `custom_model_data.strings` hold, from its `custom_data`: for a tool, one `model_key` per `tic_materials` entry in
/// slot order; for a tool part, its `Material` alone. None for data with neither (an empty material list included).
pub(crate) fn model_strings(data: &NbtCompound) -> Option<Vec<String>> {
    if let Some(Nbt::List(_)) = data.get(crate::tool_stack::TAG_MATERIALS) {
        let list = MaterialList::from_nbt(data.get(crate::tool_stack::TAG_MATERIALS));
        return Some(list.as_slice().iter().map(model_key).collect()).filter(|strings: &Vec<String>| !strings.is_empty());
    }
    data.get("Material").and_then(Nbt::as_str).filter(|text| !text.is_empty()).map(|text| vec![MaterialVariant::parse(text).map(|m| model_key(&m)).unwrap_or_default()])
}
/// The overlay flags of a tool (`model_flags::OVERLAY_BASE + i` for `OVERLAYS[i]`): which of the modifiers the tool models draw a sprite for are among its
/// crafted modifiers (`tic_upgrades`, the original's `getUpgrades`). The flags before `OVERLAY_BASE` are use-time facts written elsewhere and are kept.
fn sync_overlay_flags(flags: &mut Vec<bool>, data: &NbtCompound) {
    use crate::model_flags::{OVERLAYS, OVERLAY_BASE};
    flags.truncate(OVERLAY_BASE);
    let lit: Vec<usize> = crate::tool_stack::upgrades(data).entries().iter().filter(|entry| entry.level > 0)
        .filter_map(|entry| OVERLAYS.iter().position(|id| *id == entry.id.to_string())).collect();
    if let Some(last) = lit.iter().max() {
        flags.resize(OVERLAY_BASE + last + 1, false);
        for position in lit { flags[OVERLAY_BASE + position] = true; }
    }
    while flags.last() == Some(&false) { flags.pop(); }
}
/// Keeps the stack's `custom_model_data` in step with its tool data; every write of `tic_materials`, `tic_upgrades` or `Material` on a stack
/// goes through here (directly or through `tool_slot::sync_visuals`). The game cannot look inside `custom_data`, so item models select on this:
/// `strings` hold the sprite key of each material slot, `flags` the modifier overlays. Other lists and flags the stack's model data carries are kept.
pub(crate) fn sync_model(stack: &mut ItemSlot) {
    let mut model = stack.custom_model_data.take().unwrap_or_default();
    model.strings = model_strings(&stack.custom_data).unwrap_or_default();
    sync_overlay_flags(&mut model.flags, &stack.custom_data);
    stack.custom_model_data = Some(model).filter(|model| !model.is_empty());
}
pub(crate) fn verify_part_material(stack: &mut ItemSlot, registry: &Definitions) {
    let original = part_material(&stack.custom_data);
    if original == MaterialVariant::unknown() { return; }
    let resolved = original.resolved(registry);
    if resolved != original { set_part_material(stack, &resolved); }
}

/// ToolPartItem's item/stat binding. Actual item registration and visuals are separate.
#[derive(Clone, Debug)]
pub(crate) struct ToolPart { pub item: Identifier, pub stat: Identifier }
impl ToolPart {
    pub fn can_use(&self, material: &MaterialVariant, registry: &Definitions) -> bool {
        material.material_id().is_some_and(|id| registry.get_stats(&id, &self.stat).is_some())
    }
    pub fn set_material(&self, stack: &mut ItemSlot, material: &MaterialVariant, registry: &Definitions) -> bool {
        if !self.can_use(material, registry) { return false; }
        set_part_material(stack, material);
        true
    }
    pub fn variants(&self, registry: &Definitions, filter: &str) -> Vec<ItemSlot> {
        if !registry.server_loaded { return Vec::new(); }
        let stack = |material: &MaterialVariant| {
            let mut stack = ItemSlot::new(self.item.clone(), 1);
            set_part_material(&mut stack, material);
            stack
        };
        if !filter.is_empty() {
            if let Some(material) = MaterialVariant::parse(filter).filter(|material| self.can_use(material, registry)) {
                return vec![stack(&material)];
            }
        }
        let mut result = Vec::new();
        for id in &registry.visible {
            let material = MaterialVariant::parse(&id.to_string()).unwrap();
            if self.can_use(&material, registry) {
                result.push(stack(&material));
                if !filter.is_empty() { break; }
            }
        }
        result
    }
    pub fn client_variants(&self, filter: &str) -> Vec<ItemSlot> {
        crate::common::with_client_materials(|registry| registry.map(|registry| self.variants(registry, filter)).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn variant(text: &str) -> MaterialVariant { MaterialVariant::parse(text).unwrap() }
    #[test]
    fn variant_matching_is_directional_and_preserves_default_spelling() {
        let base = variant("wood");
        let default = variant(":wood#default");
        let patterned = variant("minecraft:wood#grain/dark");
        assert!(base.matches(&patterned));
        assert!(!patterned.matches(&base));
        assert!(default.matches(&base));
        assert!(!default.matches(&patterned));
        assert_ne!(base, default);
        assert_eq!(default.normalized(), base);
        assert_eq!(variant("wood#"), base);
        assert_eq!(variant("#grain").to_string(), "minecraft:#grain");
        for invalid in ["Wood", "wood#a#b", "wood#UPPER", "a:b:c", "wood# "] { assert!(MaterialVariant::parse(invalid).is_none()); }
    }
    #[test]
    fn material_lists_keep_invalid_string_positions_and_pad_replacements() {
        let list = MaterialList::from_nbt(Some(&Nbt::List(vec![Nbt::String("test:wood#default".into()), Nbt::String("Bad".into())])));
        assert_eq!(list.get(1), MaterialVariant::unknown());
        let extended = list.replaced(3, variant("test:iron")).unwrap();
        assert_eq!(extended.get(2), MaterialVariant::unknown());
        assert_eq!(list.as_slice().len(), 2);
        assert_eq!(MaterialList::from_nbt(Some(&extended.to_nbt())), extended);
        assert_eq!(extended.get(-1), MaterialVariant::unknown());
        assert!(list.replaced(-1, variant("test:iron")).is_err());
        assert!(MaterialList::from_nbt(Some(&Nbt::List(vec![Nbt::Int(1)]))).as_slice().is_empty());
        assert!(MaterialList::from_nbt(Some(&Nbt::String("test:wood".into()))).as_slice().is_empty());
    }
    #[test]
    fn part_migration_is_explicit_and_keeps_unrelated_stack_state() {
        let mut registry = Definitions::default();
        registry.redirects.insert(Identifier::parse("test:old").unwrap(), Identifier::parse("test:new").unwrap());
        let mut stack = ItemSlot::new(Identifier::parse("tconstruct:pick_head").unwrap(), 3);
        stack.damage = 7;
        stack.custom_data.insert("other".into(), Nbt::Long(42));
        set_part_material(&mut stack, &variant("test:old#grain"));
        assert_eq!(part_material(&stack.custom_data), variant("test:old#grain"));
        verify_part_material(&mut stack, &registry);
        assert_eq!(part_material(&stack.custom_data), variant("test:new#grain"));
        assert_eq!((stack.count, stack.damage), (3, 7));
        assert_eq!(stack.custom_data["other"], Nbt::Long(42));
        stack.custom_data.insert("Material".into(), Nbt::Int(5));
        verify_part_material(&mut stack, &registry);
        assert_eq!(stack.custom_data["Material"], Nbt::Int(5), "invalid/unknown tags are not rewritten");
    }
    #[test]
    fn crafted_modifiers_with_a_sprite_light_their_overlay_flag_and_keep_use_flags() {
        use crate::model_flags::{BLOCKING, OVERLAYS, OVERLAY_BASE};
        let upgrades = crate::modifier_entries::List::default()
            .with_modifier(Identifier::parse("tconstruct:diamond").unwrap(), 1).unwrap()
            .with_modifier(Identifier::parse("tconstruct:no_sprite_for_this").unwrap(), 2).unwrap()
            .with_modifier(Identifier::parse("tconstruct:haste").unwrap(), 3).unwrap();
        let mut stack = ItemSlot::new(Identifier::parse("tconstruct:pickaxe").unwrap(), 1);
        stack.custom_data.insert(crate::tool_stack::TAG_UPGRADES.into(), upgrades.to_nbt());
        stack.custom_model_data.get_or_insert_with(Default::default).flags = vec![true];
        sync_model(&mut stack);
        let flags = stack.custom_model_data.clone().unwrap().flags;
        let position = |name: &str| OVERLAY_BASE + OVERLAYS.iter().position(|id| *id == name).unwrap();
        assert!(flags[BLOCKING], "the use-time flags stay");
        assert_eq!(flags.iter().enumerate().filter(|(_, lit)| **lit).map(|(index, _)| index).collect::<Vec<_>>(),
                   vec![BLOCKING, position("tconstruct:diamond"), position("tconstruct:haste")]);
        stack.custom_data.remove(crate::tool_stack::TAG_UPGRADES);
        sync_model(&mut stack);
        assert_eq!(stack.custom_model_data.unwrap().flags, vec![true], "a removed modifier clears its flag and the trailing falses are dropped");
    }
}
