//! Bounded adapter fixture: real API/adapters, modeled host store/index.
#![allow(dead_code)]
extern crate self as horizon_client;
pub use pluto_inventory_api as pluto_inventory;
use std::{collections::BTreeMap, sync::Arc};
#[path="../../../mixins/pluto_native_creative/advancement-tag-state.rs"]
mod state;
pub mod chunk_map {
    #[derive(Default)]
    pub struct ChunkMap { pub state: super::state::TagState }
    impl ChunkMap {
        pub fn __horizon_mixins_pluto_native_creative_advancement_tags(&self)->&super::state::TagState { &self.state }
        pub fn __horizon_mixins_pluto_native_creative_advancement_tags_mut(&mut self)->&mut super::state::TagState { &mut self.state }
    }
}
#[derive(Default)]
struct World { chunks:chunk_map::ChunkMap }
#[derive(Default)]
struct Tree { tags:Vec<String> }
mod adv {
    use super::*;
    pub(super) use super::Tree;
    #[derive(Default)]
    pub struct InventoryIndex { pub members:BTreeMap<String,Vec<String>> }
    impl InventoryIndex {
        pub fn build<'a>(tree:&Tree,read:impl Fn(&str)->Vec<&'a str>)->Self {
            Self {members:tree.tags.iter().map(|tag|(tag.clone(),read(tag).into_iter().map(str::to_owned).collect())).collect()}
        }
    }
}
#[derive(Default)]
struct AdvState {
    tree:Arc<Tree>,inventory_index:Option<(Arc<Tree>,Arc<adv::InventoryIndex>)>,
    last_inventory:u64, progress:u64,
}
#[derive(Default)]
struct Items {item_tags:BTreeMap<String,Vec<String>>}
struct Game<'a> {world:World,adv:AdvState,items:&'a Items,actor:u32}
impl Game<'_> {fn items(&self)->&Items {self.items}}
#[path="../../../mixins/pluto_native_creative/advancement-tags.rs"]
mod native_sink;
mod native_hooks {
    use super::*;
    struct View<'a,'g>(&'a Game<'g>);
    include!("../../../mixins/pluto_native_creative/advancement-tag-predicate.rs");
    include!("../../../mixins/pluto_native_creative/advancement-tag-index.rs");
    include!("../../../mixins/pluto_native_creative/advancement-tags-clear.rs");
    pub fn query(game:&Game<'_>, registry:&str,tag:&str,id:&str)->std::ops::ControlFlow<bool> {
        pluto_advancement_tag_predicate(&View(game),registry,tag,id)
    }
    pub fn index(game:&mut Game<'_>)->std::ops::ControlFlow<Arc<adv::InventoryIndex>> {pluto_advancement_tag_index(game)}
    pub fn clear(game:&mut Game<'_>){pluto_clear_advancement_tags(&mut game.world.chunks)}
}
#[cfg(test)]
mod tests {
    use super::*;
    use pluto_inventory::advancements::tags::{TagSnapshot,Change,ReplaceError};
    use std::ops::ControlFlow::{Break,Continue};
    fn game(items:&Items)->Game<'_> {Game{world:World::default(),adv:AdvState{tree:Arc::new(Tree{tags:vec!["tag".into(),"native".into()]}),last_inventory:42,progress:7,..Default::default()},items,actor:10}}
    fn replace(game:&mut Game<'_>,owner:&str,members:Option<Vec<&str>>)->Result<Change,ReplaceError> {
        let tags=[("tag".into(),members.map(|x|x.into_iter().map(str::to_owned).collect()))].into_iter().collect();
        native_sink::with_game(game,|sink|sink.replace_owner(owner,tags))
    }
    fn built(game:&mut Game<'_>)->Arc<adv::InventoryIndex> { match native_hooks::index(game) {Break(v)=>v,Continue(())=>panic!("expected custom index")}}
    #[test]
    fn no_provider_leaves_native_cache_gate_progress_and_actor_untouched() {
        let items=Items::default();let mut game=game(&items);
        let index=Arc::new(adv::InventoryIndex::default());game.adv.inventory_index=Some((game.adv.tree.clone(),index.clone()));
        assert!(matches!(native_hooks::index(&mut game),Continue(())));
        assert!(Arc::ptr_eq(&game.adv.inventory_index.as_ref().unwrap().1,&index));
        assert_eq!(native_hooks::query(&game,"item","tag","x"),Continue(()));
        assert_eq!((game.adv.last_inventory,game.adv.progress,game.actor),(42,7,10));
    }
    #[test]
    fn unknown_and_known_empty_fail_closed_and_unmanaged_falls_back() {
        let items=Items{item_tags:[("native".into(),vec!["n".into()])].into_iter().collect()};let mut game=game(&items);
        replace(&mut game,"a",None).unwrap();
        assert_eq!(native_hooks::query(&game,"item","tag","x"),Break(false));
        assert_eq!(native_hooks::query(&game,"block","tag","x"),Continue(()));
        assert_eq!(native_hooks::query(&game,"item","native","n"),Continue(()));
        let first=built(&mut game);assert_eq!(first.members["native"],["n"]);assert!(first.members["tag"].is_empty());
        replace(&mut game,"a",Some(vec![])).unwrap();assert!(built(&mut game).members["tag"].is_empty());
        replace(&mut game,"a",Some(vec!["x"])).unwrap();assert_eq!(native_hooks::query(&game,"item","tag","x"),Break(true));
    }
    #[test]
    fn changed_snapshot_rebuilds_but_idempotence_and_guest_actor_do_not() {
        let items=Items::default();let mut game=game(&items);
        replace(&mut game,"a",Some(vec!["x"])).unwrap();let first=built(&mut game);
        game.actor=99;assert!(Arc::ptr_eq(&first,&built(&mut game)));
        replace(&mut game,"a",Some(vec!["x","x"])).unwrap();assert!(Arc::ptr_eq(&first,&built(&mut game)));
        replace(&mut game,"a",Some(vec!["y"])).unwrap();let next=built(&mut game);assert!(!Arc::ptr_eq(&first,&next));assert_eq!(next.members["tag"],["y"]);
        assert_eq!((game.adv.last_inventory,game.adv.progress,game.actor),(42,7,99));
    }
    #[test]
    fn parked_world_round_trip_never_reuses_another_worlds_custom_index() {
        let items=Items::default();let mut game=game(&items);
        replace(&mut game,"a",Some(vec!["world_a"])).unwrap();let first=built(&mut game);
        let mut parked=std::mem::take(&mut game.world);
        assert!(matches!(native_hooks::index(&mut game),Continue(())));assert!(game.adv.inventory_index.is_none());
        replace(&mut game,"a",Some(vec!["world_b"])).unwrap();assert_eq!(built(&mut game).members["tag"],["world_b"]);
        std::mem::swap(&mut parked,&mut game.world);
        let restored=built(&mut game);assert_eq!(restored.members["tag"],["world_a"]);assert!(Arc::ptr_eq(&first,&restored));
    }
    #[test]
    fn clear_and_release_drop_old_index_before_native_fallback() {
        let items=Items::default();let mut game=game(&items);
        replace(&mut game,"a",Some(vec!["x"])).unwrap();built(&mut game);
        native_hooks::clear(&mut game);assert_eq!(native_hooks::query(&game,"item","tag","x"),Continue(()));
        assert!(matches!(native_hooks::index(&mut game),Continue(())));assert!(game.adv.inventory_index.is_none());
        replace(&mut game,"a",Some(vec!["x"])).unwrap();built(&mut game);
        native_sink::with_game(&mut game,|sink|sink.replace_owner("a",TagSnapshot::new())).unwrap();
        assert!(matches!(native_hooks::index(&mut game),Continue(())));assert!(game.adv.inventory_index.is_none());
    }
    #[test]
    fn tree_change_and_explicit_invalidation_rebuild_without_inventory_event() {
        let items=Items::default();let mut game=game(&items);
        replace(&mut game,"a",Some(vec!["x"])).unwrap();let first=built(&mut game);
        game.adv.tree=Arc::new(Tree{tags:vec!["tag".into()]});let second=built(&mut game);assert!(!Arc::ptr_eq(&first,&second));
        native_sink::with_game(&mut game,|sink|sink.invalidate_candidates());assert!(!Arc::ptr_eq(&second,&built(&mut game)));
        assert_eq!((game.adv.last_inventory,game.adv.progress),(42,7));
    }
    #[test]
    fn custom_cache_never_overwrites_native_cache_and_wrong_payload_rebuilds() {
        let items=Items::default();let mut game=game(&items);
        let native=Arc::new(adv::InventoryIndex::default());
        game.adv.inventory_index=Some((game.adv.tree.clone(),native.clone()));
        replace(&mut game,"a",Some(vec!["x"])).unwrap();built(&mut game);
        assert!(Arc::ptr_eq(&game.adv.inventory_index.as_ref().unwrap().1,&native));
        game.world.chunks.state.cache=Some(Arc::new(String::from("wrong type")));
        assert_eq!(built(&mut game).members["tag"],["x"]);
        let next:[(String,TagSnapshot);1]=[("a".into(),[("tag".into(),Some(vec!["y".into()]))].into_iter().collect())];
        // Defend against a mismatched retained payload even if native storage is
        // changed without going through the safe producer API.
        game.world.chunks.state.owners=Arc::new(next.into_iter().collect());
        assert_eq!(built(&mut game).members["tag"],["y"]);
        native_hooks::clear(&mut game);
        assert!(matches!(native_hooks::index(&mut game),Continue(())));
        assert!(Arc::ptr_eq(&game.adv.inventory_index.as_ref().unwrap().1,&native));
    }
    #[test]
    fn dropping_a_detached_world_releases_snapshot_without_global_cache() {
        let items=Items::default();let mut game=game(&items);
        replace(&mut game,"a",Some(vec!["x"])).unwrap();
        let weak=Arc::downgrade(&game.world.chunks.state.owners);
        let returned_index=built(&mut game);
        let detached=std::mem::take(&mut game.world);drop(detached);
        assert!(weak.upgrade().is_none());
        assert_eq!(returned_index.members["tag"],["x"]);
        assert!(matches!(native_hooks::index(&mut game),Continue(())));
    }
    #[test]
    fn conflict_rejection_preserves_snapshot_and_cached_index() {
        let items=Items::default();let mut game=game(&items);
        replace(&mut game,"a",Some(vec!["x"])).unwrap();let first=built(&mut game);
        assert!(replace(&mut game,"b",Some(vec!["y"])).is_err());assert!(Arc::ptr_eq(&first,&built(&mut game)));
    }
}
