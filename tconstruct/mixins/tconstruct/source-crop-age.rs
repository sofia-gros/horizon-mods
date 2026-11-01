use horizon_world::registry::{BlockRegistry, BlockStateId};

// Probe the registry for the next age and reset only the age property.
pub(super) fn mature_reset(registry: &BlockRegistry, state: BlockStateId) -> Option<BlockStateId> {
    let mut crop = registry.state(state)?.clone();
    let age: u32 = crop.properties.get("age")?.parse().ok()?;
    crop.properties.insert("age".into(), age.checked_add(1)?.to_string());
    if registry.id(&crop).is_some() { return None; }
    let default = registry.state(registry.default_state(&crop.name)?)?;
    crop.properties.insert("age".into(), default.properties.get("age")?.clone());
    registry.id(&crop)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maturity_uses_registered_age_and_preserves_other_properties() {
        let json = br#"{"defaults":{"test:crop":0,"test:stone":4},"states":[
          ["test:crop",false,false,"x",{"id":"test:crop","properties":{"age":"0","side":"a"}}],
          ["test:crop",false,false,"x",{"id":"test:crop","properties":{"age":"1","side":"a"}}],
          ["test:crop",false,false,"x",{"id":"test:crop","properties":{"age":"0","side":"b"}}],
          ["test:crop",false,false,"x",{"id":"test:crop","properties":{"age":"1","side":"b"}}],
          ["test:stone",false,false,"x","test:stone"]]}"#;
        let registry=BlockRegistry::from_facts_json(&json[..]).unwrap();
        assert_eq!(mature_reset(&registry,BlockStateId(0)),None);
        assert_eq!(mature_reset(&registry,BlockStateId(1)),Some(BlockStateId(0)));
        assert_eq!(mature_reset(&registry,BlockStateId(3)),Some(BlockStateId(2)));
        assert_eq!(mature_reset(&registry,BlockStateId(4)),None);
    }
}
