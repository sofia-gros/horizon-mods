//! Connects the modules that hear a block the player's tool breaks to PlutoLib's block-break and loot events: mining capacity (the bar a
//! break fills or drains, before the break with `run_before` and after it otherwise: BlockHarvestModifierHook), loot to capacity (the loot of
//! the block, eaten into a bar: ProcessLootModifierHook), autosmelt (drops a cooking recipe takes become their cooked result, on the same
//! hook), the harvest enchantments the breaker's armor lends the breaking stack
//! (HarvestEnchantmentsModifierHook: merged into the stack before the break, restored after) and the block-break half of tool usage
//! effects (MobEffectModule.ToolUsage's afterBlockBreak). The edits land as `EntityAction::EditStackData` on the main hand. An area
//! tool's extra blocks ride the same before-break callback through `break_extra` (see area_break). The mining half of the original's
//! render event — the neighbours the held tool is effective on — is outlined through the highlight provider (see highlight).
use crate::{break_speed_hook::with_held_tool, common, melting_recipes::{MeltingKind, OreRate}, tool_damage_hook, tool_mining, tool_slot, tool_stack, weapon_effect::Usage, weapon_effect_hook::usage_actions};
use pluto_lib::{highlight_api_v1::{HighlightFacts, HighlightRegistry}, ActionTarget, BlockBreakContext, Direction, EntityAction, FluidStack, Identifier, ItemSlot, LootTableEvents, Nbt, PlayerBlockBreakEvents, StackData, StackDataEdit, StackSlot, World};
use std::sync::Mutex;

/// Whether the tool that broke the block at (dimension, position) was effective on it, noted by the
/// before-break callback for the drops callback: at drops time the block is already air, so its state
/// cannot be read back. MeltingModule's force melting needs exactly this fact. One break at a time
/// runs on the server thread, as with loot_modifier_hook's broken-block note.
static MELT_EFFECTIVE: Mutex<Option<(String, [i32; 3], bool)>> = Mutex::new(None);

/// Notes the held tool's effectiveness on the block about to break (the same test area_break runs).
fn note_melt_effectiveness(broken: &BlockBreakContext) {
    let effective = (|| {
        let held = broken.held.as_ref()?;
        let state = broken.state.as_ref()?;
        with_held_tool(held, |env, stack| {
            let tier = tool_stack::harvest_tier(&stack.custom_data);
            env.definition.and_then(|definition| definition.effective())
                .is_some_and(|effective| tool_mining::is_effective(effective, &tier, &|tag: &Identifier| state.is_in_tag(&tag.to_string())))
        })
        .ok()
    })()
    .unwrap_or(false);
    if let Ok(mut noted) = MELT_EFFECTIVE.lock() { *noted = Some((broken.dimension.clone(), broken.pos, effective)); }
}

/// Takes the noted effectiveness for the block whose drops these are; false when nothing was noted.
fn take_melt_effective(dimension: &str, block: Option<[i32; 3]>) -> bool {
    let Ok(mut noted) = MELT_EFFECTIVE.lock() else { return false; };
    match (&*noted, block) {
        (Some((noted_dimension, pos, effective)), Some(pos_drops)) if noted_dimension == dimension && *pos == pos_drops => {
            let effective = *effective;
            *noted = None;
            effective
        }
        _ => false,
    }
}

/// MeltingModule's recipe answer for one dropped stack: the item's melting recipe, its output scaled
/// by the module's own ore rates (an ore recipe) or the foundry damage scale (a damageable one), and
/// the recipe's temperature. Byproducts are the melter's business, not the tool tank's.
fn melt_drop(slot: &pluto_lib::ItemSlot, module: &crate::tool_tank::Melting, level: f32, in_tag: &dyn Fn(&Identifier, &Identifier) -> bool) -> Option<(FluidStack, i32)> {
    let data = common::smeltery_data()?.read().ok()?;
    let recipe = data.recipes.find_melting(&slot.item, in_tag, false)?;
    let amount = match &recipe.kind {
        MeltingKind::Ore { rate, .. } => module.ore_boost(level, *rate, recipe.output.amount),
        MeltingKind::Damageable { .. } => recipe.output_amount(&OreRate::FOUNDRY, slot.max_damage.map(|max| (slot.damage as i32, max))),
        MeltingKind::Plain => recipe.output.amount,
    };
    let fluid = recipe.output.resolve(&data.tags)?;
    let tag = recipe.output.nbt.as_ref().and_then(crate::fluid_world::value_to_nbt);
    Some((FluidStack { fluid, amount: amount.max(0) as u32, tag }, recipe.temperature))
}

/// The action that puts `edits` on the stack of the tool in the main hand; None when there is nothing to change.
fn edit_action(held: &StackData, edits: Vec<StackDataEdit>) -> Option<EntityAction> {
    (!edits.is_empty()).then(|| EntityAction::EditStackData { target: ActionTarget::Attacker, slot: StackSlot::MainHand, item: held.item.clone(), edits })
}

/// The enchantments a breaking stack carried before the harvest merge, for one break at a time
/// (the breaker, the block, the stack's item and data, and its own enchantments): the after-break
/// callback puts them back, where the original restores them as soon as the drops have rolled —
/// here the restore waits for the after-break telling, a tick late at most.
static HARVEST_MERGED: Mutex<Option<(u128, [i32; 3], String, Nbt, std::collections::BTreeMap<String, i32>)>> = Mutex::new(None);

/// HarvestEnchantmentsModifierHook.updateHarvestEnchantments: before the break, the enchantments
/// the breaker's worn pieces lend (their armor harvest enchantment modules, in the slots they
/// name) are merged into the breaking stack's own enchantments, so the block's loot roll sees
/// them. The merge runs only for a modifiable breaking tool, and a piece that is itself a
/// harvest tool lends nothing (the original's valid-tool test); levels add, and a total of 0 or
/// less is no enchantment, as the original's merger drops it.
fn merge_harvest_enchantments(broken: &BlockBreakContext) {
    let (Some(player), Some(held)) = (broken.player.as_ref(), broken.held.as_ref()) else { return; };
    if broken.equipment.is_empty() || with_held_tool(held, |_, _| ()).is_err() { return; }
    let items = common::item_snapshot();
    let harvest_tag = Identifier::new("tconstruct", "modifiable/harvest").expect("valid id");
    let mut merged = held.enchantments.clone();
    let mut lent = false;
    for (slot, piece) in &broken.equipment {
        let piece_item = Identifier::parse(&piece.item).ok();
        if piece_item.as_ref().is_some_and(|item| items.as_ref().is_some_and(|items| items.get_tag(&harvest_tag).is_some_and(|members| members.contains(item)))) { continue; }
        let Ok(grants) = with_held_tool(piece, |env, stack| tool_stack::armor_harvest_enchantments(&stack.custom_data, env, slot)) else { continue; };
        for (name, level) in grants {
            *merged.entry(name).or_insert(0) += level;
            lent = true;
        }
    }
    if !lent { return; }
    merged.retain(|_, level| *level > 0);
    if merged == held.enchantments { return; }
    let Some(slot) = player.selected_slot() else { return; };
    let mut stack = held.clone();
    stack.enchantments = merged;
    if player.update_stack(slot, &stack) {
        if let Ok(mut noted) = HARVEST_MERGED.lock() {
            *noted = Some((player.uuid, broken.pos, held.item.clone(), Nbt::Compound(held.custom_data.clone()), held.enchantments.clone()));
        }
    }
}

