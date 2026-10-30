extern crate self as horizon_client;
pub mod items {
    #[derive(Clone, Debug, PartialEq)]
    pub struct ItemStack { pub item: String, pub count: u32, pub extra: u32 }
    impl ItemStack { pub fn new(item: &str, count: u32) -> Self { Self { item: item.into(), count, extra: 0 } } }
}
pub mod game {
    use super::items::ItemStack;
    pub struct Game<'a> { pub inventory: Vec<Option<ItemStack>>, pub marker: std::marker::PhantomData<&'a ()> }
}
include!("../../mixins/tconstruct/command.rs");
#[test]
fn split_sticks_and_excess() {
    use items::ItemStack as S;
    let mut g = game::Game { inventory: vec![Some(S::new("minecraft:flint",2)),Some(S::new("minecraft:stick",1)),Some(S::new("minecraft:stick",2)),None,None], marker: std::marker::PhantomData };
    let _ = tconstruct_command(&mut g,"tconstruct parts");
    assert_eq!(g.inventory.iter().flatten().filter(|s| s.item.starts_with("tconstruct:")).count(), 3);
    assert_eq!(g.inventory.iter().flatten().filter(|s| s.item == "minecraft:stick").map(|s| s.count).sum::<u32>(),1);
    assert_eq!(g.inventory.iter().flatten().find(|s| s.item == "minecraft:flint").unwrap().count,1);
    let after = g.inventory.clone();
    let _ = tconstruct_command(&mut g,"tconstruct parts");
    assert_eq!(g.inventory, after);
}
#[test]
fn missing_and_full_do_not_mutate() {
    use items::ItemStack as S;
    for inv in [vec![Some(S::new("minecraft:flint",1)),Some(S::new("minecraft:stick",1)),None,None], vec![Some(S::new("minecraft:flint",2)),Some(S::new("minecraft:stick",3)),Some(S::new("other",64))]] {
        let mut inv = inv;
        inv[0].as_mut().unwrap().extra = 17;
        let mut g = game::Game { inventory: inv.clone(), marker: std::marker::PhantomData };
        let _ = tconstruct_command(&mut g,"tconstruct parts");
        assert_eq!(g.inventory, inv);
    }
}
