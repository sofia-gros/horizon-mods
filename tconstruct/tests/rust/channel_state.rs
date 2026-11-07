include!("pluto_storage_fixture.rs");
#[path = "../../mixins/tconstruct/source-melter-state.rs"] mod tconstruct_source_melter_state;
#[path = "../../mixins/tconstruct/source-channel-plan.rs"] mod tconstruct_source_channel_plan;
#[path = "../../mixins/tconstruct/source-channel-state.rs"] mod tconstruct_source_channel_state;
use horizon_world::nbt::Tag;
use tconstruct_source_channel_state::ChannelState;

#[test] fn native_roundtrip_and_lock() {
 let mut s=ChannelState::new(); assert!(s.tank.insert("test:fluid",25)); s.locked=12;
 let recovered=ChannelState::from_nbt(&s.to_nbt().unwrap()).unwrap(); assert_eq!(recovered,s);
}
#[test] fn patch_retains_outer_and_nested_extensions_and_fluid_metadata() {
 let mut s=ChannelState::new(); let meta=Tag::compound([("potion",Tag::String("test:p".into()))]);
 assert!(s.tank.insert_tagged("test:fluid",Some(&meta),25)); s.locked=10;
 let Tag::Compound(mut root)=s.to_nbt().unwrap() else { panic!() };
 let Some(Tag::Compound(tank))=root.get_mut(&"tank".into()) else { panic!() };
 tank.insert("extension".into(),Tag::Int(7)); root.insert("outside".into(),Tag::String("keep".into()));
 root.insert("is_flowing".into(),Tag::ByteArray(vec![1,2,0,0,0]));
 let before=Tag::Compound(root); let mut next=ChannelState::from_nbt(&before).unwrap();
 assert!(next.tank.take_tagged(Some(&meta),5).is_some()); next.locked=0;
 let after=next.to_nbt_preserving(&before).unwrap();
 assert_eq!(after.get("outside"),before.get("outside")); assert_eq!(after.get("is_flowing"),before.get("is_flowing"));
 assert_eq!(after.get("tank").unwrap().get("extension"),Some(&Tag::Int(7)));
 assert_eq!(ChannelState::from_nbt(&after).unwrap(),next);
}
#[test] fn malformed_and_legacy_mixed_formats_fail_closed() {
 let s=ChannelState::new(); let Tag::Compound(mut root)=s.to_nbt().unwrap() else {panic!()};
 let Some(Tag::Compound(tank))=root.get_mut(&"tank".into()) else {panic!()}; tank.insert("FluidName".into(),Tag::String("test:fluid".into()));
 let raw=Tag::Compound(root); assert!(ChannelState::from_nbt(&raw).is_none()); assert!(s.to_nbt_preserving(&raw).is_none());
 let invalid=Tag::compound([("tank",Tag::compound([("amount",Tag::Int(0)),("locked",Tag::Int(1))]))]);
 assert!(ChannelState::from_nbt(&invalid).is_none()); assert!(s.to_nbt_preserving(&invalid).is_none());
}
#[test] fn invalid_new_states_are_not_written() {
 let good=ChannelState::new(); let raw=good.to_nbt().unwrap(); let mut bad=good.clone(); bad.locked=1;
 assert!(bad.to_nbt().is_none()); assert!(bad.to_nbt_preserving(&raw).is_none()); bad.locked=0; bad.tank.capacity=41;
 assert!(bad.to_nbt_preserving(&raw).is_none());
}

#[test] fn non_utf16_fluid_identifier_is_never_rewritten() {
 let raw=Tag::compound([("tank", Tag::compound([
  ("fluid",Tag::String(horizon_world::nbt::JavaString(vec![0xd800]))),
  ("amount",Tag::Int(1)), ("locked",Tag::Int(0))]))]);
 assert!(ChannelState::from_nbt(&raw).is_none());
 assert!(ChannelState::new().to_nbt_preserving(&raw).is_none());
}