/// Puts the breaking stack's own enchantments back after a merge (see HARVEST_MERGED): only the
/// enchantments change, on the stack as the break left it, and only when the main hand still
/// holds the tool that was merged.
fn restore_harvest_enchantments(broken: &BlockBreakContext) {
    let Some(player) = broken.player.as_ref() else { return; };
    let taken = HARVEST_MERGED.lock().ok().and_then(|mut noted| {
        let matches = matches!(&*noted, Some((uuid, pos, _, _, _)) if *uuid == player.uuid && *pos == broken.pos);
        if matches { noted.take() } else { None }
    });
    let Some((_, _, item, custom_data, original)) = taken else { return; };
    let Some(held) = broken.held.as_ref() else { return; };
    if held.item != item || Nbt::Compound(held.custom_data.clone()) != custom_data { return; }
    let slot = (0..36).find(|slot| player.stack(*slot).is_some_and(|stack| stack.item == item && Nbt::Compound(stack.custom_data.clone()) == custom_data));
    let Some(slot) = slot else { return; };
    let mut stack = held.clone();
    stack.enchantments = original;
    player.update_stack(slot, &stack);
}

/// ToolHarvestLogic.runBlockBreak's area half: when the tool is effective on the block being broken, its
/// definition's AOE iterator (computed now, before the break, as the original computes it) selects the
/// extra blocks and the break takes them with it. Box and circle candidates still owe the effectiveness
/// half of the default block predicate — vein and tree select by their own block rules, as their
/// getBlocks takes no predicate — and air is skipped, as the original skips it. The predicate's hardness
/// clause has no surface here (see aoe.rs). A break whose face the game did not report has no plane to
/// expand over, so it stays single-block.
fn area_break(broken: &BlockBreakContext) {
    let (Some(held), Some(face), Some(state)) = (broken.held.as_ref(), broken.face, broken.state.as_ref()) else { return; };
    let Some(world) = World::ticking() else { return; };
    let Ok(positions) = with_held_tool(held, |env, stack| {
        let Some(iterator) = env.definition.and_then(|definition| definition.aoe()) else { return Vec::new(); };
        let tier = tool_stack::harvest_tier(&stack.custom_data);
        let effective_on = |state: &pluto_lib::BlockState| env.definition.and_then(|definition| definition.effective())
            .is_some_and(|effective| tool_mining::is_effective(effective, &tier, &|tag: &Identifier| state.is_in_tag(&tag.to_string())));
        if !effective_on(state) { return Vec::new(); }
        let expanded = match stack.custom_data.get(tool_stack::TAG_VOLATILE) {
            Some(Nbt::Compound(volatile)) => crate::modifier_entries::nbt_int(volatile.get("tconstruct:expanded")),
            _ => 0,
        };
        let block_at = |pos: [i32; 3]| world.get_block_state(pos).map(|state| state.block.clone());
        let in_tag_at = |pos: [i32; 3], tag: &Identifier| world.get_block_state(pos).is_some_and(|state| state.is_in_tag(&tag.to_string()));
        let positions = iterator.positions(broken.pos, face, broken.yaw, broken.pitch, expanded, &block_at, &in_tag_at);
        if matches!(iterator, crate::aoe::Iterator::Box { .. } | crate::aoe::Iterator::Circle { .. }) {
            positions.into_iter().filter(|pos| world.get_block_state(*pos).as_ref().is_some_and(|state| state.block.to_string() != "minecraft:air" && effective_on(state))).collect()
        } else {
            positions
        }
    }) else { return; };
    for pos in positions { broken.break_extra(pos); }
}

/// What the exchanging modifier does with one broken block: the state to place and how the provider is consumed.
#[derive(Debug, PartialEq)]
struct Exchange {
    state: pluto_lib::BlockState,
    consume: ExchangeConsume,
}

/// How one placement consumes its provider: a block item shrinks by one in the off hand; a tool takes its provider module's wear in the hand that provided it.
#[derive(Debug, PartialEq)]
enum ExchangeConsume {
    ShrinkOffhand,
    DamageOffhand { modifier: Identifier, wear: i32 },
    DamageMainHand { modifier: Identifier, wear: i32 },
}

