//! The only file that names Horizon types: it turns Horizon's hook points into PlutoLib's Fabric-like events.
//!
//! Hook points are Horizon's stable, only-ever-added surface (`native-hooks.json`). A hook that can veto returns
//! `Flow::Cancel` when the PlutoLib callbacks say so.

use horizon_client::mod_api::{
    AttackEntityEvent, ChunkLoadEvent, ClientResourcesReloadedEvent, LootTableLoadEvent, Text, TooltipEvent, ChunkUnloadEvent, DestroyBlockEvent, EntityJoinLevelEvent, EntityLeaveLevelEvent, Flow, Hooks,
    Host, LevelTickPostEvent, LevelTickPreEvent, LivingIncomingDamageEvent, PlayerLoggedInEvent, PlayerLoggedOutEvent,
    UseAnimation, UseAnimationEvent, UseDurationEvent, NativeCommand, PerformCommandEvent, RegisterCommandsEvent as RegisterCmds, ConfigLoadingEvent, ConfigReloadingEvent, ConfigUnloadingEvent, NativeConfig, NativeConfigFile, NativeConfigKind, RegisterConfigsEvent,
    AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, Tag,
    RightClickBlockEvent, ServerAboutToStartEvent, ServerStoppingEvent, ServerTickEvent, ServerTickPhase, UseItemEvent,
};

use crate::pure::entity_events_v1::{self, LivingDamage};
use crate::pure::player_api::{self, PlayerOps};
use horizon_client::items::ItemStack;
use crate::pure::command_api_v2;
use crate::pure::config_api::{self, ConfigKind, ModConfig};
use crate::pure::{events_interaction_v0, item_api_v1, lifecycle_events_v1, loot_api_v2, networking_api_v1, resource_loader_v0};

/// PlutoLib's entry point: the launcher calls it before the hooks of every mod that depends on PlutoLib.
pub fn hooks() -> Box<dyn Hooks> {
    horizon_client::creative::pluto_creative::set_document(crate::pure::item_group_api_v1::document);
    item_api_v1::set_register_sink(|item, behavior| {
        horizon_client::entry::pluto_item_use::set(
            item,
            horizon_client::entry::pluto_item_use::Spec {
                food: behavior.food.map(|food| (food.nutrition, food.saturation)),
                can_always_eat: behavior.can_always_eat,
                remainder: behavior.remainder.as_ref().map(|(id, count)| (id.to_string(), *count)),
            },
        );
    });
    Box::new(PlutoHooks)
}

struct PlutoHooks;

/// What a mod's `hooks()` returns: the launcher's entry point type. A mod that registers everything through PlutoLib's events returns
/// [`no_hooks`], so its source never names a Horizon type.
pub type ModHooks = Box<dyn Hooks>;

struct NoHooks;
impl Hooks for NoHooks {}

/// Hooks that do nothing, for a mod whose behaviour is all registered through PlutoLib events.
pub fn no_hooks() -> ModHooks {
    Box::new(NoHooks)
}

/// PlutoLib's player operations over Horizon's `Host`, open only while a callback that carries a player runs.
struct HostOps<'a>(&'a mut dyn Host);

impl PlayerOps for HostOps<'_> {
    fn tell(&mut self, player: u128, line: &str) -> bool {
        self.0.tell(player, Text::literal(line.to_owned()))
    }

    fn give(&mut self, player: u128, item: &str, count: u32) -> u32 {
        self.0.give_stack(player, ItemStack::new(item, count)).map_or(0, |left| left.count)
    }

    fn count_item(&mut self, player: u128, item: &str) -> u32 {
        let _ = player; // inside a callback the host reaches the acting player, who is the one the PlutoLib handle names
        (0..36).filter_map(|slot| self.0.carried_stack(slot).filter(|s| &*s.item == item).map(|s| s.count)).sum()
    }

    fn take_item(&mut self, player: u128, item: &str, count: u32) -> u32 {
        let _ = player;
        self.0.take_items(item, count)
    }

    fn empty_slots(&mut self, player: u128) -> u32 {
        let _ = player;
        (0..36).filter(|slot| self.0.carried_stack(*slot).is_none()).count() as u32
    }

    fn attachment_get(&mut self, player: u128, id: &str) -> Option<String> {
        let kind = json_attachment(id)?;
        match kind.get_existing(&mut *self.0, Holder::Player(player)) {
            Ok(Some(json)) if !json.is_empty() => Some(json),
            _ => None,
        }
    }

    fn attachment_set(&mut self, player: u128, id: &str, json: &str) -> bool {
        match json_attachment(id) {
            Some(kind) => kind.set(&mut *self.0, Holder::Player(player), json.to_owned()).is_ok(),
            None => false,
        }
    }
}

