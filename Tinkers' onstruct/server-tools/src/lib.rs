//! Server-side inspection and projection of TConstruct state into generic item hooks.
use serde::Serialize;
use tconstruct_horizon::{Catalog, ToolState};

/// A core-derived subset to project onto Horizon's generic Tool profile.
#[derive(Debug, PartialEq)]
pub struct MiningProjection {
    pub speed: f32,
    pub incorrect_tag: &'static str,
    pub broken: bool,
    pub damage: u32,
    pub max_damage: u32,
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
    let evaluation = catalog.evaluate(&state)?;
    let incorrect_tag = match evaluation.stats.harvest_tier.as_str() {
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
            evaluation.mining_speed(catalog, &state, true)?.max(1.0)
        },
        incorrect_tag,
        broken: state.broken,
        damage: state.damage,
        max_damage: evaluation.stats.durability()?,
    })
}

/// Core-derived melee settings. The host supplies the player base/effects.
#[derive(Debug, PartialEq)]
pub struct MeleeProjection {
    pub damage_bonus: f32,
    pub damage_multiplier: f32,
    pub attack_speed: f32,
    pub enabled: bool,
}

pub fn melee_projection(
    catalog: &Catalog,
    item: &str,
    count: u32,
    source: &str,
) -> Result<MeleeProjection, String> {
    if item != "minecraft:wooden_pickaxe" || count != 1 {
        return Err("not a TConstruct state specimen carrier".into());
    }
    let state = ToolState::from_json(catalog, source)?;
    let evaluation = catalog.evaluate(&state)?;
    let bonus = evaluation.stats.attack_damage;
    let multiplier = if state.broken || bonus == 0.0 {
        1.0
    } else {
        evaluation.melee_damage(catalog, &state)? / bonus
    };
    Ok(MeleeProjection {
        damage_bonus: bonus,
        damage_multiplier: multiplier,
        attack_speed: evaluation.stats.attack_speed,
        enabled: !state.broken,
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
        Args, GameMode, GuiWorkstationOutput, GuiWorkstationPlan, Player, Registration, ServerMod,
        WorkstationInput, WorkstationPlan, inventory_transactions, items_edit, items_read,
        items_tools, workstations,
    };

    fn active(player: &Player) -> Result<bool, String> {
        active_id(&player.id)
    }

    fn active_id(player: &str) -> Result<bool, String> {
        Ok(
            items_read::main_hand(player, &["horizon:tool_control".into(), "owner".into()])?
                .and_then(|item| item.custom_text)
                .as_deref()
                == Some("tconstruct_tools"),
        )
    }

    fn profile(projection: &MiningProjection) -> items_tools::Profile {
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

    fn melee(projection: MeleeProjection) -> items_tools::MeleeProfile {
        items_tools::MeleeProfile {
            damage_bonus: projection.damage_bonus,
            damage_multiplier: projection.damage_multiplier,
            attack_speed: projection.attack_speed,
            enabled: projection.enabled,
        }
    }

    struct Inspector(Catalog);
    impl ServerMod for Inspector {
        fn init() -> Result<(Self, Registration), String> {
            items_tools::enable_mining_events();
            items_tools::enable_melee_events();
            let path = ["tconstruct".into(), "state_json".into()];
            let tags = [
                "minecraft:planks".into(),
                "tconstruct:wood_variants/planks".into(),
            ];
            workstations::register_gui(
                "tconstruct:part_builder",
                "Part Builder",
                &path,
                &tags,
                r##"{"width":176,"height":184,"slots":[{"id":"pattern","x":8,"y":43,"role":"input","accepts":["tconstruct:pattern"],"background":false},{"id":"material","x":29,"y":43,"role":"input","accepts":["minecraft:flint","#minecraft:planks"],"background":false},{"id":"result","x":148,"y":42,"role":"output","background":false}],"elements":[{"type":"image","x":0,"y":0,"width":176,"height":184,"texture":"tconstruct:gui/part_builder","region":[0,0,176,184]},{"type":"button","id":"head","x":51,"y":15,"width":18,"height":18,"text":"H","images":{"normal":"tconstruct:gui/part_builder","regions":{"normal":[176,15,18,18],"selected":[176,33,18,18],"hover":[176,51,18,18]}}},{"type":"button","id":"handle","x":69,"y":15,"width":18,"height":18,"text":"R","images":{"normal":"tconstruct:gui/part_builder","regions":{"normal":[176,15,18,18],"selected":[176,33,18,18],"hover":[176,51,18,18]}}},{"type":"button","id":"binding","x":87,"y":15,"width":18,"height":18,"text":"B","images":{"normal":"tconstruct:gui/part_builder","regions":{"normal":[176,15,18,18],"selected":[176,33,18,18],"hover":[176,51,18,18]}}}],"player_inventory":{"x":8,"y":102}}"##,
            )?;
            workstations::register_gui(
                "tconstruct:tinker_station",
                "Tinker Station",
                &path,
                &tags,
                r##"{"width":176,"height":184,"slots":[{"id":"first","x":53,"y":22,"role":"input"},{"id":"second","x":15,"y":60,"role":"input"},{"id":"third","x":33,"y":42,"role":"input"},{"id":"result","x":114,"y":38,"role":"output","background":false}],"elements":[{"type":"image","x":0,"y":0,"width":176,"height":184,"texture":"tconstruct:gui/tinker","region":[0,0,176,184]}],"player_inventory":{"x":8,"y":102,"background":false}}"##,
            )?;
            Ok((
                Self(Catalog::embedded()?),
                Registration::new().command("tconstruct_tool"),
            ))
        }
        fn workstation_preview(
            &mut self,
            block: &str,
            tool: &WorkstationInput,
            ingredient: &WorkstationInput,
        ) -> Result<Option<WorkstationPlan>, String> {
            if block != "tconstruct:tinker_station"
                || tool.item != "minecraft:wooden_pickaxe"
                || tool.count != 1
            {
                return Ok(None);
            }
            let Some(source) = tool.custom_text.as_deref() else {
                return Ok(None);
            };
            let material = if ingredient.item == "minecraft:flint" {
                "flint"
            } else if ingredient.tags.iter().any(|t| t == "minecraft:planks")
                && !ingredient
                    .tags
                    .iter()
                    .any(|t| t == "tconstruct:wood_variants/planks")
            {
                "wood"
            } else {
                return Ok(None);
            };
            let state = ToolState::from_json(&self.0, source)?;
            let Ok(plan) = tconstruct_horizon::crafting::plan_repair(
                &self.0,
                &state,
                material,
                ingredient.count,
            ) else {
                return Ok(None);
            };
            let replacement = plan.output.to_json()?;
            let mining = mining_projection(&self.0, &tool.item, tool.count, &replacement)?;
            Ok(Some(WorkstationPlan {
                replacement: replacement.clone(),
                consume: plan.material_items,
                tool: profile(&mining).into(),
                damage: mining.damage,
                max_damage: mining.max_damage,
                melee: melee(melee_projection(
                    &self.0,
                    &tool.item,
                    tool.count,
                    &replacement,
                )?)
                .into(),
            }))
        }
        fn gui_workstation_preview(
            &mut self,
            block: &str,
            inputs: &[Option<WorkstationInput>],
            selection: Option<&str>,
        ) -> Result<Option<GuiWorkstationPlan>, String> {
            use tconstruct_horizon::crafting::{
                Part, PartKind, plan_part, plan_pickaxe, plan_repair,
            };
            let material = |input: &WorkstationInput| -> Option<&'static str> {
                if input.item == "minecraft:flint" {
                    Some("flint")
                } else if input.tags.iter().any(|t| t == "minecraft:planks")
                    && !input
                        .tags
                        .iter()
                        .any(|t| t == "tconstruct:wood_variants/planks")
                {
                    Some("wood")
                } else {
                    None
                }
            };
            if block == "tconstruct:part_builder" {
                let [Some(pattern), Some(ingredient), None] = inputs else {
                    return Ok(None);
                };
                if pattern.item != "tconstruct:pattern" {
                    return Ok(None);
                }
                let kind = match selection {
                    Some("head") => PartKind::Head,
                    Some("handle") => PartKind::Handle,
                    Some("binding") => PartKind::Binding,
                    _ => return Ok(None),
                };
                let Some(name) = material(ingredient) else {
                    return Ok(None);
                };
                let Ok(part) = plan_part(&self.0, kind, name, ingredient.count, true) else {
                    return Ok(None);
                };
                let item = match kind {
                    PartKind::Head => "tconstruct:pick_head",
                    PartKind::Handle => "tconstruct:tool_handle",
                    PartKind::Binding => "tconstruct:tool_binding",
                };
                return Ok(Some(GuiWorkstationPlan {
                    consume: vec![0, part.material_items, 0],
                    outputs: vec![GuiWorkstationOutput {
                        item: item.into(),
                        count: 1,
                        source_slot: None,
                        text: Some(serde_json::to_string(&part.output).map_err(|e| e.to_string())?),
                        tool: None,
                        durability: None,
                        melee: None,
                    }],
                }));
            }
            if block != "tconstruct:tinker_station" {
                return Ok(None);
            }
            let [Some(first), Some(second), third, None] = inputs else {
                return Ok(None);
            };
            let (state, consume, source) = if first.item == "minecraft:wooden_pickaxe"
                && first.count == 1
                && third.is_none()
            {
                let Some(text) = first.custom_text.as_deref() else {
                    return Ok(None);
                };
                let Some(name) = material(second) else {
                    return Ok(None);
                };
                let Ok(state) = ToolState::from_json(&self.0, text) else {
                    return Ok(None);
                };
                let Ok(plan) = plan_repair(&self.0, &state, name, second.count) else {
                    return Ok(None);
                };
                (plan.output, vec![1, plan.material_items, 0, 0], Some(0))
            } else {
                let Some(third) = third else {
                    return Ok(None);
                };
                if first.item != "tconstruct:pick_head"
                    || second.item != "tconstruct:tool_handle"
                    || third.item != "tconstruct:tool_binding"
                {
                    return Ok(None);
                }
                let parse = |s: &WorkstationInput| -> Option<Part> {
                    serde_json::from_str(s.custom_text.as_deref()?).ok()
                };
                let (Some(a), Some(b), Some(c)) = (parse(first), parse(second), parse(third))
                else {
                    return Ok(None);
                };
                let Ok(state) = plan_pickaxe(&self.0, &[a, b, c]) else {
                    return Ok(None);
                };
                (state, vec![1, 1, 1, 0], None)
            };
            let replacement = state.to_json()?;
            let mining = mining_projection(&self.0, "minecraft:wooden_pickaxe", 1, &replacement)?;
            let combat = melee_projection(&self.0, "minecraft:wooden_pickaxe", 1, &replacement)?;
            Ok(Some(GuiWorkstationPlan {
                consume,
                outputs: vec![GuiWorkstationOutput {
                    item: "minecraft:wooden_pickaxe".into(),
                    count: 1,
                    source_slot: source,
                    text: Some(replacement),
                    tool: Some(profile(&mining).into()),
                    durability: Some((mining.damage, mining.max_damage)),
                    melee: Some(melee(combat).into()),
                }],
            }))
        }
        fn mined(
            &mut self,
            player: &str,
            _dimension: &str,
            _pos: [i32; 3],
            _block: &str,
            hardness: f32,
        ) -> Result<(), String> {
            if !active_id(player)? {
                return Ok(());
            }
            let path = ["tconstruct".into(), "state_json".into()];
            let item = items_read::main_hand(player, &path)?.ok_or("empty main hand")?;
            if item.item != "minecraft:wooden_pickaxe" || item.count != 1 {
                return Ok(());
            }
            let source = item
                .custom_text
                .as_deref()
                .ok_or("no TConstruct state in main hand")?;
            let mut state = ToolState::from_json(&self.0, source)?;
            let before = state.clone();
            state.damage_after_mining(&self.0, hardness)?;
            if state == before {
                return Ok(());
            }
            let replacement = state.to_json()?;
            let projection = mining_projection(&self.0, &item.item, item.count, &replacement)?;
            items_tools::replace_main_hand_combat_profile(
                player,
                &path,
                source,
                &replacement,
                &profile(&projection),
                projection.damage,
                projection.max_damage,
                melee(melee_projection(
                    &self.0,
                    &item.item,
                    item.count,
                    &replacement,
                )?),
            )
        }
        fn melee_hit(
            &mut self,
            player: &str,
            _dimension: &str,
            _target_id: u64,
            _target_kind: &str,
            _dealt: f32,
        ) -> Result<(), String> {
            if !active_id(player)? {
                return Ok(());
            }
            let path = ["tconstruct".into(), "state_json".into()];
            let item = items_read::main_hand(player, &path)?.ok_or("empty main hand")?;
            if item.item != "minecraft:wooden_pickaxe" || item.count != 1 {
                return Ok(());
            }
            let source = item
                .custom_text
                .as_deref()
                .ok_or("no TConstruct state in main hand")?;
            let mut state = ToolState::from_json(&self.0, source)?;
            let before = state.clone();
            state.damage_after_melee(&self.0, true)?;
            if state == before {
                return Ok(());
            }
            let replacement = state.to_json()?;
            let projection = mining_projection(&self.0, &item.item, item.count, &replacement)?;
            items_tools::replace_main_hand_combat_profile(
                player,
                &path,
                source,
                &replacement,
                &profile(&projection),
                projection.damage,
                projection.max_damage,
                melee(melee_projection(
                    &self.0,
                    &item.item,
                    item.count,
                    &replacement,
                )?),
            )
        }
        fn command(
            &mut self,
            player: &Player,
            _: &str,
            args: &Args,
        ) -> Result<Option<String>, String> {
            let words: Vec<_> = args.text().split_whitespace().collect();
            if let ["repair-flint", slot] = words.as_slice() {
                let slot = slot.parse::<u32>().map_err(|_| "slot must be 0..35")?;
                if slot >= 36 {
                    return Err("slot must be 0..35".into());
                }
                if !active(player)? {
                    return Err("activate the TConstruct tool first".into());
                }
                let path = ["tconstruct".into(), "state_json".into()];
                let item = items_read::main_hand(&player.id, &path)?.ok_or("empty main hand")?;
                let source = item
                    .custom_text
                    .as_deref()
                    .ok_or("no TConstruct state in main hand")?;
                let ingredient = inventory_transactions::inventory_slot(&player.id, slot, &[])?
                    .ok_or("empty ingredient slot")?;
                let (replacement, used) = repair_flint(
                    &self.0,
                    &item.item,
                    item.count,
                    source,
                    &ingredient.item,
                    ingredient.count,
                )?;
                let projection = mining_projection(&self.0, &item.item, item.count, &replacement)?;
                inventory_transactions::replace_main_hand_combat_profile_consuming(
                    &player.id,
                    &path,
                    source,
                    &replacement,
                    &profile(&projection),
                    projection.damage,
                    projection.max_damage,
                    melee(melee_projection(
                        &self.0,
                        &item.item,
                        item.count,
                        &replacement,
                    )?),
                    slot,
                    used,
                )?;
                return Ok(None);
            }
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
                items_tools::replace_main_hand_combat_profile(
                    &player.id,
                    &path,
                    source,
                    source,
                    &profile(&projection),
                    projection.damage,
                    projection.max_damage,
                    melee(melee_projection(&self.0, &item.item, item.count, source)?),
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
                    items_tools::replace_main_hand_combat_profile(
                        &player.id,
                        &path,
                        source,
                        &replacement,
                        &profile(&projection),
                        projection.damage,
                        projection.max_damage,
                        melee(melee_projection(
                            &self.0,
                            &item.item,
                            item.count,
                            &replacement,
                        )?),
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

/// Plan the supported flint repair; the host must atomically apply consumption.
pub fn repair_flint(
    catalog: &Catalog,
    item: &str,
    count: u32,
    source: &str,
    ingredient: &str,
    available: u32,
) -> Result<(String, u32), String> {
    if item != "minecraft:wooden_pickaxe" || count != 1 || ingredient != "minecraft:flint" {
        return Err("requires a TConstruct tool and flint".into());
    }
    let state = ToolState::from_json(catalog, source)?;
    let plan = tconstruct_horizon::crafting::plan_repair(catalog, &state, "flint", available)?;
    Ok((plan.output.to_json()?, plan.material_items))
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
