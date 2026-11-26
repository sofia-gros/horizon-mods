// Borrowed native merchant context and owned factory snapshots. Source-only
// attachment until the attachment-contract/native/game checks pass.
use sunrise_client::merchant::{TradeContext, Trades};
use sunrise_client::merchant::offer::MerchantOffer;
use sunrise_core::random::RandomSource;
use serde_json::Value;
use sunrise_client_lib::*;
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

/// Adapted from pinned MIT Sunrise merchant.rs (its copies are module-private):
/// evaluate a trade amount IntProvider against the caller's random source.
fn int_provider<R: RandomSource>(v: &serde_json::Value, random: &mut R) -> i32 {
    use serde_json::Value;
    match v {
        Value::Number(n) => n.as_i64().unwrap_or(0) as i32,
        Value::Object(o) => match o.get("type").and_then(Value::as_str).unwrap_or("") {
            "minecraft:uniform" => {
                let (min, max) = (
                    o["min"].as_i64().unwrap_or(0) as i32,
                    o["max"].as_i64().unwrap_or(0) as i32,
                );
                if min >= max { min } else { min + random.next_int_bound(max - min + 1).unwrap_or(0) }
            }
            "minecraft:constant" => o["value"].as_i64().unwrap_or(0) as i32,
            "minecraft:add" => {
                let mut total = 0i64;
                for input in o["inputs"].as_array().into_iter().flatten() {
                    total += i64::from(int_provider(input, random));
                }
                total.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
            }
            "minecraft:binomial" => {
                let n = int_provider(&o["n"], random);
                let p = float_provider(&o["p"], random);
                (0..n).filter(|_| random.next_float() < p).count() as i32
            }
            _ => o.get("value").and_then(Value::as_i64).unwrap_or(0) as i32,
        },
        _ => 0,
    }
}

fn float_provider<R: RandomSource>(v: &serde_json::Value, random: &mut R) -> f32 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(0.0) as f32,
        Value::Object(o) => match o.get("type").and_then(Value::as_str).unwrap_or("") {
            "minecraft:uniform" => {
                let min = float_provider(&o["min"], random);
                let max = float_provider(&o["max"], random);
                random.next_float() * (max - min) + min
            }
            _ => o.get("value").and_then(Value::as_f64).unwrap_or(0.0) as f32,
        },
        _ => 0.0,
    }
}
/// Adapted from pinned MIT Sunrise merchant.rs (its copy is private):
/// a trade set's id list — a `#tag` reference, one id, or an array of ids.
fn trade_list(trades: &Trades, spec: &Value) -> Vec<String> {
    match spec {
        Value::String(s) => match s.strip_prefix('#') {
            Some(tag) => trades.tags.get(tag).cloned().unwrap_or_default(),
            None => vec![s.clone()],
        },
        Value::Array(a) => a
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect(),
        _ => vec![],
    }
}

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
    let mut potential: Vec<_>=trade_list(trades, &set["trades"]).into_iter().map(Listing::Native).collect();
    potential.extend(custom.into_iter().map(Listing::Registered));
    let amount=int_provider(&set["amount"],random);
    let duplicates=set["allow_duplicates"].as_bool().unwrap_or(false);
    crate::pluto_trade_pool::roll(potential,amount,duplicates,random,
        |r,b|r.next_int_bound(b).unwrap_or(0),
        |entry,r| match entry {
            Listing::Native(id)=>Ok(trades.trades.get(id).and_then(|trade|trades.offer_from(ctx,trade,r))),
            Listing::Registered(factory)=>(factory.create)(ctx,&mut CallerRandom(r)),
        })
}
