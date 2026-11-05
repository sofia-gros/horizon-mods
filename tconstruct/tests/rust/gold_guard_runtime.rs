// Real ItemStack/NBT with a narrow Game/attribute adapter. Not live gameplay.
mod game {
    use horizon_items::items::ItemStack;
    use horizon_world::nbt::Tag;
    use std::collections::{BTreeMap,HashMap,HashSet};
    pub(crate) use crate::tconstruct_gold_guard;
    pub struct Chunks { pub defs:Option<Result<BTreeMap<String,Vec<u8>>,()>>, pub tags:Option<Result<HashMap<String,HashSet<String>>,()>> }
    impl Chunks {
        pub fn __horizon_mixins_tconstruct_modifier_resource_snapshot(&self)->&Option<Result<BTreeMap<String,Vec<u8>>,()>> {&self.defs}
        pub fn __horizon_mixins_tconstruct_resolved_item_tags(&self)->&Option<Result<HashMap<String,HashSet<String>>,()>> {&self.tags}
    }
    pub struct World {pub chunks:Chunks}
    pub struct Items {pub item_tags:HashMap<String,HashSet<String>>}
    pub struct Resources(pub Items);
    impl Resources {pub fn items(&self)->&Items {&self.0}}
    #[derive(Clone)] pub struct Key;
    #[derive(Default)] pub struct Attributes(pub BTreeMap<String,f64>);
    impl Attributes {
        pub fn key(&self,_:&str)->Option<Key>{Some(Key)}
        pub fn has_modifier(&mut self,_:Option<Key>,id:&str)->Result<bool,()>{Ok(self.0.contains_key(id))}
        pub fn modifier_value(&mut self,_:Option<Key>,id:&str)->Result<f64,()>{self.0.get(id).copied().ok_or(())}
    }
    pub struct Health(pub f32);
    impl Health {pub fn health(&self)->f32{self.0}}
    pub struct Player {pub attributes:Attributes,pub health:Health,pub clamps:usize,pub writes:usize}
    impl Player {
        pub fn set_modifier(&mut self,_:&str,id:&str,amount:f64,_:&str,present:bool)->Result<(),()> {
            self.writes+=1;self.attributes.0.remove(id);if present{self.attributes.0.insert(id.into(),amount);}Ok(())
        }
        pub fn attribute(&mut self,_:&str)->Result<f64,()>{Ok(20.0+self.attributes.0.values().sum::<f64>())}
        pub fn set_health(&mut self,value:f32,_:bool)->Result<(),()>{self.clamps+=1;self.health.0=value;Ok(())}
    }
    pub struct Game<'a> {pub remote:Option<()>,pub player:Option<Player>,pub armor:[Option<ItemStack>;4],pub world:World,pub res:Resources,pub marker:std::marker::PhantomData<&'a ()>}
    pub mod tconstruct_modifier_snapshot {
        use super::*;
        pub fn highest_definition<'a>(raw:&'a BTreeMap<String,Vec<u8>>,id:&str)->Option<&'a [u8]>{raw.get(id).map(Vec::as_slice)}
    }
    pub mod tconstruct_source_material_traits {
        use super::*;
        pub struct Traits {pub levels:BTreeMap<String,i32>}
        pub fn levels(_: &World, stack:&ItemStack)->Option<Traits>{stack.data()?.other("minecraft:custom_data")?;Some(Traits {levels: match stack.data()?.other("minecraft:custom_data")?.get("revitalizing_trait") {Some(Tag::Int(n))=>BTreeMap::from([("tconstruct:revitalizing".into(),*n)]),_=>BTreeMap::new()}})}
        pub fn level_with_upgrade(stack:&ItemStack,traits:&Traits,id:&str)->Option<i32>{
            match stack.data()?.other("minecraft:custom_data")?.get(id){None=>Some(traits.levels.get(id).copied().unwrap_or(0)),Some(Tag::Int(n)) if *n>=0=>Some(*n+traits.levels.get(id).copied().unwrap_or(0)),_=>None}
        }
    }
    pub mod tconstruct_source_ranged_wear {include!("../../mixins/tconstruct/source-ranged-wear.rs");}
    pub mod tconstruct_gold_guard_world {include!("../../mixins/tconstruct/gold-guard-world.rs");}
    include!("../../mixins/tconstruct/gold-guard-attributes.rs");
    include!("../../mixins/tconstruct/golden-armor.rs");
    fn armor(guard:i32,golden:i32)->ItemStack {
        let mut s=ItemStack::new("tconstruct:slime_helmet",1);
        s.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tconstruct:gold_guard",Tag::Int(guard)),("tconstruct:golden",Tag::Int(golden))])));s
    }
    fn setup()->Game<'static> {
        Game {remote:None,player:Some(Player {attributes:Attributes::default(),health:Health(20.0),clamps:0,writes:0}),armor:[Some(armor(1,0)),None,None,None],
            world:World {chunks:Chunks {defs:Some(Ok(BTreeMap::from([
                ("tconstruct:tinkering/modifiers/gold_guard".into(),br#"{"level_display":"tconstruct:single_level","modules":[{"type":"tconstruct:golden_attribute","attribute":"minecraft:generic.max_health","each_level":4.0,"flat":4.0,"operation":"addition"},{"type":"tconstruct:volatile_flag","flag":"tconstruct:piglin_neutral","modifier_level":{"min":2}}]}"#.to_vec()),
                ("tconstruct:tinkering/modifiers/golden".into(),br#"{"level_display":"tconstruct:no_levels","modules":[{"type":"tconstruct:volatile_flag","flag":"tconstruct:piglin_neutral"}]}"#.to_vec())
            ]))),tags:Some(Ok(HashMap::from([("tconstruct:modifiable/armor/worn".into(),HashSet::from(["tconstruct:slime_helmet".into()]))]))) }},
            res:Resources(Items {item_tags:HashMap::from([("minecraft:piglin_safe_armor".into(),HashSet::from(["minecraft:golden_chestplate".into()]))])}),marker:Default::default()}
    }
    #[test] fn live_projection_counts_native_and_tool_flags_and_does_not_stack() {
        let mut game=setup();tconstruct_gold_guard_attributes((), &mut game);
        assert_eq!(game.player.as_mut().unwrap().attribute("max").unwrap(),24.0);
        assert!(!tconstruct_golden_armor(false,&game));
        game.armor[0]=Some(armor(2,0));game.armor[1]=Some(ItemStack::new("minecraft:golden_chestplate",1));game.armor[2]=Some(armor(1,1));
        tconstruct_gold_guard_attributes((), &mut game);
        let player=game.player.as_mut().unwrap();assert_eq!(player.attribute("max").unwrap(),36.0);assert_eq!(player.attributes.0.len(),1);assert_eq!(player.health.0,20.0);
        assert!(tconstruct_golden_armor(false,&game));
        let writes=game.player.as_ref().unwrap().writes;tconstruct_gold_guard_attributes((), &mut game);assert_eq!(game.player.as_ref().unwrap().writes,writes);
    }
    #[test] fn wrong_class_slot_does_not_activate_guard_but_remains_a_neutral_slot() {
        let mut game=setup();game.armor[0]=None;game.armor[1]=Some(armor(2,0));
        tconstruct_gold_guard_attributes((), &mut game);
        assert!(game.player.as_ref().unwrap().attributes.0.is_empty());
        assert!(tconstruct_golden_armor(false,&game));
        game.armor[0]=Some(armor(1,0));tconstruct_gold_guard_attributes((), &mut game);
        assert_eq!(game.player.as_mut().unwrap().attribute("max").unwrap(),28.0);
    }
    #[test] fn removal_reload_and_world_replacement_restore_health_without_healing() {
        let mut game=setup();game.armor[0]=Some(armor(2,0));tconstruct_gold_guard_attributes((), &mut game);
        game.player.as_mut().unwrap().health.0=28.0;game.armor[0]=Some(armor(1,0));tconstruct_gold_guard_attributes((), &mut game);
        assert_eq!(game.player.as_ref().unwrap().health.0,24.0);
        let defs=game.world.chunks.defs.take();tconstruct_gold_guard_attributes((), &mut game);
        assert_eq!(game.player.as_ref().unwrap().health.0,20.0);assert!(game.player.as_ref().unwrap().attributes.0.is_empty());
        game.world.chunks.defs=defs;tconstruct_gold_guard_attributes((), &mut game);assert_eq!(game.player.as_ref().unwrap().health.0,20.0);
        game.armor=[None,None,None,None];tconstruct_gold_guard_attributes((), &mut game);assert!(game.player.as_ref().unwrap().attributes.0.is_empty());
        let mut reloaded=setup();tconstruct_gold_guard_attributes((), &mut reloaded);assert_eq!(reloaded.player.as_mut().unwrap().attribute("max").unwrap(),24.0);
    }
    #[test] fn changed_source_malformed_tool_remote_and_broken_rules() {
        let mut game=setup();game.armor[0]=Some(armor(2,0));game.armor[0].as_mut().unwrap().damage=999;
        tconstruct_gold_guard_attributes((), &mut game);assert_eq!(game.player.as_mut().unwrap().attribute("max").unwrap(),28.0);assert!(tconstruct_golden_armor(false,&game));
        game.world.chunks.defs.as_mut().unwrap().as_mut().unwrap().insert("tconstruct:tinkering/modifiers/gold_guard".into(),b"{}".to_vec());
        tconstruct_gold_guard_attributes((), &mut game);assert_eq!(game.player.as_mut().unwrap().attribute("max").unwrap(),20.0);assert!(!tconstruct_golden_armor(false,&game));assert!(tconstruct_golden_armor(true,&game));
        let mut game=setup();game.armor[0]=Some(armor(-1,0));tconstruct_gold_guard_attributes((), &mut game);assert!(game.player.as_ref().unwrap().attributes.0.is_empty());
        let mut game=setup();game.remote=Some(());tconstruct_gold_guard_attributes((), &mut game);assert!(game.player.as_ref().unwrap().attributes.0.is_empty());
    }
    fn rev_setup()->Game<'static> {
        let mut game=setup();game.armor=[None,None,None,None];
        game.world.chunks.defs.as_mut().unwrap().as_mut().unwrap().insert("tconstruct:tinkering/modifiers/revitalizing".into(),br#"{
  "level_display": "tconstruct:default",
  "modules": [
    {
      "type": "tconstruct:attribute",
      "attribute": "minecraft:generic.max_health",
      "each_level": 2.0,
      "operation": "addition",
      "slots": [
        "mainhand",
        "offhand",
        "feet",
        "legs",
        "chest",
        "head"
      ],
      "tooltip_style": "boost"
    },
    {
      "type": "tconstruct:update_health",
      "slots": [
        "feet",
        "legs",
        "chest",
        "head"
      ]
    },
    {
      "type": "tconstruct:update_health",
      "bonus": {
        "each_level": 2.0
      },
      "slots": [
        "mainhand",
        "offhand"
      ]
    }
  ]
}"#.to_vec());
        game.world.chunks.tags.as_mut().unwrap().as_mut().unwrap().get_mut("tconstruct:modifiable/armor/worn").unwrap().extend(["tconstruct:plate_chestplate".into(),"tconstruct:plate_leggings".into()]);
        game
    }
    fn rev(item:&str,level:i32)->ItemStack {
        let mut stack=ItemStack::new(item,1);
        stack.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("revitalizing_trait",Tag::Int(level)),("tic_max_damage",Tag::Int(999))])));stack
    }
    #[test] fn revitalizing_worn_levels_clamp_without_healing_and_restore() {
        let mut game=rev_setup();game.armor[1]=Some(rev("tconstruct:plate_chestplate",2));game.armor[2]=Some(rev("tconstruct:plate_leggings",1));
        tconstruct_gold_guard_attributes((),&mut game);
        assert_eq!(game.player.as_mut().unwrap().attribute("max").unwrap(),26.0);
        assert_eq!(game.player.as_ref().unwrap().health.0,20.0);
        game.player.as_mut().unwrap().health.0=26.0;game.armor[1]=None;
        tconstruct_gold_guard_attributes((),&mut game);assert_eq!(game.player.as_ref().unwrap().health.0,22.0);
        game.armor[2].as_mut().unwrap().damage=999;
        tconstruct_gold_guard_attributes((),&mut game);assert_eq!(game.player.as_ref().unwrap().health.0,20.0);
        game.armor[2].as_mut().unwrap().damage=0;
        tconstruct_gold_guard_attributes((),&mut game);assert_eq!(game.player.as_mut().unwrap().attribute("max").unwrap(),22.0);assert_eq!(game.player.as_ref().unwrap().health.0,20.0);
    }
    #[test] fn revitalizing_same_refresh_gold_guard_swap_uses_final_maximum() {
        let mut game=rev_setup();game.armor[0]=Some(armor(1,0));tconstruct_gold_guard_attributes((),&mut game);
        game.player.as_mut().unwrap().health.0=24.0;game.armor[0]=None;game.armor[1]=Some(rev("tconstruct:plate_chestplate",2));
        tconstruct_gold_guard_attributes((),&mut game);assert_eq!(game.player.as_ref().unwrap().health.0,24.0);assert_eq!(game.player.as_ref().unwrap().clamps,0);
        game.armor[1]=None;game.armor[0]=Some(armor(1,0));tconstruct_gold_guard_attributes((),&mut game);assert_eq!(game.player.as_ref().unwrap().health.0,24.0);
    }
    #[test] fn revitalizing_reload_wrong_slot_upgrade_and_remote_fail_closed() {
        let mut game=rev_setup();game.armor[0]=Some(rev("tconstruct:plate_chestplate",1));tconstruct_gold_guard_attributes((),&mut game);assert!(game.player.as_ref().unwrap().attributes.0.is_empty());
        game.armor[1]=game.armor[0].take();tconstruct_gold_guard_attributes((),&mut game);assert_eq!(game.player.as_mut().unwrap().attribute("max").unwrap(),22.0);
        let definitions=game.world.chunks.defs.take();game.player.as_mut().unwrap().health.0=22.0;
        tconstruct_gold_guard_attributes((),&mut game);assert_eq!(game.player.as_ref().unwrap().health.0,20.0);
        game.world.chunks.defs=definitions;tconstruct_gold_guard_attributes((),&mut game);assert_eq!(game.player.as_mut().unwrap().attribute("max").unwrap(),22.0);
        game.armor[1].as_mut().unwrap().data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("revitalizing_trait",Tag::Int(1)),("tconstruct:revitalizing",Tag::Int(1))])));
        tconstruct_gold_guard_attributes((),&mut game);assert_eq!(game.player.as_mut().unwrap().attribute("max").unwrap(),20.0);
        game.armor[1]=Some(rev("tconstruct:plate_chestplate",1));game.remote=Some(());tconstruct_gold_guard_attributes((),&mut game);assert_eq!(game.player.as_mut().unwrap().attribute("max").unwrap(),20.0);
    }

    #[test] fn revitalizing_changed_definition_tag_and_invalid_level_remove_bonus() {
        let mut game=rev_setup();game.armor[1]=Some(rev("tconstruct:plate_chestplate",1));tconstruct_gold_guard_attributes((),&mut game);
        let defs=game.world.chunks.defs.as_mut().unwrap().as_mut().unwrap();let old=defs.insert("tconstruct:tinkering/modifiers/revitalizing".into(),b"{}".to_vec()).unwrap();
        tconstruct_gold_guard_attributes((),&mut game);assert_eq!(game.player.as_mut().unwrap().attribute("max").unwrap(),20.0);
        game.world.chunks.defs.as_mut().unwrap().as_mut().unwrap().insert("tconstruct:tinkering/modifiers/revitalizing".into(),old);
        game.world.chunks.tags.as_mut().unwrap().as_mut().unwrap().get_mut("tconstruct:modifiable/armor/worn").unwrap().remove("tconstruct:plate_chestplate");
        tconstruct_gold_guard_attributes((),&mut game);assert_eq!(game.player.as_mut().unwrap().attribute("max").unwrap(),20.0);
        let mut game=rev_setup();game.armor[1]=Some(rev("tconstruct:plate_chestplate",-1));tconstruct_gold_guard_attributes((),&mut game);assert!(game.player.as_ref().unwrap().attributes.0.is_empty());
        for invalid in [Tag::Int(-1),Tag::String("1".into())] {
            game.armor[1]=Some(rev("tconstruct:plate_chestplate",1));
            game.armor[1].as_mut().unwrap().data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("revitalizing_trait",Tag::Int(1)),("tconstruct:revitalizing",invalid)])));
            tconstruct_gold_guard_attributes((),&mut game);assert!(game.player.as_ref().unwrap().attributes.0.is_empty());
        }
    }

}
