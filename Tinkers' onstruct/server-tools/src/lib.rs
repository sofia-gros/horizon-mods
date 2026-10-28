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
                r##"{"width":302,"height":184,"shared_selection":true,"slots":[{"id":"pattern","x":8,"y":43,"role":"storage","accepts":["tconstruct:pattern"],"background":false},{"id":"material","x":29,"y":43,"role":"storage","accepts":["minecraft:flint","#minecraft:planks"],"background":false},{"id":"result","x":148,"y":42,"role":"output","background":false}],"elements":[{"type":"image","x":0,"y":0,"width":176,"height":184,"texture":"tconstruct:gui/part_builder","region":[0,0,176,184]},{"type":"button","id":"head","icon":"tconstruct:gui/tinker_pattern/pick_head","x":51,"y":15,"width":18,"height":18,"text":"","images":{"normal":"tconstruct:gui/part_builder","regions":{"normal":[176,15,18,18],"selected":[176,33,18,18],"hover":[176,51,18,18]}}},{"type":"button","id":"handle","icon":"tconstruct:gui/tinker_pattern/tool_handle","x":69,"y":15,"width":18,"height":18,"text":"","images":{"normal":"tconstruct:gui/part_builder","regions":{"normal":[176,15,18,18],"selected":[176,33,18,18],"hover":[176,51,18,18]}}},{"type":"button","id":"binding","icon":"tconstruct:gui/tinker_pattern/tool_binding","x":87,"y":15,"width":18,"height":18,"text":"","images":{"normal":"tconstruct:gui/part_builder","regions":{"normal":[176,15,18,18],"selected":[176,33,18,18],"hover":[176,51,18,18]}}},{"type":"image","x":176,"y":0,"width":4,"height":4,"texture":"tconstruct:gui/panel","region":[0,0,4,4]},{"type":"image","x":180,"y":0,"width":118,"height":4,"texture":"tconstruct:gui/panel","region":[4,0,118,4]},{"type":"image","x":298,"y":0,"width":4,"height":4,"texture":"tconstruct:gui/panel","region":[122,0,4,4]},{"type":"image","x":176,"y":4,"width":4,"height":176,"texture":"tconstruct:gui/panel","region":[0,4,4,75]},{"type":"image","x":180,"y":4,"width":118,"height":176,"texture":"tconstruct:gui/panel","region":[4,4,118,75]},{"type":"image","x":298,"y":4,"width":4,"height":176,"texture":"tconstruct:gui/panel","region":[122,4,4,75]},{"type":"image","x":176,"y":180,"width":4,"height":4,"texture":"tconstruct:gui/panel","region":[0,79,4,4]},{"type":"image","x":180,"y":180,"width":118,"height":4,"texture":"tconstruct:gui/panel","region":[4,79,118,4]},{"type":"image","x":298,"y":180,"width":4,"height":4,"texture":"tconstruct:gui/panel","region":[122,79,4,4]},{"type":"label","x":183,"y":12,"text":"Pick Head","color":[240,240,240],"when_selected":"head","scale":78},{"type":"label","x":183,"y":12,"text":"Tool Handle","color":[240,240,240],"when_selected":"handle","scale":78},{"type":"label","x":183,"y":12,"text":"Tool Binding","color":[240,240,240],"when_selected":"binding","scale":78},{"type":"label","x":183,"y":31,"text":"Material: {item} x{count}","color":[127,255,255],"slot":"material","scale":78},{"type":"label","x":183,"y":48,"text":"Wood / Flint","color":[240,240,240],"scale":78},{"type":"label","x":183,"y":60,"text":"Other materials","color":[240,240,240],"scale":78},{"type":"label","x":183,"y":70,"text":"not yet supported","color":[240,240,240],"scale":78},{"type":"label","x":183,"y":21,"text":"Cost: 2","color":[255,190,80],"when_selected":"head","scale":78},{"type":"label","x":183,"y":21,"text":"Cost: 1","color":[255,190,80],"when_selected":"handle","scale":78},{"type":"label","x":183,"y":21,"text":"Cost: 1","color":[255,190,80],"when_selected":"binding","scale":78},{"type":"label","x":183,"y":84,"text":"Head durability: 85","color":[240,240,240],"scale":78,"slot":"material","matches":"minecraft:flint"},{"type":"label","x":183,"y":92,"text":"Mining speed: 3.5","color":[240,240,240],"scale":78,"slot":"material","matches":"minecraft:flint"},{"type":"label","x":183,"y":100,"text":"Melee attack: 1.25","color":[240,240,240],"scale":78,"slot":"material","matches":"minecraft:flint"},{"type":"label","x":183,"y":108,"text":"Mining tier: stone","color":[240,240,240],"scale":78,"slot":"material","matches":"minecraft:flint"},{"type":"label","x":183,"y":116,"text":"Handle durability: -15%","color":[240,240,240],"scale":78,"slot":"material","matches":"minecraft:flint"},{"type":"label","x":183,"y":124,"text":"Handle damage: +10%","color":[240,240,240],"scale":78,"slot":"material","matches":"minecraft:flint"},{"type":"label","x":183,"y":132,"text":"Trait: Jagged","color":[240,240,240],"scale":78,"slot":"material","matches":"minecraft:flint"},{"type":"label","x":183,"y":84,"text":"Head durability: 60","color":[240,240,240],"scale":78,"slot":"material","matches":"#minecraft:planks","excludes":"#tconstruct:wood_variants/planks"},{"type":"label","x":183,"y":92,"text":"Mining speed: 2.0","color":[240,240,240],"scale":78,"slot":"material","matches":"#minecraft:planks","excludes":"#tconstruct:wood_variants/planks"},{"type":"label","x":183,"y":100,"text":"Melee attack: 0","color":[240,240,240],"scale":78,"slot":"material","matches":"#minecraft:planks","excludes":"#tconstruct:wood_variants/planks"},{"type":"label","x":183,"y":108,"text":"Mining tier: wood","color":[240,240,240],"scale":78,"slot":"material","matches":"#minecraft:planks","excludes":"#tconstruct:wood_variants/planks"},{"type":"label","x":183,"y":116,"text":"Handle modifiers: 0","color":[240,240,240],"scale":78,"slot":"material","matches":"#minecraft:planks","excludes":"#tconstruct:wood_variants/planks"},{"type":"label","x":183,"y":124,"text":"Trait: Cultivated","color":[240,240,240],"scale":78,"slot":"material","matches":"#minecraft:planks","excludes":"#tconstruct:wood_variants/planks"}],"player_inventory":{"x":8,"y":102},"scroll":{"x":51,"y":15,"columns":4,"rows":4,"cell_width":18,"cell_height":18,"buttons":["head","handle","binding"],"bar_x":126,"bar_y":15,"bar_height":72,"handle_width":12,"handle_height":15,"handle":{"normal":"tconstruct:gui/part_builder","regions":{"normal":[176,0,12,15],"disabled":[188,0,12,15]}}}}"##,
            )?;
            workstations::register_gui(
                "tconstruct:tinker_station",
                "Tinker Station",
                &path,
                &tags,
                r##"{"width":302,"height":184,"slots":[{"id":"tool","x":137,"y":74,"role":"storage"},{"id":"head","x":53,"y":22,"role":"storage"},{"id":"handle","x":15,"y":60,"role":"storage"},{"id":"binding","x":33,"y":42,"role":"storage"},{"id":"result","x":114,"y":38,"role":"output","background":false}],"elements":[{"type":"image","x":0,"y":0,"width":176,"height":184,"texture":"tconstruct:gui/tinker","region":[0,0,176,184]},{"type":"button","id":"repair","x":82,"y":65,"width":18,"height":18,"text":"","icon":"tconstruct:gui/tinker_pattern/button_repair","images":{"normal":"tconstruct:gui/part_builder","regions":{"normal":[176,15,18,18],"selected":[176,33,18,18],"hover":[176,51,18,18]}}},{"type":"button","id":"pickaxe","x":100,"y":65,"width":18,"height":18,"text":"","icon":"tconstruct:gui/tinker_pattern/pickaxe","images":{"normal":"tconstruct:gui/part_builder","regions":{"normal":[176,15,18,18],"selected":[176,33,18,18],"hover":[176,51,18,18]}}},{"type":"image","x":176,"y":0,"width":4,"height":4,"texture":"tconstruct:gui/panel","region":[0,0,4,4]},{"type":"image","x":180,"y":0,"width":118,"height":4,"texture":"tconstruct:gui/panel","region":[4,0,118,4]},{"type":"image","x":298,"y":0,"width":4,"height":4,"texture":"tconstruct:gui/panel","region":[122,0,4,4]},{"type":"image","x":176,"y":4,"width":4,"height":176,"texture":"tconstruct:gui/panel","region":[0,4,4,75]},{"type":"image","x":180,"y":4,"width":118,"height":176,"texture":"tconstruct:gui/panel","region":[4,4,118,75]},{"type":"image","x":298,"y":4,"width":4,"height":176,"texture":"tconstruct:gui/panel","region":[122,4,4,75]},{"type":"image","x":176,"y":180,"width":4,"height":4,"texture":"tconstruct:gui/panel","region":[0,79,4,4]},{"type":"image","x":180,"y":180,"width":118,"height":4,"texture":"tconstruct:gui/panel","region":[4,79,118,4]},{"type":"image","x":298,"y":180,"width":4,"height":4,"texture":"tconstruct:gui/panel","region":[122,79,4,4]},{"type":"label","x":183,"y":12,"text":"Pickaxe","color":[240,240,240],"when_selected":"pickaxe","scale":78},{"type":"label","x":183,"y":24,"text":"Head, Handle, Binding","color":[240,240,240],"when_selected":"pickaxe","scale":78},{"type":"label","x":183,"y":36,"text":"Wood / Flint supported","color":[240,240,240],"when_selected":"pickaxe","scale":78},{"type":"label","x":183,"y":48,"text":"Insert a tool to replace","color":[240,240,240],"when_selected":"pickaxe","scale":78},{"type":"label","x":183,"y":60,"text":"one part.","color":[240,240,240],"when_selected":"pickaxe","scale":78},{"type":"label","x":183,"y":12,"text":"Repair","color":[240,240,240],"when_selected":"repair","scale":78},{"type":"label","x":183,"y":24,"text":"Insert a tool and material","color":[240,240,240],"when_selected":"repair","scale":78},{"type":"label","x":183,"y":36,"text":"Wood / Flint supported","color":[240,240,240],"when_selected":"repair","scale":78},{"type":"label","x":183,"y":48,"text":"Other materials unsupported","color":[240,240,240],"when_selected":"repair","scale":78},{"type":"label","x":183,"y":78,"text":"Head: {item} x{count}","color":[127,255,255],"when_selected":"pickaxe","slot":"head","scale":78},{"type":"label","x":183,"y":89,"text":"Handle: {item} x{count}","color":[127,255,255],"when_selected":"pickaxe","slot":"handle","scale":78},{"type":"label","x":183,"y":100,"text":"Binding: {item} x{count}","color":[127,255,255],"when_selected":"pickaxe","slot":"binding","scale":78},{"type":"label","x":183,"y":65,"text":"Tool: {item}","color":[127,255,255],"when_selected":"repair","slot":"tool","scale":78}],"player_inventory":{"x":8,"y":102,"background":false},"shared_selection":true,"layouts":[{"selection":"repair","slots":[{"id":"tool","x":53,"y":41},{"id":"head","x":11,"y":41},{"id":"handle","x":31,"y":30},{"id":"binding","x":31,"y":50}]}],"text_input":{"x":80,"y":7,"width":82,"height":9,"max_length":50,"when_selected":"pickaxe","output":"result","empty_slot":"tool"}}"##,
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
                Part, PartKind, plan_part, plan_part_replacement, plan_pickaxe, plan_repair,
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
            let [tool, head, handle, binding, None] = inputs else {
                return Ok(None);
            };
            let parse = |s: &WorkstationInput| -> Option<Part> {
                serde_json::from_str(s.custom_text.as_deref()?).ok()
            };
            let (state, consume, source) = if let Some(tool) = tool {
                if tool.item != "minecraft:wooden_pickaxe" || tool.count != 1 {
                    return Ok(None);
                }
                let Some(text) = tool.custom_text.as_deref() else {
                    return Ok(None);
                };
                let Ok(state) = ToolState::from_json(&self.0, text) else {
                    return Ok(None);
                };
                match (selection, head, handle, binding) {
                    (Some("repair"), Some(ingredient), None, None) => {
                        let Some(name) = material(ingredient) else {
                            return Ok(None);
                        };
                        let Ok(plan) = plan_repair(&self.0, &state, name, ingredient.count) else {
                            return Ok(None);
                        };
                        (plan.output, vec![1, plan.material_items, 0, 0, 0], Some(0))
                    }
                    (Some("pickaxe"), head, handle, binding) => {
                        let choices = [head, handle, binding];
                        let Some((index, part)) = choices
                            .iter()
                            .enumerate()
                            .find_map(|(i, v)| v.as_ref().map(|p| (i, p)))
                        else {
                            return Ok(None);
                        };
                        if choices.iter().filter(|v| v.is_some()).count() != 1 {
                            return Ok(None);
                        }
                        let expected = [
                            "tconstruct:pick_head",
                            "tconstruct:tool_handle",
                            "tconstruct:tool_binding",
                        ][index];
                        if part.item != expected {
                            return Ok(None);
                        }
                        let Some(part) = parse(part) else {
                            return Ok(None);
                        };
                        if part.kind != [PartKind::Head, PartKind::Handle, PartKind::Binding][index]
                        {
                            return Ok(None);
                        }
                        let Ok(result) = plan_part_replacement(&self.0, &state, &part) else {
                            return Ok(None);
                        };
                        let mut consume = vec![1, 0, 0, 0, 0];
                        consume[index + 1] = 1;
                        (result, consume, Some(0))
                    }
                    _ => return Ok(None),
                }
            } else {
                if selection != Some("pickaxe") {
                    return Ok(None);
                }
                let (Some(first), Some(second), Some(third)) = (head, handle, binding) else {
                    return Ok(None);
                };
                if first.item != "tconstruct:pick_head"
                    || second.item != "tconstruct:tool_handle"
                    || third.item != "tconstruct:tool_binding"
                {
                    return Ok(None);
                }
                let (Some(a), Some(b), Some(c)) = (parse(first), parse(second), parse(third))
                else {
                    return Ok(None);
                };
                let Ok(state) = plan_pickaxe(&self.0, &[a, b, c]) else {
                    return Ok(None);
                };
                (state, vec![0, 1, 1, 1, 0], None)
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