/// The exchange for one broken block, decided from the tool and the hands alone: the state to place and how the provider is consumed.
/// `carries` is whether the breaking tool carries exchanging; `offhand` the off-hand stack; `offhand_provider`/`mainhand_provider` are the
/// tools' block_item_provider answers (providing modifier, item, wear). Off hand first, then the breaking tool — the original's order.
/// A plain block item provides itself when the block registry snapshot names its id as a block (the original's `BlockItem` instance
/// test, answered by the registry the world was built from); without a block capture the item snapshot is the mirror, as it was before
/// the block registry was captured — an item it names whose id names no block answers a state the host refuses to place, so the
/// placement fails and nothing is consumed. A tool only ever provides through its provider module, and the module's load validation
/// is what guarantees its item is a BlockItem. None when the tool carries no exchanging or nothing provides a block.
/// The adventure gate runs last, on the chosen provider, as the original checks it on the provided stack before placing:
/// a breaker who may not build (`may_build` false) places only when the provided stack's place tag names the mined block.
/// The port's provider stacks carry no place tag (their module data names a bare item), so only the plain off-hand stack —
/// whose tag the break context reports as `place_on` — can pass it. When the context also carries the tag's structured
/// predicates (the full `can_place_on` read, blocks and state ranges and block-entity NBT included), those answer the gate
/// instead of the flattened text entries, as they are the same tag read without anything dropped.
fn exchange_decision(
    carries: bool,
    offhand: Option<&StackData>,
    offhand_provider: Option<(Identifier, Identifier, i32)>,
    mainhand_provider: Option<(Identifier, Identifier, i32)>,
    blocks: Option<&pluto_lib::RegistryLookup>,
    items: Option<&pluto_lib::RegistryLookup>,
    may_build: bool,
    mined: Option<&pluto_lib::BlockState>,
    place_on: &[String],
    place_predicates: &[pluto_lib::AdventurePlacePredicate],
    mined_block_entity: Option<&pluto_lib::NbtCompound>,
) -> Option<Exchange> {
    if !carries {
        return None;
    }
    let mut exchange = None;
    if let Some(stack) = offhand.filter(|stack| stack.count > 0) {
        if let Some((modifier, item, wear)) = offhand_provider {
            exchange = Some(Exchange { state: pluto_lib::BlockState::new(item), consume: ExchangeConsume::DamageOffhand { modifier, wear } });
        } else if !stack.custom_data.contains_key(tool_stack::TAG_MATERIALS) {
            if let Ok(item) = Identifier::parse(&stack.item) {
                let provides = match blocks {
                    Some(blocks) => blocks.contains(&item),
                    None => items.is_some_and(|items| items.contains(&item)),
                };
                if provides {
                    exchange = Some(Exchange { state: pluto_lib::BlockState::new(item), consume: ExchangeConsume::ShrinkOffhand });
                }
            }
        }
    }
    let exchange = match exchange {
        Some(exchange) => exchange,
        None => {
            let (modifier, item, wear) = mainhand_provider?;
            Exchange { state: pluto_lib::BlockState::new(item), consume: ExchangeConsume::DamageMainHand { modifier, wear } }
        }
    };
    if !may_build {
        let allowed = matches!(exchange.consume, ExchangeConsume::ShrinkOffhand)
            && mined.is_some_and(|mined| {
                if place_predicates.is_empty() {
                    adventure_place_allowed(place_on, mined, blocks)
                } else {
                    pluto_lib::place_predicates_allow(place_predicates, mined, mined_block_entity, blocks)
                }
            });
        if !allowed {
            return None;
        }
    }
    Some(exchange)
}

/// `ItemStack.hasAdventureModePlaceTagForBlock` for the block being mined: one of the place tag's entries names it — a
/// block id (with an optional state suffix whose properties the mined state must carry) or a `#tag` the block registry
/// snapshot resolves. An entry that does not parse names nothing, and a `#tag` names nothing without the block snapshot.
fn adventure_place_allowed(entries: &[String], mined: &pluto_lib::BlockState, blocks: Option<&pluto_lib::RegistryLookup>) -> bool {
    entries.iter().any(|entry| {
        if let Some(tag) = entry.strip_prefix('#') {
            let Ok(tag) = Identifier::parse(tag) else { return false; };
            return blocks.and_then(|blocks| blocks.get_tag(&tag)).is_some_and(|members| members.contains(&mined.block));
        }
        let Ok(wanted) = pluto_lib::BlockState::parse(entry) else { return false; };
        wanted.block == mined.block && wanted.properties.iter().all(|(key, value)| mined.properties.get(key) == Some(value))
    })
}

/// Whether the tool's item is in the durability tag (a Tinkers tool is); true when the snapshot is missing, as the wear path assumes.
fn durability_tagged(item: &Identifier) -> bool {
    let Some(items) = common::item_snapshot() else { return true; };
    let tag = Identifier::new("tconstruct", "modifiable/durability").expect("valid id");
    items.get_tag(&tag).is_some_and(|members| members.contains(item))
}

/// A slot as the player API's stack data (the write-back form of a damaged tool).
fn stack_data_of_slot(slot: &pluto_lib::ItemSlot) -> StackData {
/// The AOE_INTERACTION tag's members (tconstruct/content/data/tconstruct/tinkering/tags/modifiers/aoe_interaction.json):
/// the modifiers whose presence makes the original's render event draw the highlight even when the
/// tool is not effective on the aimed block.
const AOE_INTERACTION: [&str; 6] = ["tconstruct:pathing", "tconstruct:stripping", "tconstruct:tilling", "tconstruct:brushing", "tconstruct:splashing", "tconstruct:harvest"];

    StackData {
        item: slot.item.to_string(),
        count: slot.count,
        damage: slot.damage,
        max_damage: slot.max_damage.map(|max| u32::try_from(max).unwrap_or(0)),
        custom_data: slot.custom_data.clone(),
        custom_model_data: slot.custom_model_data.clone(),
        enchantments: slot.enchantments.clone().unwrap_or_default(),
    }
}

