//! Server-side inspection and controlled core-state edits. Gameplay is vanilla.
use serde::Serialize;
use tconstruct_horizon::{Catalog, ToolState};

#[derive(Serialize)]
struct Report<'a> {
    request: &'a str,
    parts: [&'a str; 3],
    damage: u32,
    broken: bool,
    durability: u32,
    mining: f32,
    attack: f32,
    speed: f32,
    vanilla_damage: u32,
    hooks: bool,
}

pub fn inspect(
    catalog: &Catalog,
    item: &str,
    count: u32,
    source: &str,
    vanilla_damage: u32,
    request: &str,
) -> Result<String, String> {
    if item != "minecraft:wooden_pickaxe" || count != 1 {
        return Err("not a TConstruct state specimen carrier".into());
    }
    if request.len() > 32 || !request.bytes().all(|c| c.is_ascii_alphanumeric()) {
        return Err("request must be at most 32 ASCII letters or digits".into());
    }
    let state = ToolState::from_json(catalog, source)?;
    let stats = catalog.evaluate(&state)?.stats;
    let report = Report {
        request,
        parts: state
            .materials
            .each_ref()
            .map(|part| part.strip_prefix("tconstruct:").unwrap_or(part)),
        damage: state.damage,
        broken: state.broken,
        durability: stats.durability()?,
        mining: stats.mining_speed,
        attack: stats.attack_damage,
        speed: stats.attack_speed,
        vanilla_damage,
        hooks: false,
    };
    let reply = serde_json::to_string(&report).map_err(|error| error.to_string())?;
    if reply.chars().count() > 256 {
        return Err("inspection reply exceeds the host chat limit".into());
    }
    Ok(reply)
}

#[cfg(target_arch = "wasm32")]
mod adapter {
    use super::*;
    use horizon_mod_sdk::server::{
        GameMode, Player, Registration, ServerMod, items_edit, items_read,
    };

    struct Inspector(Catalog);
    impl ServerMod for Inspector {
        fn init() -> Result<(Self, Registration), String> {
            Ok((
                Self(Catalog::embedded()?),
                Registration::new().command("tconstruct_tool"),
            ))
        }
        fn command(
            &mut self,
            player: &Player,
            _: &str,
            args: &str,
        ) -> Result<Option<String>, String> {
            let words: Vec<_> = args.split_whitespace().collect();
            if let [operation @ ("test-damage" | "test-repair"), points] = words.as_slice() {
                if !matches!(&player.game_mode, GameMode::Creative) {
                    return Err("test edits require creative mode".into());
                }
                let points = points
                    .parse::<u32>()
                    .map_err(|_| "points must be an unsigned integer")?;
                let path = ["tconstruct".into(), "state_json".into()];
                let item = items_read::main_hand(&player.id, &path)?.ok_or("empty main hand")?;
                let source = item
                    .custom_text
                    .as_deref()
                    .ok_or("no TConstruct state in main hand")?;
                let replacement =
                    edit_state(&self.0, &item.item, item.count, source, operation, points)?;
                items_edit::replace_main_hand_text(&player.id, &path, source, &replacement)?;
                return Ok(None); // Only the host reports whether the queued edit applied.
            }
            let request = match words.as_slice() {
                [] | ["inspect"] => "manual",
                ["inspect", request] => request,
                _ => return Err("usage: /tconstruct_tool inspect [request]".into()),
            };
            let item =
                items_read::main_hand(&player.id, &["tconstruct".into(), "state_json".into()])?
                    .ok_or("empty main hand")?;
            let source = item
                .custom_text
                .as_deref()
                .ok_or("no TConstruct state in main hand")?;
            inspect(
                &self.0,
                &item.item,
                item.count,
                source,
                item.damage,
                request,
            )
            .map(Some)
        }
    }
    horizon_mod_sdk::export_server_mod!(Inspector);
}

/// Raw core points for Creative test commands, not recipe costs or game wear.
pub fn edit_state(
    catalog: &Catalog,
    item: &str,
    count: u32,
    source: &str,
    operation: &str,
    points: u32,
) -> Result<String, String> {
    if item != "minecraft:wooden_pickaxe" || count != 1 {
        return Err("not a TConstruct state specimen carrier".into());
    }
    let mut state = ToolState::from_json(catalog, source)?;
    match operation {
        "test-damage" => state.damage_points(catalog, points)?,
        "test-repair" => {
            state.repair_points(catalog, points)?;
        }
        _ => return Err("unsupported test edit".into()),
    }
    state.to_json()
}
