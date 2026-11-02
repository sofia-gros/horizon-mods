fn tconstruct_chest_tint(value: (), out: &mut SectionMesh, tints: &mut Tints<'_>, pos: [i32;3], face: &BakedFace) {
    use horizon_world::nbt::Tag;
    if face.quad.tint_index != 0 { return value; }
    let [cx,cz,base_y]=tints.origin;
    let at=(cx*16+pos[0],base_y+pos[1],cz*16+pos[2]);
    let Some(chunk)=tints.region[1][1] else {return value;};
    let Some(raw)=chunk.saved.as_deref().and_then(|saved|saved.block_entity_sources.iter().find(|(p,_)|*p==at)).map(|(_,tag)|tag) else {return value;};
    if !matches!(raw.get("id"),Some(Tag::String(id)) if id.to_string_lossy()=="tconstruct:modifier_chest") {return value;}
    let color=match raw.get("tconstruct:modifier_chest").and_then(|v|v.get("color")).or_else(||raw.get("color")) {Some(Tag::Int(color))=>*color as u32,_=>0x407686};
    let target=if face.translucent {&mut out.translucent} else {&mut out.solid};
    let Some(first)=target.len().checked_sub(4) else {return value;};
    for vertex in &mut target[first..] {
        vertex.color=crate::mesher::tconstruct_chest_tint_math::apply(vertex.color,color);
    }
    value
}