/// Consumes the provider of one exchange, on the stacks the player holds now (the decision's stacks are a tick old, so the live stack
/// must still be the same tool before it is worn, as the harvest-merge restore does). A block item shrinks in the off hand; a tool
/// takes its provider's wear on ToolDamageUtil.damageAnimated's reduced path, with the providing modifier as the cause. A wear that
/// fails to apply leaves the tool unworn rather than eating it. A creative holder consumes nothing: `BlockItem.place` does not
/// shrink the stack of a player with instabuild, so the original's fake stack never empties and its consume is never called — no
/// shrink, no wear (ToolDamageUtil's own `consumesResources` gate agrees). The damage calls below therefore pass the original's
/// `true`: by the time they run, the holder is known not to be creative.
fn apply_exchange_consume(player: &pluto_lib::ServerPlayer, held: &StackData, offhand: Option<&StackData>, exchange: &Exchange, creative: bool) {
    if creative {
        return;
    }
    match &exchange.consume {
        ExchangeConsume::ShrinkOffhand => {
            let Some(stack) = offhand.filter(|stack| stack.count > 0) else { return; };
            let live = player.stack(-106).or_else(|| offhand.cloned());
            let Some(live) = live.filter(|live| live.item == stack.item && live.count > 0) else { return; };
            if live.count <= 1 {
                player.set_equipment(StackSlot::OffHand, None);
            } else {
                let mut shrunk = live;
                shrunk.count -= 1;
                player.set_equipment(StackSlot::OffHand, Some(&shrunk));
            }
        }
        ExchangeConsume::DamageOffhand { modifier, wear } => {
            let Some(stack) = offhand else { return; };
            let live = player.stack(-106).or_else(|| offhand.cloned());
            let Some(live) = live.filter(|live| live.item == stack.item && Nbt::Compound(live.custom_data.clone()) == Nbt::Compound(stack.custom_data.clone())) else { return; };
            let damaged = with_held_tool(&live, |env, slot| {
                let mut slot = slot.clone();
                let broke = tool_slot::damage_slot(&mut slot, env, *wear, true, durability_tagged(&slot.item), Some(modifier), &mut tool_damage_hook::random).ok()?.broke;
                Some((stack_data_of_slot(&slot), broke))
            })
            .ok()
            .flatten();
            if let Some((damaged, broke)) = damaged {
                // The wear landed on the stack the host still holds only once the write-back succeeds; the
                // break broadcast (the sound and crumbs of damageAnimated) follows it, and only when the
                // wear actually broke the stack — the host's own wear already broadcasts itself.
                if player.set_equipment(StackSlot::OffHand, Some(&damaged)) && broke {
                    player.broadcast_item_break(StackSlot::OffHand, &damaged);
                }
            }
        }
        ExchangeConsume::DamageMainHand { modifier, wear } => {
            let Some(selected) = player.selected_slot() else { return; };
            let Some(live) = player.stack(selected).filter(|live| live.item == held.item && Nbt::Compound(live.custom_data.clone()) == Nbt::Compound(held.custom_data.clone())) else { return; };
            let damaged = with_held_tool(&live, |env, slot| {
                let mut slot = slot.clone();
                let broke = tool_slot::damage_slot(&mut slot, env, *wear, true, durability_tagged(&slot.item), Some(modifier), &mut tool_damage_hook::random).ok()?.broke;
                Some((stack_data_of_slot(&slot), broke))
            })
            .ok()
            .flatten();
            if let Some((damaged, broke)) = damaged {
                if player.update_stack(selected, &damaged) && broke {
                    player.broadcast_item_break(StackSlot::MainHand, &damaged);
                }
            }
        }
    }
}

/// ExchangingModifier.removeBlock, after the break: the break already rolled its drops (and its XP), so the replacement lands here, in
/// the air the break left. A veto in the before callback would have kept the block and lost the drops, which the
/// original harvests; and the after telling only fires for a block that actually changed, so the glow and fire vetoes (which keep their
/// block) never reach here, whatever their priority. The timing is the after telling's own, and it is the one point where this
/// differs from the original observably: ToolHarvestLogic.breakBlock runs the REMOVE_BLOCK hooks synchronously, before the block
/// is removed, so the original's swap never shows air; the game's own break now reports its completion to the bridge (see the
/// break-completion seam), so this telling runs inside the break, before the removal is sent to the players around — the position
/// stands open only for the calls between the removal and the return, not until the breaking player's next tick. What else the port
/// does not carry: the neighbour packet (UpdateNeighborsPacket). The host names no client verb to recompute a block's neighbour
/// shapes; the packet's work — refreshing the old state's connection shapes on the breaking player's client — has no stale shape to
/// fix here, as the host's client derives connection shapes from the block states when it remeshes after the synced block change.
/// And the slab orientation of BlockItem.place (the state is placed as its default, as the fluid place-block effect does). The tool
/// is read as the break left it, as the after-break telling does. The off hand is read live first (a plain block item carries no
/// custom data, so the tick memory never names it); the captured equipment is the fallback for a tool provider. The two environment
/// builds run sequentially, never nested, so no registry lock is ever taken twice.
fn exchange_after(broken: &BlockBreakContext) {
    let (Some(player), Some(held)) = (broken.player.as_ref(), broken.held.as_ref()) else { return; };
    // Another placement has taken the air since the break; never overwrite it.
    let world = World::new(&broken.dimension);
    if world.get_block_state(broken.pos).is_some_and(|now| now.block.to_string() != "minecraft:air") {
        return;
    }
    let offhand = player.stack(-106).or_else(|| broken.equipment.iter().find(|(slot, _)| slot == "offhand").map(|(_, stack)| stack.clone()));
    let blocks = common::block_snapshot();
    let items = common::item_snapshot();
    let offhand_provider = offhand.as_ref().and_then(|stack| {
        with_held_tool(stack, |env, slot| tool_stack::block_item_provider(&slot.custom_data, env)).ok().flatten()
    });
    let exchanging = Identifier::new("tconstruct", "exchanging").expect("valid id");
    let Some(exchange) = with_held_tool(held, |env, slot| {
        let carries = tool_stack::saved_modifiers(&slot.custom_data, env.modifiers).iter().any(|(entry, _)| entry.id == exchanging);
        let mainhand_provider = tool_stack::block_item_provider(&slot.custom_data, env);
        exchange_decision(carries, offhand.as_ref(), offhand_provider, mainhand_provider, blocks.as_deref(), items.as_deref(), broken.may_build, broken.state.as_ref(), &broken.offhand_place_on, &broken.offhand_place_predicates, broken.mined_block_entity.as_ref())
    })
    .ok()
    .flatten() else {
        return;
    };
    // A failed placement consumes nothing, as the original consumes only what its fake stack spent.
    if !world.set_block_state(broken.pos, &exchange.state) {
        return;
    }
    apply_exchange_consume(player, held, offhand.as_ref(), &exchange, broken.creative);
    // The place animation: the original swings the off hand once the placement lands, whether or not the
    // holder was creative — the swing is the placement's, not the consume's.
    player.swing_hand(pluto_lib::Hand::Off);
}

