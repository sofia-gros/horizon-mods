//! Executes the actual gameplay HEAD helper. These facade types do not verify
//! native Plan/NewBlock/J types, attachment or piston resolution.
use std::collections::BTreeMap;
struct Declaration {id:String}
struct NewBlock {decl:Declaration}
#[derive(Debug,PartialEq)]
enum J {Str(String),Obj(BTreeMap<String,J>)}
impl J {fn set(&mut self,key:&str,value:J) {let Self::Obj(row)=self else {panic!("expected gameplay object")};row.insert(key.into(),value);}}
pub(crate) mod bridge {
    use crate::block_settings_access_tests::Settings;
    static SETTINGS:std::sync::OnceLock<Settings>=std::sync::OnceLock::new();
    pub(super) fn install(settings:Settings) {assert!(SETTINGS.set(settings).is_ok());}
    pub(crate) fn block_settings(block:&str,set:impl FnOnce(&'static str)) {SETTINGS.get().unwrap().apply(block,set);}
}
include!("../../../mod/src/pluto-block-settings-gameplay.rs");
#[test]
fn actual_gameplay_helper_changes_only_declared_push_and_preserves_every_state_row() {
    use crate::{BlockBehavior,Identifier,PistonBehavior};
    use crate::block_settings_access_tests::{Declaration as NativeDeclaration,Settings};
    let behaviors=[BlockBehavior::new(Identifier::parse("example:floor").unwrap()).piston_behavior(PistonBehavior::Destroy),
        BlockBehavior::new(Identifier::parse("example:wall").unwrap()).piston_behavior(PistonBehavior::Normal),
        BlockBehavior::new(Identifier::parse("example:unchanged").unwrap())];
    for behavior in behaviors {crate::BlockBehaviors::register(behavior);}
    let settings=Settings::new(crate::pure::block_api_v1::piston_behavior_declarations().unwrap()
        .into_iter().map(|(block,p)|NativeDeclaration {block,piston:p as u8}).collect()).unwrap();
    bridge::install(settings);
    for (block,expected) in [("example:floor","POPPED"),("example:wall","PUSH_PULL"),("example:unchanged","template") ] {
        for state in ["north","south","east","west"] {
            let mut row=J::Obj(BTreeMap::from([("push".into(),J::Str("template".into())),("facing".into(),J::Str(state.into()))]));
            pluto_block_settings_gameplay(&NewBlock {decl:Declaration {id:block.into()}},&mut row);
            assert_eq!(row,J::Obj(BTreeMap::from([("push".into(),J::Str(expected.into())),("facing".into(),J::Str(state.into()))])));
        }
    }
}

