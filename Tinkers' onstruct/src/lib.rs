//! Game-independent port core, diagnostics and state transport specimens.
pub mod catalog;
pub mod specimen;
pub mod stats;
pub mod tool;

#[cfg(target_arch = "wasm32")]
mod adapter;

pub use catalog::Catalog;
pub use tool::{Evaluation, ToolState};

pub fn diagnostic(catalog: &Catalog, args: &str) -> Result<String, String> {
    let words: Vec<_> = args.split_whitespace().collect();
    match words.as_slice() {
        [] | ["help"] => Ok("Development diagnostics only; usage: /tconstruct stats <head> <handle> <binding> | specimen <head> <handle> <binding> [core-damage] | support. Specimens use vanilla gameplay.".into()),
        ["support"] => Ok("Core: wood/flint pickaxe, cultivated/jagged/pierce evaluation. Game hooks, playable tools and machines: not implemented.".into()),
        ["stats", head, handle, binding] => {
            let state = ToolState::pickaxe([*head, *handle, *binding]);
            let result = catalog.evaluate(&state)?;
            Ok(format!("Pickaxe: durability={}, mining={:.3}, damage={:.3}, speed={:.3}; diagnostics only",
                result.stats.durability()?, result.stats.mining_speed, result.stats.attack_damage, result.stats.attack_speed))
        }
        _ => Err("usage: /tconstruct stats <head> <handle> <binding> | support".into()),
    }
}
