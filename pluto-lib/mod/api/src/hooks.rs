//! The only file that names Horizon types: it turns Horizon's hook points into PlutoLib's Fabric-like events.
//!
//! Hook points are Horizon's stable, only-ever-added surface (`native-hooks.json`). A hook that can veto returns
//! `Flow::Cancel` when the PlutoLib callbacks say so.

use horizon_client::mod_api::{
    AttackEntityEvent, ChunkLoadEvent, ClientResourcesReloadedEvent, LootTableLoadEvent, Text, TooltipEvent, ChunkUnloadEvent, DestroyBlockEvent, EntityJoinLevelEvent, EntityLeaveLevelEvent, Flow, Hooks,
    Host, LevelTickPostEvent, LevelTickPreEvent, LivingIncomingDamageEvent, PlayerLoggedInEvent, PlayerLoggedOutEvent,
    RightClickBlockEvent, ServerAboutToStartEvent, ServerStoppingEvent, ServerTickEvent, ServerTickPhase, UseItemEvent,
};

use crate::pure::entity_events_v1::{self, LivingDamage};
use crate::pure::{events_interaction_v0, item_api_v1, lifecycle_events_v1, loot_api_v2, networking_api_v1, resource_loader_v0};

/// PlutoLib's entry point: the launcher calls it before the hooks of every mod that depends on PlutoLib.
pub fn hooks() -> Box<dyn Hooks> {
    Box::new(PlutoHooks)
}

struct PlutoHooks;

fn cancel_if(handled: bool) -> Flow {
    if handled { Flow::Cancel } else { Flow::Continue }
}

impl Hooks for PlutoHooks {
    fn server_about_to_start(&self, _host: &mut dyn Host, event: &mut ServerAboutToStartEvent) {
        lifecycle_events_v1::server_starting(event.dedicated);
    }

    fn server_stopping(&self, _host: &mut dyn Host, event: &mut ServerStoppingEvent) {
        lifecycle_events_v1::server_stopping(event.dedicated);
    }

    fn server_tick(&self, _host: &mut dyn Host, event: &mut ServerTickEvent<'_>) {
        match event.phase {
            ServerTickPhase::ServerTickStart { .. } => lifecycle_events_v1::server_tick(true),
            ServerTickPhase::ServerTickEnd { .. } => lifecycle_events_v1::server_tick(false),
            #[allow(unreachable_patterns)]
            _ => {}
        }
    }

    fn level_tick_pre(&self, _host: &mut dyn Host, event: &mut LevelTickPreEvent<'_>) {
        lifecycle_events_v1::world_tick(true, event.dimension);
    }

    fn level_tick_post(&self, _host: &mut dyn Host, event: &mut LevelTickPostEvent<'_>) {
        lifecycle_events_v1::world_tick(false, event.dimension);
    }

    fn chunk_load(&self, _host: &mut dyn Host, event: &mut ChunkLoadEvent<'_>) {
        lifecycle_events_v1::chunk_loaded(event.dimension, event.chunk, event.new);
    }

    fn chunk_unload(&self, _host: &mut dyn Host, event: &mut ChunkUnloadEvent<'_>) {
        lifecycle_events_v1::chunk_unloaded(event.dimension, event.chunk);
    }

    fn player_logged_in(&self, _host: &mut dyn Host, event: &mut PlayerLoggedInEvent<'_>) {
        networking_api_v1::player_joined(event.name.to_string());
    }

    fn player_logged_out(&self, _host: &mut dyn Host, event: &mut PlayerLoggedOutEvent<'_>) {
        networking_api_v1::player_left(event.name.to_string());
    }

    fn entity_join_level(&self, _host: &mut dyn Host, event: &mut EntityJoinLevelEvent<'_>) -> Flow {
        entity_events_v1::entity_loaded(event.entity, event.kind, event.dimension);
        Flow::Continue
    }

    fn entity_leave_level(&self, _host: &mut dyn Host, event: &mut EntityLeaveLevelEvent<'_>) {
        entity_events_v1::entity_unloaded(event.entity, event.kind, event.dimension);
    }

    fn use_item(&self, _host: &mut dyn Host, event: &mut UseItemEvent<'_>) -> Flow {
        cancel_if(events_interaction_v0::use_item(&event.stack.item, event.stack.count))
    }

    fn right_click_block(&self, _host: &mut dyn Host, event: &mut RightClickBlockEvent) -> Flow {
        cancel_if(events_interaction_v0::use_block(event.pos, event.face))
    }

    fn attack_entity(&self, _host: &mut dyn Host, event: &mut AttackEntityEvent<'_>) -> Flow {
        cancel_if(events_interaction_v0::attack_entity(event.entity, event.kind, event.pos))
    }

    fn destroy_block(&self, _host: &mut dyn Host, event: &mut DestroyBlockEvent) -> Flow {
        cancel_if(!events_interaction_v0::before_block_break(event.pos))
    }

    fn tooltip(&self, _host: &dyn Host, event: &mut TooltipEvent<'_>) {
        for line in item_api_v1::item_tooltip(&event.stack.item, event.stack.count) {
            event.lines.push((Text::literal(line.text), line.color));
        }
    }

    fn loot_table_load(&self, event: &mut LootTableLoadEvent<'_>) {
        let edited = loot_api_v2::modify(event.id, event.table.clone());
        *event.table = edited;
    }

    fn client_resources_reloaded(&self, _host: &mut dyn Host, _event: &mut ClientResourcesReloadedEvent) {
        resource_loader_v0::client_resources_reloaded();
    }

    fn living_incoming_damage(&self, event: &mut LivingIncomingDamageEvent<'_>) -> Flow {
        let damage = LivingDamage {
            entity: event.entity,
            kind: event.kind.to_owned(),
            damage_type: event.damage_type.to_owned(),
            attacker: event.causing,
            direct: event.direct,
            amount: event.amount,
        };
        cancel_if(!entity_events_v1::allow_damage(&damage))
    }
}
