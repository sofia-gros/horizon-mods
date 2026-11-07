include!("pluto_storage_fixture.rs");
#[path = "../../mixins/tconstruct/source-melter-state.rs"] mod tconstruct_source_melter_state;
#[path = "../../mixins/tconstruct/source-channel-plan.rs"] mod tconstruct_source_channel_plan;
#[path = "../../mixins/tconstruct/source-channel-state.rs"] mod tconstruct_source_channel_state;
#[path = "../../mixins/tconstruct/source-channel-stage.rs"] mod tconstruct_source_channel_stage;
use horizon_world::nbt::Tag;
use tconstruct_source_channel_state::ChannelState;
use tconstruct_source_channel_stage::{Snapshot,stage};
fn s(pos:(i32,i32,i32),fluid:Option<(&str,Option<&Tag>,u32)>,locked:u32)->Snapshot {
 let mut state=ChannelState::new(); if let Some((id,tag,n))=fluid {assert!(state.tank.insert_tagged(id,tag,n));} state.locked=locked;
 Snapshot {pos,raw:state.to_nbt().unwrap()}
}
#[test] fn downward_transfer_stages_both_and_locks_new_fluid() {
 let a=s((0,0,0),Some(("test:fluid",None,30)),5); let b=s((0,-1,0),None,0);
 let result=stage(a.clone(),Some(b.clone()),[(false,None),(false,None),(false,None),(false,None)]).unwrap();
 assert_eq!(result[0].before,a.raw); assert_eq!(result[1].before,b.raw);
 let src=ChannelState::from_nbt(&result[0].after).unwrap(); let dst=ChannelState::from_nbt(&result[1].after).unwrap();
 assert_eq!((src.tank.amount,src.locked,dst.tank.amount,dst.locked),(20,0,10,10));
}
#[test] fn tags_and_unknown_nbt_survive_staging() {
 let tag=Tag::compound([("k",Tag::Int(9))]); let mut a=s((0,0,0),Some(("test:f",Some(&tag),20)),0);
 let Tag::Compound(ref mut root)=a.raw else {panic!()}; root.insert("extension".into(),Tag::Int(8));
 let b=s((1,0,0),Some(("test:f",Some(&tag),10)),3);
 let r=stage(a,None,[(true,Some(b)),(false,None),(false,None),(false,None)]).unwrap();
 assert_eq!(r[0].after.get("extension"),Some(&Tag::Int(8)));
 let out=ChannelState::from_nbt(&r[1].after).unwrap(); assert_eq!((out.tank.amount,out.locked,out.tank.tag),(20,13,Some(tag)));
}
#[test] fn alias_and_malformed_sinks_are_rejected_without_mutation() {
 let a=s((0,0,0),Some(("test:f",None,20)),0); let b=s((1,0,0),None,0);
 assert!(stage(a.clone(),Some(a.clone()),[(false,None),(false,None),(false,None),(false,None)]).is_none());
 assert!(stage(a.clone(),Some(b.clone()),[(true,Some(b)),(false,None),(false,None),(false,None)]).is_none());
 assert!(stage(a.clone(),Some(Snapshot{pos:(2,0,0),raw:Tag::Int(7)}),[(false,None),(false,None),(false,None),(false,None)]).is_none());
 assert_eq!(ChannelState::from_nbt(&a.raw).unwrap().tank.amount,20);
}
#[test] fn incompatible_tags_block_sink_and_locked_fluid_cannot_flow() {
 let tag=Tag::compound([("k",Tag::Int(1))]); let a=s((0,0,0),Some(("test:f",Some(&tag),20)),20); let b=s((1,0,0),Some(("test:f",None,5)),0);
 let r=stage(a,None,[(true,Some(b)),(false,None),(false,None),(false,None)]).unwrap();
 assert_eq!(ChannelState::from_nbt(&r[0].after).unwrap().tank.amount,20);
 assert_eq!(ChannelState::from_nbt(&r[0].after).unwrap().locked,0);
 assert_eq!(r[1].before,r[1].after);
}
#[test] fn closed_side_with_sink_is_rejected() {
 let a=s((0,0,0),None,0); let b=s((1,0,0),None,0);
 assert!(stage(a,None,[(false,Some(b)),(false,None),(false,None),(false,None)]).is_none());
}
#[test] fn incompatible_down_falls_back_and_blocked_outputs_still_split() {
 let tag=Tag::compound([("k",Tag::Int(1))]);
 let a=s((0,0,0),Some(("test:f",Some(&tag),20)),0);
 let down=s((0,-1,0),Some(("test:f",None,5)),0);
 let side=s((1,0,0),None,0);
 let r=stage(a,Some(down),[(true,Some(side)),(true,None),(false,None),(false,None)]).unwrap();
 assert_eq!(r[1].before,r[1].after);
 assert_eq!(ChannelState::from_nbt(&r[2].after).unwrap().tank.amount,10);
 assert_eq!(ChannelState::from_nbt(&r[0].after).unwrap().tank.amount,10);
}
#[test] fn nested_unknown_tank_data_remains_in_staged_result() {
 let mut a=s((0,0,0),Some(("test:f",None,20)),0);
 let Tag::Compound(ref mut root)=a.raw else {panic!()};
 let Some(Tag::Compound(tank))=root.get_mut(&"tank".into()) else {panic!()};
 tank.insert("opaque_extension".into(),Tag::Long(42));
 let b=s((1,0,0),None,0);
 let r=stage(a,None,[(true,Some(b)),(false,None),(false,None),(false,None)]).unwrap();
 assert_eq!(r[0].after.get("tank").unwrap().get("opaque_extension"),Some(&Tag::Long(42)));
}
#[test] fn newly_received_fluid_does_not_flow_again_in_same_tick() {
 let a=s((0,0,0),Some(("test:f",None,20)),0); let b=s((1,0,0),None,0); let c=s((2,0,0),None,0);
 let first=stage(a,None,[(true,Some(b)),(false,None),(false,None),(false,None)]).unwrap();
 let b_after=Snapshot{pos:(1,0,0),raw:first[1].after.clone()};
 let second=stage(b_after,None,[(true,Some(c)),(false,None),(false,None),(false,None)]).unwrap();
 assert_eq!(ChannelState::from_nbt(&second[0].after).unwrap().tank.amount,10);
 assert_eq!(ChannelState::from_nbt(&second[0].after).unwrap().locked,0);
 assert_eq!(second[1].before,second[1].after);
}
#[test] fn repeated_call_requires_caller_tick_guard() {
 let a=s((0,0,0),Some(("test:f",None,20)),20); let b=s((1,0,0),None,0);
 let first=stage(a,None,[(true,Some(b)),(false,None),(false,None),(false,None)]).unwrap();
 assert_eq!(ChannelState::from_nbt(&first[0].after).unwrap().locked,0);
 let again=stage(Snapshot{pos:(0,0,0),raw:first[0].after.clone()},None,
   [(true,Some(Snapshot{pos:(1,0,0),raw:first[1].after.clone()})),(false,None),(false,None),(false,None)]).unwrap();
 // This deliberately demonstrates why a caller must reject a second stage in
 // the same game tick; this pure function has no tick identity to enforce it.
 assert_eq!(ChannelState::from_nbt(&again[1].after).unwrap().tank.amount,10);
}