/// Horizon wants attachment keys as `'static` strs and a codec made of plain functions, while PlutoLib attachments are created at run time.
/// So each id gets one leaked `AttachmentType<String>` (made once, kept for the process) whose value is the JSON text in a string tag.
fn json_attachment(id: &str) -> Option<&'static AttachmentType<String>> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static KINDS: OnceLock<Mutex<HashMap<String, &'static AttachmentType<String>>>> = OnceLock::new();
    let (namespace, name) = id.split_once(':')?;
    let mut kinds = KINDS.get_or_init(Default::default).lock().ok()?;
    if let Some(kind) = kinds.get(id) {
        return Some(*kind);
    }
    let kind: &'static AttachmentType<String> = Box::leak(Box::new(AttachmentType {
        namespace: Box::leak(namespace.to_owned().into_boxed_str()),
        name: Box::leak(name.to_owned().into_boxed_str()),
        default: String::new,
        codec: Some(AttachmentCodec {
            encode: |json: &String| Ok([("json".into(), Tag::String(json.as_str().into()))].into()),
            decode: |data: &Compound| match data.get(&"json".into()) {
                Some(Tag::String(s)) => Ok(s.to_string_lossy()),
                _ => Err(AttachmentError::Decode),
            },
        }),
        copy_on_death: false,
        sync: false,
    }));
    kinds.insert(id.to_owned(), kind);
    Some(kind)
}

fn cancel_if(handled: bool) -> Flow {
    if handled { Flow::Cancel } else { Flow::Continue }
}

fn model_config(file: &NativeConfigFile) -> ModConfig {
    let kind = match file.kind {
        NativeConfigKind::Client => ConfigKind::Client,
        NativeConfigKind::Common => ConfigKind::Common,
        NativeConfigKind::Server => ConfigKind::Server,
    };
    ModConfig { mod_id: file.mod_id.clone(), kind, file: file.file.clone(), text: file.text.clone() }
}

impl Hooks for PlutoHooks {
    fn use_duration(&self, _host: &dyn Host, event: &mut UseDurationEvent<'_>) {
        event.duration = item_api_v1::use_duration(&event.stack.item, event.duration);
    }

    fn use_animation(&self, _host: &dyn Host, event: &mut UseAnimationEvent<'_>) {
        use item_api_v1::UseAction;
        if let Some(action) = item_api_v1::use_animation(&event.stack.item) {
            *event.animation = match action {
                UseAction::None => UseAnimation::None,
                UseAction::Eat => UseAnimation::Eat,
                UseAction::Drink => UseAnimation::Drink,
                UseAction::Block => UseAnimation::Block,
                UseAction::Bow => UseAnimation::Bow,
                UseAction::Trident => UseAnimation::Trident,
                UseAction::Crossbow => UseAnimation::Crossbow,
                UseAction::Spyglass => UseAnimation::Spyglass,
                UseAction::TootHorn => UseAnimation::TootHorn,
                UseAction::Brush => UseAnimation::Brush,
                UseAction::Bundle => UseAnimation::Bundle,
                UseAction::Spear => UseAnimation::Spear,
            };
        }
    }

    fn register_commands(&self, _host: &mut dyn Host, event: &mut RegisterCmds<'_>) {
        for (usage, level) in command_api_v2::collect() {
            event.commands.push(if level == 0 { NativeCommand::new(&usage) } else { NativeCommand::op(&usage, level) });
        }
    }

    fn perform_command(&self, host: &mut dyn Host, event: &mut PerformCommandEvent<'_>) -> Flow {
        let (uuid, name, line) = (event.player.uuid, event.player.name.clone(), event.command.to_owned());
        let ran = player_api::with_ops(&mut HostOps(host), || command_api_v2::execute(uuid, name, &line));
        cancel_if(ran)
    }

    fn register_configs(&self, _host: &mut dyn Host, event: &mut RegisterConfigsEvent<'_>) {
        for spec in config_api::registered() {
            let kind = match spec.kind {
                ConfigKind::Client => NativeConfigKind::Client,
                ConfigKind::Common => NativeConfigKind::Common,
                ConfigKind::Server => NativeConfigKind::Server,
            };
            let config = NativeConfig::new(&spec.mod_id, kind, &spec.default_text);
            event.configs.push(match &spec.file {
                Some(file) => config.file(file),
                None => config,
            });
        }
    }

    fn config_loading(&self, _host: &mut dyn Host, event: &mut ConfigLoadingEvent<'_>) {
        config_api::loading(model_config(event.config));
    }

    fn config_reloading(&self, _host: &mut dyn Host, event: &mut ConfigReloadingEvent<'_>) {
        config_api::reloading(model_config(event.config));
    }

    fn config_unloading(&self, _host: &mut dyn Host, event: &mut ConfigUnloadingEvent<'_>) {
        config_api::unloading(model_config(event.config));
    }

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

    fn player_logged_in(&self, host: &mut dyn Host, event: &mut PlayerLoggedInEvent<'_>) {
        let (uuid, name) = (event.player, event.name.to_string());
        player_api::with_ops(&mut HostOps(host), || networking_api_v1::player_joined(uuid, name));
    }

    fn player_logged_out(&self, host: &mut dyn Host, event: &mut PlayerLoggedOutEvent<'_>) {
        let (uuid, name) = (event.player, event.name.to_string());
        player_api::with_ops(&mut HostOps(host), || networking_api_v1::player_left(uuid, name));
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
