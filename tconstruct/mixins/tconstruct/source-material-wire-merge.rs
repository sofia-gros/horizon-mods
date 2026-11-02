use horizon_protocol::chunk::BlockEntityInfo;
use horizon_world::nbt::{Compound, Tag};
use std::collections::HashMap;

/// Add only our material field to matching entries, preserving existing NBT.
pub(super) fn merge(entities: &mut [BlockEntityInfo], chunk: (i32, i32), kind: i32, materials: &HashMap<(i32, i32, i32), Tag>) {
    for entity in entities {
        if entity.kind != kind { continue; }
        let pos = (chunk.0 * 16 + i32::from(entity.packed_xz as u8 >> 4), i32::from(entity.y), chunk.1 * 16 + i32::from(entity.packed_xz as u8 & 15));
        let Some(material) = materials.get(&pos) else { continue; };
        if matches!(entity.tag, Tag::End) { entity.tag = Tag::Compound(Compound::new()); }
        if let Tag::Compound(fields) = &mut entity.tag { fields.insert("Material".into(), material.clone()); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_protocol::buf::{Codec, List, Reader, Writer};
    #[test]
    fn codec_preserves_existing_and_unrelated_entities() {
        let mut entities = vec![
            BlockEntityInfo { packed_xz: 0x12, y: 64, kind: 7, tag: Tag::compound([("existing", Tag::Int(42))]) },
            BlockEntityInfo { packed_xz: 0x34, y: 64, kind: 8, tag: Tag::compound([("other", Tag::Int(9))]) },
            BlockEntityInfo { packed_xz: 0x56, y: 64, kind: 7, tag: Tag::End },
            BlockEntityInfo { packed_xz: 0x78, y: 64, kind: 7, tag: Tag::Int(123) },
        ];
        let materials = HashMap::from([
            ((17, 64, 34), Tag::String("tconstruct:copper".into())),
            ((21, 64, 38), Tag::String("tconstruct:iron".into())),
            ((19, 64, 36), Tag::String("tconstruct:wrong_kind".into())),
            ((23, 64, 40), Tag::String("tconstruct:noncompound".into())),
        ]);
        merge(&mut entities, (1, 2), 7, &materials);
        let mut writer = Writer::new(); List::<BlockEntityInfo>::write(&entities, &mut writer);
        let decoded = List::<BlockEntityInfo>::read(&mut Reader::new(&writer.data)).unwrap();
        assert_eq!(decoded[0].tag.get("existing"), Some(&Tag::Int(42)));
        assert_eq!(decoded[0].tag.get("Material"), Some(&Tag::String("tconstruct:copper".into())));
        assert_eq!(decoded[1].tag.get("other"), Some(&Tag::Int(9)));
        assert!(decoded[1].tag.get("Material").is_none());
        assert_eq!(decoded[2].tag.get("Material"), Some(&Tag::String("tconstruct:iron".into())));
        assert_eq!(decoded[3].tag, Tag::Int(123));
        let once = writer.data.clone();
        merge(&mut entities, (1, 2), 7, &materials);
        let mut again = Writer::new(); List::<BlockEntityInfo>::write(&entities, &mut again);
        assert_eq!(again.data, once);
    }
}