/// The mining half of the original's render event over the AOE iterator's DISPLAY predicate
/// (ToolRenderEvents.renderBlockHighlights): while the player holds a mining tool and aims at a
/// block, the neighbours the tool is effective on are outlined — the predicate's other half to the
/// brush's shouldHighlight answers. The gate is the render event's own: nothing is outlined when
/// the tool is effective on nothing at the aim and carries no AOE-interaction modifier (pathing,
/// stripping, tilling, brushing, splashing, harvest), and the distance gate is the frame's block
/// reach, as the brush's is. Box and circle candidates are filtered to the effective ones, as the
/// break hook filters its area; vein, tree and conditional positions ride as the break hook takes
/// them, so only when the tool is effective on the aimed block itself — otherwise a brush tool's
/// vein would outline blocks its stroke never breaks. The hardness clause of the predicate (a
/// neighbour more than three times as hard as the aim, or unbreakable) has no surface here, as in
/// the break hook (see aoe.rs): the outlined set is the break set.
fn highlight(facts: &HighlightFacts<'_>) -> Vec<[i32; 3]> {
    let Some(slot) = facts.main_hand else { return Vec::new() };
    let Some(target) = &facts.target else { return Vec::new() };
    if target.distance > facts.block_reach { return Vec::new() };
    let stack = stack_data_of_slot(slot);
    let Ok(positions) = with_held_tool(&stack, |env, tool| {
        let Some(iterator) = env.definition.and_then(|definition| definition.aoe()) else { return Vec::new() };
        let tier = tool_stack::harvest_tier(&tool.custom_data);
        let effective = env.definition.and_then(|definition| definition.effective());
        let effective_on = |pos: [i32; 3]| effective.is_some_and(|effective| tool_mining::is_effective(effective, &tier, &|tag: &Identifier| (facts.in_tag_at)(pos, tag)));
        let has_aoe_interaction = tool_stack::saved_modifiers(&tool.custom_data, env.modifiers).iter()
            .any(|(entry, _)| entry.level > 0 && AOE_INTERACTION.contains(&entry.id.to_string().as_str()));
        if !effective_on(target.pos) && !has_aoe_interaction { return Vec::new() };
        let expanded = match tool.custom_data.get(tool_stack::TAG_VOLATILE) {
            Some(Nbt::Compound(volatile)) => crate::modifier_entries::nbt_int(volatile.get("tconstruct:expanded")),
            _ => 0,
        };
        let face = Direction::from_offset(target.face).unwrap_or(Direction::Up);
        let positions = iterator.positions(target.pos, face, facts.yaw, facts.pitch, expanded, facts.block_at, facts.in_tag_at);
        if matches!(iterator, crate::aoe::Iterator::Box { .. } | crate::aoe::Iterator::Circle { .. }) {
            positions.into_iter().filter(|pos| (facts.block_at)(*pos).is_some_and(|block| block.to_string() != "minecraft:air") && effective_on(*pos)).collect()
        } else if effective_on(target.pos) {
            positions
        } else {
            Vec::new()
        }
    }) else { return Vec::new() };
    positions
}

