#[path = "../../mixins/tconstruct/source-melter-state.rs"] mod tconstruct_source_melter_state;
use tconstruct_source_melter_state::{Faucet,Tank};
#[test]
fn moves_one_packet_between_known_tanks_and_resumes_after_save() {
 let mut source=Tank::new(4000); assert!(source.insert("tconstruct:molten_copper",180));
 let mut target=Tank::new(4320); let mut faucet=Faucet::new();
 assert!(faucet.activate(&mut source,&target));
 assert_eq!((source.amount,faucet.pending.amount,target.amount),(90,90,0));
 for _ in 0..4 { faucet.tick(&mut source,Some(&mut target)); }
 let mut faucet=Faucet::from_nbt(&faucet.to_nbt()).unwrap();
 let mut source=Tank::from_nbt(&source.to_nbt(),4000).unwrap();
 let mut target=Tank::from_nbt(&target.to_nbt(),4320).unwrap();
 for _ in 0..5 { faucet.tick(&mut source,Some(&mut target)); }
 assert_eq!((source.amount,faucet.pending.amount,target.amount),(90,0,90));
 faucet.tick(&mut source,Some(&mut target));
 assert_eq!((source.amount,faucet.pending.amount,target.amount),(0,90,90));
}
#[test]
fn blocked_and_tagged_destinations_never_consume_source_or_packet() {
 let tag=horizon_world::nbt::Tag::compound([("x",horizon_world::nbt::Tag::Int(1))]);
 let mut source=Tank::new(4000); assert!(source.insert_tagged("x:copper",Some(&tag),90));
 let mut target=Tank::new(90); assert!(target.insert("x:iron",90));
 let mut faucet=Faucet::new(); assert!(!faucet.activate(&mut source,&target)); assert_eq!(source.amount,90);
 target=Tank::new(90); assert!(faucet.activate(&mut source,&target));
 assert!(target.insert("x:iron",90)); faucet.tick(&mut source,Some(&mut target));
 assert_eq!((source.amount,faucet.pending.amount,target.amount),(0,90,90));
 assert_eq!(faucet.pending.tag,Some(tag));
}
