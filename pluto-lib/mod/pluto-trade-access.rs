// Borrowed native merchant context and owned factory snapshots. Source-only
// attachment until the attachment-contract/native/game checks pass.
use super::*;
use std::sync::{Arc, OnceLock};
pub trait Random {
    fn next_int(&mut self, bound: i32) -> i32;
    fn next_float(&mut self) -> f32;
    fn next_double(&mut self) -> f64;
}
pub type Create = dyn Fn(&TradeContext<'_>, &mut dyn Random) -> std::result::Result<Option<MerchantOffer>, String> + Send + Sync;
#[derive(Clone)]
pub struct Factory { pub create: Arc<Create> }
pub type Provider = fn(&str) -> std::result::Result<Vec<Factory>, String>;
static PROVIDER: OnceLock<Provider> = OnceLock::new();
pub fn set_provider(provider: Provider) -> std::result::Result<(), String> {
    PROVIDER.set(provider).map_err(|_| "trade factory provider already installed".into())
}
pub fn factories(set: &str) -> std::result::Result<Vec<Factory>, String> {
    PROVIDER.get().map_or_else(|| Ok(Vec::new()), |provider| provider(set))
}
struct CallerRandom<'a,R>(&'a mut R);
impl<R: RandomSource> Random for CallerRandom<'_,R> {
    fn next_int(&mut self,bound:i32)->i32 { self.0.next_int_bound(bound).expect("positive bounded trade RNG") }
    fn next_float(&mut self)->f32 { self.0.next_float() }
    fn next_double(&mut self)->f64 { self.0.next_double() }
}
#[derive(Clone)]
enum Listing { Native(String), Registered(Factory) }
/// Native candidate list and amount providers stay native. Generic R is only
/// an ordinary helper parameter; no generic owner/method is injected.
pub fn generate<R: RandomSource>(trades: &Trades, ctx: &TradeContext<'_>, set_id: &str,
    custom: Vec<Factory>, random: &mut R) -> std::result::Result<Vec<MerchantOffer>, String> {
    let Some(set)=trades.sets.get(set_id) else { return Ok(Vec::new()) };
    let mut potential: Vec<_>=trades.trade_list(&set["trades"]).into_iter().map(Listing::Native).collect();
    potential.extend(custom.into_iter().map(Listing::Registered));
    let amount=int_provider(&set["amount"],random);
    let duplicates=set["allow_duplicates"].as_bool().unwrap_or(false);
    super::pluto_trade_pool::roll(potential,amount,duplicates,random,
        |r,b|r.next_int_bound(b).unwrap_or(0),
        |entry,r| match entry {
            Listing::Native(id)=>Ok(trades.trades.get(id).and_then(|trade|trades.offer_from(ctx,trade,r))),
            Listing::Registered(factory)=>(factory.create)(ctx,&mut CallerRandom(r)),
        })
}