/// Registers the callbacks. Called once from the product's hooks.
pub(crate) fn register() {
    // The mining half of the original's render event: the neighbours the held tool is effective on.
    HighlightRegistry::register(highlight);
    // startHarvest: the modules that run before the break (overwield spends overslime as the tool starts to dig). Runs even when a later
    // callback vetoes the break, as the original runs it before the break is attempted.
    PlayerBlockBreakEvents::BEFORE.register(|broken| {
        if let Some(held) = broken.held.as_ref() {
            if let Ok(edits) = with_held_tool(held, |env, stack| tool_stack::mining_capacity_edits(&stack.custom_data, env, true)) {
                broken.actions.borrow_mut().extend(edit_action(held, edits));
            }
        }
        area_break(broken);
        note_melt_effectiveness(broken);
        merge_harvest_enchantments(broken);
        true
    });
    // PlaceGlowModule.removeBlock: a glow block is not removed by a left-click-interactable tool
    // carrying the module (unbroken), so the click that follows a placement does not instantly break
    // the glow just placed. The definition's interaction gate (canInteract for the left click) is not
    // consulted, as nowhere in this port (see tool_definitions).
    PlayerBlockBreakEvents::BEFORE.register(|broken| {
        let (Some(held), Some(state)) = (broken.held.as_ref(), broken.state.as_ref()) else { return true; };
        if state.block.to_string() != "tconstruct:glow" { return true; }
        let Some(items) = common::item_snapshot() else { return true; };
        let left = Identifier::new("tconstruct", "modifiable/interactable/left").expect("valid id");
        let Ok(item) = Identifier::parse(&held.item) else { return true; };
        if !items.get_tag(&left).is_some_and(|members| members.contains(&item)) { return true; }
        let carries = with_held_tool(held, |env, stack| !tool_stack::is_broken(&stack.custom_data) && !tool_stack::place_glow(&stack.custom_data, env).is_empty()).unwrap_or(false);
        !carries
    });
    // PlaceFireModule.removeBlock: a fire block is not removed by a left-click-interactable tool
    // carrying the module (unbroken), so a left click on fire reaches the interaction instead of
    // putting the fire out. The original names Blocks.FIRE alone — soul fire is not protected and
    // breaks as usual. The definition's interaction gate (canInteract for the left click) is not
    // consulted, as nowhere in this port (see tool_definitions).
    PlayerBlockBreakEvents::BEFORE.register(|broken| {
        let (Some(held), Some(state)) = (broken.held.as_ref(), broken.state.as_ref()) else { return true; };
        if state.block.to_string() != "minecraft:fire" { return true; }
        let Some(items) = common::item_snapshot() else { return true; };
        let left = Identifier::new("tconstruct", "modifiable/interactable/left").expect("valid id");
        let Ok(item) = Identifier::parse(&held.item) else { return true; };
        if !items.get_tag(&left).is_some_and(|members| members.contains(&item)) { return true; }
        let carries = with_held_tool(held, |env, stack| !tool_stack::is_broken(&stack.custom_data) && tool_stack::place_fire(&stack.custom_data, env)).unwrap_or(false);
        !carries
    });
    // finishHarvest (one block harvested) and afterBlockBreak: the capacity modules that run after the break and the effects the tool puts on
    // its holder. The tool is read as the break left it.
    PlayerBlockBreakEvents::AFTER.register(|broken| {
        restore_harvest_enchantments(broken);
        let Some(held) = broken.held.as_ref() else { return };
        if let Ok(edits) = with_held_tool(held, |env, stack| tool_stack::mining_capacity_edits(&stack.custom_data, env, false)) {
            broken.actions.borrow_mut().extend(edit_action(held, edits));
        }
        // finishHarvest: a harvest tool's smelting modules cook by the one block harvested.
        if let Some(items) = common::item_snapshot() {
            let harvest_tag = Identifier::new("tconstruct", "modifiable/harvest").expect("valid id");
            let is_harvest = Identifier::parse(&held.item).is_ok_and(|item| items.get_tag(&harvest_tag).is_some_and(|members| members.contains(&item)));
            if is_harvest {
                if let Ok(edits) = with_held_tool(held, |env, stack| tool_stack::smelting_edits(&stack.custom_data, env, 1.0)) {
                    broken.actions.borrow_mut().extend(edit_action(held, edits));
                }
            }
        }
        broken.actions.borrow_mut().extend(usage_actions(&[&broken.held], Usage::BlockBreak));
    });
    // ExchangingModifier.removeBlock, after the break (see exchange_after): the break already rolled its drops, so the replacement lands here.
    PlayerBlockBreakEvents::AFTER.register(|broken| exchange_after(broken));
    // processLoot: the loot of a block a player's tool broke. ModifierLootModifier runs it for tools of the loot capable tag that are not broken.
    LootTableEvents::MODIFY_DROPS.register(|drops| {
        let (Some(tool), Some(_), true) = (drops.tool.as_ref(), drops.player, drops.block.is_some()) else { return };
        let Some(items) = common::item_snapshot() else { return };
        let in_tag = |item: &Identifier, tag: &Identifier| items.get_tag(tag).is_some_and(|members| members.contains(item));
        let (Ok(item), Ok(capable)) = (Identifier::parse(&tool.item), Identifier::new("tconstruct", "modifiable/loot_capable_tool")) else { return };
        if !in_tag(&item, &capable) { return; }
        let mut loot = drops.drops.borrow_mut();
        let eaten = with_held_tool(tool, |env, stack| tool_stack::loot_capacity(&stack.custom_data, env, &mut loot, &in_tag, &mut tool_damage_hook::random));
        if let Ok(edits) = eaten { drops.actions.borrow_mut().extend(edit_action(tool, edits)); }
        // processLoot's melting half: drops the tool's melting modules can melt become tank fluid.
        // Force melting (the volatile flag is set) counts only when the tool was effective on the
        // block, which the before-break callback noted.
        let melted = with_held_tool(tool, |env, stack| {
            let force = tool_stack::volatile_flag(&stack.custom_data, "tconstruct:force_melting") && take_melt_effective(&drops.dimension, drops.block);
            tool_stack::melting_loot(&mut stack.custom_data.clone(), env, &mut loot, force, &|slot, module, level| melt_drop(slot, module, level, &in_tag))
        });
        if let Ok(edits) = melted { drops.actions.borrow_mut().extend(edit_action(tool, edits)); }
        // processLoot's autosmelt half: drops a cooking recipe takes become their cooked result.
        let _ = with_held_tool(tool, |env, stack| tool_stack::autosmelt_loot(&stack.custom_data, env, &mut loot, &in_tag, &mut tool_damage_hook::random));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{MemoryPlayerOps, NbtCompound, RegistryLookup, ServerPlayer, with_ops};
    use serde_json::json;

    fn stack(item: &str, count: u32) -> StackData {
        StackData { item: item.into(), count, damage: 0, max_damage: None, custom_data: NbtCompound::new(), custom_model_data: None, enchantments: Default::default() }
    }

    fn tool_stack(item: &str) -> StackData {
        let mut data = NbtCompound::new();
        data.insert(tool_stack::TAG_MATERIALS.into(), Nbt::List(Vec::new()));
        StackData { item: item.into(), count: 1, damage: 0, max_damage: None, custom_data: data, custom_model_data: None, enchantments: Default::default() }
    }

    fn lookup(names: &[&str]) -> RegistryLookup {
        RegistryLookup::from_parts(&names.iter().map(|name| name.to_string()).collect::<Vec<_>>(), &[]).unwrap()
    }

    fn lookup_with_tag(names: &[&str], tag: &str, members: &[&str]) -> RegistryLookup {
        RegistryLookup::from_parts(
            &names.iter().map(|name| name.to_string()).collect::<Vec<_>>(),
            &[(tag.to_string(), members.iter().map(|name| name.to_string()).collect::<Vec<_>>())],
        )
        .unwrap()
    }

    fn id(value: &str) -> Identifier {
        Identifier::parse(value).unwrap()
    }

    fn provider(modifier: &str, item: &str, wear: i32) -> (Identifier, Identifier, i32) {
        (id(modifier), id(item), wear)
    }

    #[test]
    fn no_exchanging_means_no_exchange_even_when_a_hand_provides() {
        let offhand = stack("minecraft:stone", 64);
        let known = lookup(&["minecraft:stone"]);
        assert_eq!(exchange_decision(false, Some(&offhand), None, Some(provider("tconstruct:glowing", "minecraft:glowstone", 5)), Some(&known), None, true, None, &[], &[], None), None);
    }

    #[test]
    fn an_offhand_block_item_places_and_shrinks() {
        let offhand = stack("minecraft:stone", 64);
        let known = lookup(&["minecraft:stone"]);
        assert_eq!(
            exchange_decision(true, Some(&offhand), None, None, Some(&known), None, true, None, &[], &[], None),
            Some(Exchange { state: pluto_lib::BlockState::new(id("minecraft:stone")), consume: ExchangeConsume::ShrinkOffhand })
        );
    }

    #[test]
    fn an_offhand_block_shadows_the_mainhand_provider() {
        let offhand = stack("minecraft:stone", 64);
        let known = lookup(&["minecraft:stone"]);
        assert_eq!(
            exchange_decision(true, Some(&offhand), None, Some(provider("tconstruct:glowing", "minecraft:glowstone", 5)), Some(&known), None, true, None, &[], &[], None),
            Some(Exchange { state: pluto_lib::BlockState::new(id("minecraft:stone")), consume: ExchangeConsume::ShrinkOffhand })
        );
    }

    #[test]
    fn an_offhand_tool_provider_wears_the_offhand_tool() {
        let offhand = tool_stack("tconstruct:pickaxe");
        assert_eq!(
            exchange_decision(true, Some(&offhand), Some(provider("tconstruct:glowing", "minecraft:glowstone", 5)), None, None, None, true, None, &[], &[], None),
            Some(Exchange { state: pluto_lib::BlockState::new(id("minecraft:glowstone")), consume: ExchangeConsume::DamageOffhand { modifier: id("tconstruct:glowing"), wear: 5 } })
        );
    }

    #[test]
    fn an_offhand_tool_without_a_provider_falls_through_to_the_mainhand() {
        let offhand = tool_stack("tconstruct:pickaxe");
        assert_eq!(
            exchange_decision(true, Some(&offhand), None, Some(provider("tconstruct:glowing", "minecraft:glowstone", 5)), None, None, true, None, &[], &[], None),
            Some(Exchange { state: pluto_lib::BlockState::new(id("minecraft:glowstone")), consume: ExchangeConsume::DamageMainHand { modifier: id("tconstruct:glowing"), wear: 5 } })
        );
    }

    #[test]
    fn an_offhand_non_block_item_falls_through_to_the_mainhand() {
        // A stick is an item but names no block: the block registry does not name it, so it cannot
        // provide itself (the item snapshot alone used to let it answer, and the host refused the
        // placement — the main hand was never tried).
        let offhand = stack("minecraft:stick", 1);
        let known = lookup(&["minecraft:stone"]);
        assert_eq!(
            exchange_decision(true, Some(&offhand), None, Some(provider("tconstruct:glowing", "minecraft:glowstone", 5)), Some(&known), None, true, None, &[], &[], None),
            Some(Exchange { state: pluto_lib::BlockState::new(id("minecraft:glowstone")), consume: ExchangeConsume::DamageMainHand { modifier: id("tconstruct:glowing"), wear: 5 } })
        );
    }

    #[test]
    fn without_a_block_capture_the_item_snapshot_is_the_mirror() {
        let offhand = stack("minecraft:stone", 64);
        let known = lookup(&["minecraft:stone"]);
        assert_eq!(
            exchange_decision(true, Some(&offhand), None, None, None, Some(&known), true, None, &[], &[], None),
            Some(Exchange { state: pluto_lib::BlockState::new(id("minecraft:stone")), consume: ExchangeConsume::ShrinkOffhand })
        );
    }

    #[test]
    fn an_empty_offhand_stack_falls_through_to_the_mainhand() {
        let offhand = stack("minecraft:stone", 0);
        let known = lookup(&["minecraft:stone"]);
        assert_eq!(
            exchange_decision(true, Some(&offhand), None, Some(provider("tconstruct:glowing", "minecraft:glowstone", 5)), Some(&known), None, true, None, &[], &[], None),
            Some(Exchange { state: pluto_lib::BlockState::new(id("minecraft:glowstone")), consume: ExchangeConsume::DamageMainHand { modifier: id("tconstruct:glowing"), wear: 5 } })
        );
    }

    #[test]
    fn nothing_providing_means_no_exchange() {
        let offhand = stack("minecraft:stick", 1);
        let known = lookup(&["minecraft:stone"]);
        assert_eq!(exchange_decision(true, Some(&offhand), None, None, Some(&known), None, true, None, &[], &[], None), None);
        assert_eq!(exchange_decision(true, None, None, None, Some(&known), None, true, None, &[], &[], None), None);
    }

    #[test]
    fn a_player_who_may_not_build_needs_the_place_tag_on_the_mined_block() {
        let offhand = stack("minecraft:stone", 64);
        let known = lookup(&["minecraft:stone"]);
        let mined = pluto_lib::BlockState::parse("minecraft:dirt").unwrap();
        // No tag at all: no exchange.
        assert_eq!(exchange_decision(true, Some(&offhand), None, None, Some(&known), None, false, Some(&mined), &[], &[], None), None);
        // A tag naming the placed block but not the mined one: the original checks the mined
        // position, so this is still no exchange.
        let entries = vec!["minecraft:stone".to_string()];
        assert_eq!(exchange_decision(true, Some(&offhand), None, None, Some(&known), None, false, Some(&mined), &entries, &[], None), None);
        // A tag naming the mined block: the exchange runs.
        let entries = vec!["minecraft:dirt".to_string()];
        assert_eq!(
            exchange_decision(true, Some(&offhand), None, None, Some(&known), None, false, Some(&mined), &entries, &[], None),
            Some(Exchange { state: pluto_lib::BlockState::new(id("minecraft:stone")), consume: ExchangeConsume::ShrinkOffhand })
        );
    }

    #[test]
    fn a_place_tag_entry_with_a_state_suffix_must_match_the_mined_state() {
        let offhand = stack("minecraft:stone", 64);
        let known = lookup(&["minecraft:stone", "minecraft:oak_stairs"]);
        let mined = pluto_lib::BlockState::parse("minecraft:oak_stairs[facing=east]").unwrap();
        let entries = vec!["minecraft:oak_stairs[facing=west]".to_string()];
        assert_eq!(exchange_decision(true, Some(&offhand), None, None, Some(&known), None, false, Some(&mined), &entries, &[], None), None);
        let entries = vec!["minecraft:oak_stairs[facing=east]".to_string()];
        assert!(exchange_decision(true, Some(&offhand), None, None, Some(&known), None, false, Some(&mined), &entries, &[], None).is_some());
    }

    #[test]
    fn a_place_tag_tag_entry_resolves_through_the_block_snapshot() {
        let offhand = stack("minecraft:stone", 64);
        let known = lookup_with_tag(&["minecraft:stone", "minecraft:oak_log"], "minecraft:logs", &["minecraft:oak_log"]);
        let mined = pluto_lib::BlockState::parse("minecraft:oak_log").unwrap();
        let entries = vec!["#minecraft:logs".to_string()];
        assert!(exchange_decision(true, Some(&offhand), None, None, Some(&known), None, false, Some(&mined), &entries, &[], None).is_some());
        // Without the block snapshot a #tag names nothing (the item mirror still provides the
        // block, so the gate is what refuses).
        let items = lookup(&["minecraft:stone"]);
        assert_eq!(exchange_decision(true, Some(&offhand), None, None, None, Some(&items), false, Some(&mined), &entries, &[], None), None);
    }

    #[test]
    fn a_place_tag_nbt_predicate_matches_the_mined_block_entity() {
        let offhand = stack("minecraft:stone", 64);
        let known = lookup(&["minecraft:stone", "minecraft:chest"]);
        let mined = pluto_lib::BlockState::parse("minecraft:chest").unwrap();
        // The predicate names the mined block and the block entity's custom name; the text entries name
        // nothing (a predicate carrying nbt never flattens into one), so only the structured read can
        // pass this gate.
        let component = pluto_lib::Nbt::parse_snbt("{blocks:[\"minecraft:chest\"],nbt:{CustomName:'{\"text\":\"Loots\"}'}}").unwrap();
        let predicates = pluto_lib::adventure_place_predicates(&component);
        let mut be = NbtCompound::new();
        be.insert("CustomName".into(), pluto_lib::Nbt::String("{\"text\":\"Loots\"}".to_string()));
        assert!(exchange_decision(true, Some(&offhand), None, None, Some(&known), None, false, Some(&mined), &[], &predicates, Some(&be)).is_some());
        // Another name in the block entity: the predicate does not match, and the text gate has nothing
        // to fall back to, so there is no exchange.
        let mut other = NbtCompound::new();
        other.insert("CustomName".into(), pluto_lib::Nbt::String("{\"text\":\"Junk\"}".to_string()));
        assert_eq!(exchange_decision(true, Some(&offhand), None, None, Some(&known), None, false, Some(&mined), &[], &predicates, Some(&other)), None);
        // No block entity at all: an nbt predicate cannot match.
        assert_eq!(exchange_decision(true, Some(&offhand), None, None, Some(&known), None, false, Some(&mined), &[], &predicates, None), None);
    }

    #[test]
    fn a_place_tag_state_range_predicate_answers_without_a_block_entity() {
        let offhand = stack("minecraft:stone", 64);
        let known = lookup(&["minecraft:stone", "minecraft:oak_stairs"]);
        let mined = pluto_lib::BlockState::parse("minecraft:oak_stairs[waterlogged=false]").unwrap();
        // A range predicate with no nbt: the mined state answers it with no block entity needed.
        let component = pluto_lib::Nbt::parse_snbt("{blocks:[\"minecraft:oak_stairs\"],state:{waterlogged:false}}").unwrap();
        let predicates = pluto_lib::adventure_place_predicates(&component);
        assert!(exchange_decision(true, Some(&offhand), None, None, Some(&known), None, false, Some(&mined), &[], &predicates, None).is_some());
    }

    #[test]
    fn a_player_who_may_not_build_cannot_exchange_through_a_provider() {
        // The port's provider stacks carry no place tag, so nothing a provider offers passes the gate.
        let mined = pluto_lib::BlockState::parse("minecraft:dirt").unwrap();
        let entries = vec!["minecraft:dirt".to_string()];
        assert_eq!(
            exchange_decision(true, None, None, Some(provider("tconstruct:glowing", "minecraft:glowstone", 5)), None, None, false, Some(&mined), &entries, &[], None),
            None
        );
    }

    #[test]
    fn a_tool_carrying_exchanging_and_a_provider_resolves_both() {
        let data = crate::tool_definitions::Data::decode(&json!({"modules":[
            {"type":"tconstruct:base_stats","stats":{"tconstruct:durability":100,"tconstruct:mining_speed":4,"tconstruct:harvest_tier":"minecraft:stone"}},
            {"type":"tconstruct:is_effective","predicate_type":"mantle:tag","tag":"minecraft:mineable/pickaxe"}
        ]})).unwrap();
        let mut materials = crate::materials::Definitions::default();
        materials.server_loaded = true;
        let mut registry = crate::modifiers::Registry::default();
        for modifier in crate::modifiers::ported_code_modifiers().unwrap() {
            registry.register_static(modifier).unwrap();
        }
        let glowing = crate::modifiers::Modifier::decode(
            id("tconstruct:glowing"),
            &json!({"modules":[{"type":"tconstruct:block_item_provider","item":"minecraft:glowstone","tool_damage":5}]}),
        )
        .unwrap();
        registry.register_static(glowing).unwrap();
        let list = crate::modifier_entries::List::from_nbt(None)
            .with_modifier(id("tconstruct:exchanging"), 1).unwrap()
            .with_modifier(id("tconstruct:glowing"), 1).unwrap();
        let mut nbt = NbtCompound::new();
        nbt.insert(tool_stack::TAG_MODIFIERS.into(), list.to_nbt());
        let mut stats = NbtCompound::new();
        stats.insert("tconstruct:durability".into(), Nbt::Float(100.0));
        nbt.insert(tool_stack::TAG_STATS.into(), Nbt::Compound(stats));
        let item = id("tconstruct:pickaxe");
        let env = tool_stack::Environment { definition: Some(&data), has_materials: false, materials: &materials, modifiers: &registry, item_tags_loaded: false, item: Some(&item), item_tags: None };
        assert!(tool_stack::saved_modifiers(&nbt, &registry).iter().any(|(entry, _)| entry.id == id("tconstruct:exchanging")));
        assert_eq!(tool_stack::block_item_provider(&nbt, &env), Some((id("tconstruct:glowing"), id("minecraft:glowstone"), 5)));
        let exchanging = crate::modifiers::ported_code_modifiers().unwrap().into_iter().find(|modifier| modifier.id == id("tconstruct:exchanging")).unwrap();
        assert_eq!(exchanging.priority, -32788);
        assert!(exchanging.modules().iter().any(|module| matches!(module, crate::modifier_modules::Module::Exchanging)));
        assert!(crate::modifiers::unported_code_modifiers().unwrap().iter().all(|modifier| modifier.id != id("tconstruct:exchanging")));
    }

    #[test]
    fn shrinking_the_offhand_writes_back_through_the_player() {
        let mut ops = MemoryPlayerOps::default();
        let player = ServerPlayer { uuid: 1, name: "Alex".into() };
        let held = stack("tconstruct:pickaxe", 1);
        let offhand = stack("minecraft:stone", 3);
        let exchange = Exchange { state: pluto_lib::BlockState::new(id("minecraft:stone")), consume: ExchangeConsume::ShrinkOffhand };
        with_ops(&mut ops, || apply_exchange_consume(&player, &held, Some(&offhand), &exchange, false));
        assert_eq!(ops.carried.get(&(1, -106)).map(|stack| stack.count), Some(2));
        // The last block clears the hand.
        let offhand = stack("minecraft:stone", 1);
        with_ops(&mut ops, || apply_exchange_consume(&player, &held, Some(&offhand), &exchange, false));
        assert!(!ops.carried.contains_key(&(1, -106)));
    }

    #[test]
    fn a_creative_holder_consumes_nothing() {
        // BlockItem.place does not shrink a creative player's stack, so the original's fake stack
        // never empties and no consume runs: neither the shrink nor a provider's wear lands.
        let mut ops = MemoryPlayerOps::default();
        let player = ServerPlayer { uuid: 1, name: "Alex".into() };
        let held = stack("tconstruct:pickaxe", 1);
        let offhand = stack("minecraft:stone", 3);
        let shrink = Exchange { state: pluto_lib::BlockState::new(id("minecraft:stone")), consume: ExchangeConsume::ShrinkOffhand };
        with_ops(&mut ops, || apply_exchange_consume(&player, &held, Some(&offhand), &shrink, true));
        assert!(ops.carried.is_empty());
        let wear = Exchange { state: pluto_lib::BlockState::new(id("minecraft:glowstone")), consume: ExchangeConsume::DamageMainHand { modifier: id("tconstruct:glowing"), wear: 5 } };
        with_ops(&mut ops, || apply_exchange_consume(&player, &held, Some(&offhand), &wear, true));
        assert!(ops.carried.is_empty());
    }
}
