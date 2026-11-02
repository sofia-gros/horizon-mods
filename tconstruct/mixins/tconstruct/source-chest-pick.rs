fn tconstruct_chest_pick(mut value: Option<ItemStack>,game: &crate::game::Game<'_>,pos: [i32;3]) -> Option<ItemStack> {
    use horizon_world::nbt::Tag;
    let Some(stack)=value.as_mut() else {return value;};
    if &*stack.item!="tconstruct:tinkers_chest" {return value;}
    if let Some(Tag::Int(color))=crate::game::tconstruct_source_melter_store::read(&game.world,(pos[0],pos[1],pos[2]),"tconstruct:modifier_chest").and_then(|value|value.get("color").cloned()) {stack.data_mut().dyed_color=Some(color);}
    value
}
