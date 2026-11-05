// Adapter fixture: real ItemStack/NBT and the unchanged production injection.
// This is not a complete Game, networking, or gameplay test.
mod game {
    use horizon_items::items::ItemStack;
    use horizon_world::nbt::Tag;
    pub(crate) use crate::tconstruct_multishot;
    pub struct Chunks(pub Option<Result<Vec<u8>, ()>>);
    impl Chunks {
        pub fn __horizon_mixins_tconstruct_modifier_resource_snapshot(&self) -> &Option<Result<Vec<u8>, ()>> { &self.0 }
    }
    pub struct Random(pub usize);
    impl Random { pub fn next_float(&mut self) -> f32 { self.0 += 1; 0.5 } }
    pub struct World { pub chunks: Chunks, pub random: Random, pub traits: Option<i32> }
    pub struct Position { pub x: f64, pub y: f64, pub z: f64 }
    pub struct Player { pub position: Position }
    #[derive(Default)] pub struct Audio(pub usize);
    impl Audio { pub fn play(&mut self, _: &str, _: Option<[f64; 3]>, _: f32, _: f32) { self.0 += 1; } }
    pub struct Ammo { pub stack: ItemStack, pub slot: Option<usize> }
    pub struct Game<'a> {
        pub remote: Option<()>, pub world: World, pub inventory: Vec<Option<ItemStack>>,
        pub creative: bool, pub using: Option<(usize, u32)>, pub crossbow_sounds: [bool; 2],
        pub player: Option<Player>, pub audio: Audio, pub ammo: Option<Ammo>, pub mutate: u8,
        pub marker: std::marker::PhantomData<&'a ()>,
    }
    impl Game<'_> {
        pub fn ticks_held(&self, _: &ItemStack, ticks: u32) -> u32 { ticks }
        pub fn charge_ticks(_: &ItemStack) -> u32 { 20 }
        pub fn charging_sounds(_: &ItemStack) -> [&'static str; 3] { ["start", "middle", "end"] }
        pub fn stack_in(&self, i: usize) -> Option<&ItemStack> { self.inventory.get(i)?.as_ref() }
        pub fn stack_in_mut(&mut self, i: usize) -> &mut Option<ItemStack> { &mut self.inventory[i] }
        pub fn get_projectile(&mut self, _: &ItemStack) -> Option<Ammo> {
            match self.mutate {
                1 => self.inventory[0] = Some(ItemStack::new("minecraft:stone", 1)),
                2 => self.inventory[1] = Some(ItemStack::new("minecraft:spectral_arrow", 9)),
                _ => {},
            }
            self.ammo.take()
        }
    }
    pub mod tconstruct_source_material_traits {
        use super::*;
        pub struct Traits(pub i32);
        pub fn levels(world: &World, _: &ItemStack) -> Option<Traits> { world.traits.map(Traits) }
        pub fn level_with_upgrade(stack: &ItemStack, traits: &Traits, _: &str) -> Option<i32> {
            let saved = tconstruct_multishot::saved_level(stack)?;
            if traits.0 < 0 { return None; }
            saved.checked_add(traits.0)
        }
    }
    pub mod tconstruct_source_ranged_wear {
        use super::*;
        pub fn is_broken(stack: &ItemStack) -> bool { stack.damage == 999 }
    }
    pub mod tconstruct_modifier_snapshot {
        pub fn highest_definition<'a>(raw: &'a [u8], _: &str) -> Option<&'a [u8]> { Some(raw) }
    }
    mod crossbow {
        use super::*;
        const OFFHAND_SLOT: usize = 40;
        fn charge_sounds(_: f32, played: [bool; 2], _: [&str; 3]) -> ([bool; 2], Option<&str>, Option<&str>) { (played, None, None) }
        include!("../../mixins/tconstruct/multishot-crossbow-load.rs");
        pub fn load(game: &mut Game<'_>, ticks: u32, bow: &ItemStack) -> std::ops::ControlFlow<()> { tconstruct_multishot_load(game, 0, ticks, bow) }
    }
    fn setup() -> (Game<'static>, ItemStack) {
        let mut bow = ItemStack::new("tconstruct:crossbow", 1);
        bow.data_mut().set_other("minecraft:custom_data", Some(Tag::compound([("tic_upgrades", Tag::compound([("tconstruct:multishot", Tag::Int(1))]))])));
        let arrow = ItemStack::new("minecraft:arrow", 2);
        let mut inventory = vec![None; 41];
        inventory[0] = Some(bow.clone()); inventory[1] = Some(arrow.clone()); inventory[2] = Some(ItemStack::new("minecraft:arrow", 5));
        (Game { remote: None, world: World { chunks: Chunks(Some(Ok(br#"{"level_display":"tconstruct:default","modules":[{"type":"tconstruct:volatile_int","each_level":2,"flag":"tconstruct:multishot"}]}"#.to_vec()))), random: Random(0), traits: Some(0) }, inventory, creative: false, using: None, crossbow_sounds: [false;2], player: None, audio: Audio::default(), ammo: Some(Ammo {stack: arrow, slot: Some(1)}), mutate: 0, marker: Default::default() }, bow)
    }
    #[test] fn real_injection_consumes_and_preserves_charged_components() {
        let (mut game, bow) = setup();
        assert!(crossbow::load(&mut game, 19, &bow).is_break());
        assert!(game.inventory[1].is_none()); assert_eq!(game.inventory[2].as_ref().unwrap().count, 4);
        let charged = &game.inventory[0].as_ref().unwrap().data().unwrap().charged;
        assert_eq!(charged.len(), 3); assert!(charged.iter().all(|s|s.count == 1));
        // Clone is the ItemStack representation used by inventory/state snapshots.
        assert_eq!(game.inventory[0].clone().unwrap().data().unwrap().charged.len(), 3);
        assert_eq!(game.world.random.0, 1); assert_eq!(game.using, Some((0,20)));
        let charged_bow = game.inventory[0].clone().unwrap();
        assert!(crossbow::load(&mut game, 20, &charged_bow).is_continue());
        assert_eq!(game.inventory[2].as_ref().unwrap().count, 4);
    }
    #[test] fn real_injection_partial_creative_early_and_remote_paths() {
        let (mut game, bow) = setup(); game.inventory[2]=None;
        assert!(crossbow::load(&mut game, 18, &bow).is_continue());
        assert_eq!(game.inventory[1].as_ref().unwrap().count,2);
        assert!(crossbow::load(&mut game, 19, &bow).is_break());
        assert_eq!(game.inventory[0].as_ref().unwrap().data().unwrap().charged.len(),2);
        let (mut game,bow)=setup();game.creative=true;game.ammo.as_mut().unwrap().slot=None;
        assert!(crossbow::load(&mut game,19,&bow).is_break());
        assert_eq!(game.inventory[1].as_ref().unwrap().count,2);
        assert!(game.inventory[0].as_ref().unwrap().data().unwrap().charged.iter().all(|s|s.data().unwrap().other("minecraft:intangible_projectile").is_some()));
        let (mut game,bow)=setup();game.remote=Some(());
        assert!(crossbow::load(&mut game,19,&bow).is_continue());
        assert_eq!(game.inventory[1].as_ref().unwrap().count,2);
    }
    #[test] fn real_injection_rejects_replaced_slots_without_consumption() {
        for mutate in [1,2] {
            let (mut game, bow) = setup(); game.mutate = mutate;
            assert!(crossbow::load(&mut game,19,&bow).is_break());
            assert_eq!(game.inventory[2].as_ref().unwrap().count,5);
            assert!(game.inventory[0].as_ref().unwrap().data().is_none_or(|d|d.charged.is_empty()));
            assert_eq!(game.world.random.0,0);
        }
    }
    #[test] fn real_injection_definition_reload_and_unsupported_sources_fail_closed() {
        for failure in 0..7 {
            let (mut game,bow)=setup();
            match failure {
                0=>game.world.chunks.0=None,
                1=>game.world.chunks.0=Some(Ok(b"{}".to_vec())),
                2=>game.ammo.as_mut().unwrap().slot=None,
                3=>game.ammo.as_mut().unwrap().stack.item="minecraft:firework_rocket".into(),
                4=>game.world.traits=Some(31),
                5=>game.world.traits=None,
                _=>game.world.traits=Some(i32::MAX),
            }
            assert!(crossbow::load(&mut game,19,&bow).is_break());
            assert_eq!(game.inventory[1].as_ref().unwrap().count,2);
            assert!(game.inventory[0].as_ref().unwrap().data().unwrap().charged.is_empty());
            assert_eq!(game.world.random.0,0);
        }
        let (mut game,bow)=setup();let canonical=game.world.chunks.0.take();
        assert!(crossbow::load(&mut game,19,&bow).is_break());
        game.world.chunks.0=canonical;
        assert!(crossbow::load(&mut game,19,&bow).is_break());
        assert_eq!(game.inventory[0].as_ref().unwrap().data().unwrap().charged.len(),3);
    }
}
