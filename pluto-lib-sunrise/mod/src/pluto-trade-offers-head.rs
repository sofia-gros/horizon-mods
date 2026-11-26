// Adapted from pinned MIT Horizon game/trading.rs::offers_from_set.
// License: pluto-lib/LICENSE.horizon. Map and RandomSequences ownership stays
// identical, and the sequence is returned even on an explicit factory error.
fn pluto_trade_offers(game: &mut Game<'_>, set: &str, variant: &str, origin: [f64; 3]) -> core::ops::ControlFlow<Vec<MerchantOffer>> {
    use crate::pluto_trade_access as access;
    if !game.native_authoritative() { return std::ops::ControlFlow::Continue(()) }
    let factories=access::factories(set).expect("PlutoLib trade pool snapshot failed");
    if factories.is_empty() { return std::ops::ControlFlow::Continue(()) }
    let Some(sequence)=game.stations().trades.sequence(set) else { return std::ops::ControlFlow::Continue(()) };
    let mut random=game.rand_sequences.lend(&sequence,game.level.seed);
    let next_map=game.peek_map_id();
    let tag=|t:&str|game.structure_tag(t);
    let maps=GameMaps { terrain:game.end.locator(),structure_tag:&tag,
        made:Default::default(),next_id:std::cell::Cell::new(next_map) };
    let ctx=TradeContext { items:game.items(),enchantments:&game.stations().enchantments,
        variant,origin,maps:Some(&maps) };
    let result=access::generate(&game.res.stations().trades,&ctx,set,factories,&mut random);
    let made=maps.made.into_inner();
    game.rand_sequences.give_back(sequence,random);
    let offers=result.expect("PlutoLib selected trade factory failed");
    game.register_exploration_maps(made);
    std::ops::ControlFlow::Break(offers)
}
