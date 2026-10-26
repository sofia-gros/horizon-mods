//! Server-side inspection and controlled core-state edits. Gameplay is vanilla.
use serde::Serialize;
use tconstruct_horizon::{Catalog, ToolState};

/// A core-derived subset to project onto Horizon's generic Tool profile.
#[derive(Debug, PartialEq)]
pub struct MiningProjection {
    pub speed: f32,
    pub incorrect_tag: &'static str,
    pub broken: bool,
}

pub fn mining_projection(
    catalog: &Catalog,
    item: &str,
    count: u32,
    source: &str,
) -> Result<MiningProjection, String> {
    if item != "minecraft:wooden_pickaxe" || count != 1 {
        return Err("not a TConstruct state specimen carrier".into());
    }
    let state = ToolState::from_json(catalog, source)?;
    let stats = catalog.evaluate(&state)?.stats;
    let incorrect_tag = match stats.harvest_tier.as_str() {
        "minecraft:wood" => "minecraft:incorrect_for_wooden_tool",
        "minecraft:stone" => "minecraft:incorrect_for_stone_tool",
        "minecraft:iron" => "minecraft:incorrect_for_iron_tool",
        "minecraft:diamond" => "minecraft:incorrect_for_diamond_tool",
        "minecraft:netherite" => "minecraft:incorrect_for_netherite_tool",
        _ => return Err("unsupported mining tier".into()),
    };
    Ok(MiningProjection {
        speed: if state.broken {
            0.3
        } else {
            stats.mining_speed.max(1.0)
        },
        incorrect_tag,
        broken: state.broken,
    })
}

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
    hooks: serde_json::Value,
}

pub fn inspect(
    catalog: &Catalog,
    item: &str,
    count: u32,
    source: &str,
    vanilla_damage: u32,
    request: &str,
) -> Result<String, String> {
    inspect_with_profile(catalog, item, count, source, vanilla_damage, request, false)
}

fn inspect_with_profile(
    catalog: &Catalog,
    item: &str,
    count: u32,
    source: &str,
    vanilla_damage: u32,
    request: &str,
    active: bool,
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
        hooks: if active {
            "mining".into()
        } else {
            false.into()
        },
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
        GameMode, Player, Registration, ServerMod, items_edit, items_read, items_tools,
    };

    fn active(player: &Player) -> Result<bool, String> {
        Ok(
            items_read::main_hand(&player.id, &["horizon:tool_control".into(), "owner".into()])?
                .and_then(|item| item.custom_text)
                .as_deref()
                == Some("tconstruct_tools"),
        )
    }

    fn profile(projection: MiningProjection) -> items_tools::Profile {
        let rules = if projection.broken {
            vec![]
        } else {
            vec![
                items_tools::Rule {
                    blocks: vec![format!("#{}", projection.incorrect_tag)],
                    speed: None,
                    correct_for_drops: Some(false),
                },
                items_tools::Rule {
                    blocks: vec!["#minecraft:mineable/pickaxe".into()],
                    speed: Some(projection.speed),
                    correct_for_drops: Some(true),
                },
            ]
        };
        items_tools::Profile {
            rules,
            default_speed: if projection.broken {
                projection.speed
            } else {
                1.0
            },
            damage_per_block: 0,
            can_destroy_in_creative: true,
            mining_enabled: true,
            unbreakable: true,
        }
    }

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
            if let ["activate"] = words.as_slice() {
                if !matches!(&player.game_mode, GameMode::Creative) {
                    return Err("activation requires creative mode".into());
                }
                let path = ["tconstruct".into(), "state_json".into()];
                let item = items_read::main_hand(&player.id, &path)?.ok_or("empty main hand")?;
                let source = item
                    .custom_text
                    .as_deref()
                    .ok_or("no TConstruct state in main hand")?;
                let projection = mining_projection(&self.0, &item.item, item.count, source)?;
                items_tools::replace_main_hand_profile(
                    &player.id,
                    &path,
                    source,
                    source,
                    &profile(projection),
                )?;
                return Ok(None);
            }
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
                if active(player)? {
                    let projection =
                        mining_projection(&self.0, &item.item, item.count, &replacement)?;
                    items_tools::replace_main_hand_profile(
                        &player.id,
                        &path,
                        source,
                        &replacement,
                        &profile(projection),
                    )?;
                } else {
                    items_edit::replace_main_hand_text(&player.id, &path, source, &replacement)?;
                }
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
            inspect_with_profile(
                &self.0,
                &item.item,
                item.count,
                source,
                item.damage,
                request,
                active(player)?,
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
