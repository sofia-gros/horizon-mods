//! Counterpart of Fabric TradeOfferHelper: append factories before selection.
//! Native trade-set attachment remains an explicit support-matrix gap.
use std::sync::{Arc, Mutex, OnceLock};
use super::player_api::StackData;

/// The caller's authoritative trade RNG, shared by selection and construction.
pub trait TradeRandom {
    fn next_int(&mut self, bound: i32) -> i32;
    fn next_float(&mut self) -> f32;
    fn next_double(&mut self) -> f64;
}
/// Borrowed registry view; no static tag fallback or invented item existence.
pub trait TradeWorld {
    /// None is unavailable/missing. Preserve native tag member order.
    fn item_tag(&self, tag: &str) -> Option<Vec<String>>;
    fn item_exists(&self, item: &str) -> bool;
}
/// Context for a selected factory; creating a pool must not roll the offer.
pub struct TradeOfferContext<'a> {
    pub world: &'a dyn TradeWorld,
    pub position: [f64; 3],
    pub variant: &'a str,
}
/// A fresh merchant offer. Existing uses/demand/special prices are native state.
#[derive(Clone, Debug, PartialEq)]
pub struct TradeOffer {
    pub first_buy: StackData,
    pub second_buy: Option<StackData>,
    pub sell: StackData,
    pub max_uses: i32,
    pub experience: i32,
    pub price_multiplier: f32,
}
type Create = dyn Fn(&TradeOfferContext<'_>, &mut dyn TradeRandom) -> Result<Option<TradeOffer>, String> + Send + Sync;
/// Cloneable factory identity. Repeating it in a pool increases selection weight.
#[derive(Clone)]
pub struct TradeOfferFactory { create: Arc<Create> }
impl TradeOfferFactory {
    pub fn new(create: impl Fn(&TradeOfferContext<'_>, &mut dyn TradeRandom) -> Result<Option<TradeOffer>, String> + Send + Sync + 'static) -> Self {
        Self { create: Arc::new(create) }
    }
    /// None is a declined listing; Err is an explicit construction failure.
    pub fn create(&self, context: &TradeOfferContext<'_>, random: &mut dyn TradeRandom) -> Result<Option<TradeOffer>, String> {
        (self.create)(context, random)
    }
}
type Provider = Arc<dyn Fn() -> Vec<TradeOfferFactory> + Send + Sync>;
fn registry() -> &'static Mutex<Vec<(u32, Provider)>> {
    static PROVIDERS: OnceLock<Mutex<Vec<(u32, Provider)>>> = OnceLock::new();
    PROVIDERS.get_or_init(Default::default)
}
/// Register wandering-trader factories, level 1 common / level 2 rare.
pub struct TradeOfferHelper;
impl TradeOfferHelper {
    /// The provider appends owned factories without invoking them. It runs
    /// outside registry locks for each pool snapshot, allowing config updates.
    /// Snapshots keep their factory list unchanged if config changes mid-roll.
    pub fn register_wandering_trader_offers(level: u32,
        provider: impl Fn() -> Vec<TradeOfferFactory> + Send + Sync + 'static) -> Result<(), String> {
        if !matches!(level, 1 | 2) { return Err("wandering trade level must be 1 (common) or 2 (rare)".into()); }
        registry().lock().map_err(|_| "trade registry poisoned")?.push((level, Arc::new(provider)));
        Ok(())
    }
    /// Custom entries to append to a native pool BEFORE selection, in provider
    /// registration order, with duplicates preserved. This does not itself run
    /// a merchant or replace the native pool's RNG/amount/duplicate policy.
    pub fn wandering_trader_offers(level: u32) -> Result<Vec<TradeOfferFactory>, String> {
        if !matches!(level, 1 | 2) { return Err("wandering trade level must be 1 or 2".into()); }
        let providers: Vec<_> = registry().lock().map_err(|_| "trade registry poisoned")?.iter()
            .filter(|(l, _)| *l == level).map(|(_, p)| p.clone()).collect();
        Ok(providers.into_iter().flat_map(|provider| provider()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn providers_snapshot_duplicates_without_rolling_and_allow_reentry() {
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
        let enabled = Arc::new(AtomicBool::new(true));
        let invoked = Arc::new(AtomicUsize::new(0));
        let flag = enabled.clone(); let calls = invoked.clone();
        TradeOfferHelper::register_wandering_trader_offers(2, move || {
            // Different level proves no registry lock is held during providers.
            let _ = TradeOfferHelper::wandering_trader_offers(1).unwrap();
            let calls = calls.clone();
            let factory = TradeOfferFactory::new(move |_, _| { calls.fetch_add(1, Ordering::Relaxed); Ok(None) });
            if flag.load(Ordering::Relaxed) { vec![factory.clone(), factory] } else { vec![] }
        }).unwrap();
        let snapshot = TradeOfferHelper::wandering_trader_offers(2).unwrap();
        assert_eq!(snapshot.len(),2); assert_eq!(invoked.load(Ordering::Relaxed),0);
        enabled.store(false, Ordering::Relaxed);
        assert!(TradeOfferHelper::wandering_trader_offers(2).unwrap().is_empty());
        assert_eq!(snapshot.len(),2);
        struct World;
        impl TradeWorld for World {
            fn item_tag(&self, _: &str) -> Option<Vec<String>> { None }
            fn item_exists(&self, _: &str) -> bool { false }
        }
        struct Random;
        impl TradeRandom for Random {
            fn next_int(&mut self, _: i32) -> i32 { panic!("declined factory must not roll") }
            fn next_float(&mut self) -> f32 { panic!() }
            fn next_double(&mut self) -> f64 { panic!() }
        }
        let context = TradeOfferContext { world: &World, position: [0.0;3], variant:"minecraft:plains" };
        for factory in snapshot { assert!(factory.create(&context, &mut Random).unwrap().is_none()); }
        assert_eq!(invoked.load(Ordering::Relaxed),2);
        assert!(TradeOfferFactory::new(|_,_|Err("build failed".into())).create(&context,&mut Random).is_err());
        assert!(TradeOfferHelper::register_wandering_trader_offers(3, Vec::new).is_err());
    }
}
