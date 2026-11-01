use super::ItemStack;
use horizon_world::nbt::Tag;

// Kama's source shears trait is unconditional. Scythe's silky_shears module
// requires the separate Silky modifier, whose acquisition remains unfinished.
pub(super) fn capability(stack:&ItemStack)->Option<bool> {
    let item=stack.item.as_ref();
    if !matches!(item,"tconstruct:kama"|"tconstruct:scythe") {return None;}
    let custom=stack.data().and_then(|data|data.other("minecraft:custom_data"));
    let max=custom.and_then(|tag|tag.get("tic_max_damage")).and_then(|tag|match tag {Tag::Int(n) if *n>0=>Some(*n as u32),_=>None});
    if max.is_none_or(|max|stack.damage>=max) || matches!(custom.and_then(|tag|tag.get("tic_broken")),Some(Tag::Byte(value)) if *value!=0) {return Some(false);}
    if item=="tconstruct:kama" {return Some(true);}
    let silky=custom.and_then(|tag|tag.get("tic_modifiers")).and_then(|tag|match tag {Tag::List(entries)=>Some(entries),_=>None}).is_some_and(|entries|entries.iter().any(|entry|
        matches!(entry.get("name"),Some(Tag::String(name)) if name.to_string_lossy()=="tconstruct:silky") &&
        matches!(entry.get("level"),Some(Tag::Int(level)) if *level>0)));
    Some(silky)
}
