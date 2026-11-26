//! PlutoLib: Horizon's Fabric API.
//!
//! An independent mod that other mods call (they declare `pluto_lib` under `[dependencies]` in `sunrise-mod.toml`). Names and
//! behaviour follow Fabric API; see `pluto-lib/docs/design-principles.md` and the specification. Modules mirror Fabric API
//! artifacts (`lifecycle_events_v1` is `fabric-lifecycle-events-v1`). The host-independent part is [`pure`]; [`hooks`] is the
//! one file that connects it to the game.

mod exports;
mod block_part;
mod hooks;
mod parts;
pub mod pure;

pub use parts::register_parts;

pub use crate::exports::*;

/// Block movement factors (speed and jump) for mod blocks.
pub use pluto_movement_api as movement;

/// PlutoLib's native entry point. The hook bodies live in [`hooks`], which is
/// being ported from the Horizon host bridge to this crate's `mod_api`
/// surface; methods are wired up there as the port lands.
#[derive(Default)]
pub struct Mod;

use mod_api::RegistrationContext;

#[mod_api::sunrise_mod]
impl mod_api::Hooks for Mod {
    fn register(&self, registrar: &mut dyn mod_api::Registrar, cx: &RegistrationContext) {
        hooks::register(registrar, cx);
    }

    fn create_skull_models(&self, _context: &RegistrationContext, event: &mut mod_api::CreateSkullModelsEvent<'_>) {
        parts::create_skull_models(event);
    }

    fn register_block_tint_sources(&self, _context: &RegistrationContext, event: &mut mod_api::RegisterBlockTintSourcesEvent<'_>) {
        parts::register_block_tint_sources(event);
    }

    fn player_container_open(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::PlayerContainerOpenEvent<'_>) {
        parts::player_container_open(host, event);
    }

    fn player_logged_in(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::PlayerLoggedInEvent<'_>) {
        hooks::player_logged_in(host, event);
    }

    fn player_logged_out(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::PlayerLoggedOutEvent<'_>) {
        hooks::player_logged_out(host, event);
    }

    fn server_about_to_start(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::ServerAboutToStartEvent) {
        let _ = host;
        hooks::server_about_to_start(event);
    }

    fn server_stopping(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::ServerStoppingEvent) {
        let _ = host;
        hooks::server_stopping(event);
    }

    fn server_tick(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::ServerTickEvent<'_>) {
        hooks::server_tick(host, event);
    }

    fn chunk_load(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::ChunkLoadEvent<'_>) {
        hooks::chunk_load(host, event);
    }

    fn chunk_unload(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::ChunkUnloadEvent<'_>) {
        let _ = host;
        hooks::chunk_unload(event);
    }

    fn fluid_passage(&self, host: &dyn mod_api::LevelHost, event: &mut mod_api::FluidPassageEvent<'_>) {
        hooks::fluid_passage(host, event);
    }

    fn entity_join_level(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::EntityJoinLevelEvent<'_>) -> mod_api::Flow {
        hooks::entity_join_level(host, event)
    }

    fn entity_leave_level(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::EntityLeaveLevelEvent<'_>) {
        hooks::entity_leave_level(host, event);
    }

    fn attack_entity(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::AttackEntityEvent<'_>) -> mod_api::Flow {
        let _ = host;
        hooks::attack_entity(event)
    }


    fn right_click_block(&self, host: mod_api::HostSide, event: &mut mod_api::RightClickBlockEvent<'_>) -> mod_api::Flow {
        let _ = host;
        hooks::right_click_block(event)
    }

    fn loot_table_load(&self, context: &dyn mod_api::DataLoadContext, event: &mut mod_api::LootTableLoadEvent<'_>) {
        let _ = context;
        hooks::loot_table_load(event);
    }




    fn client_resources_reloaded(&self, context: &dyn mod_api::ClientReloadHost, event: &mut mod_api::ClientResourcesReloadedEvent) {
        let _ = (context, event);
        hooks::client_resources_reloaded();
    }

    fn living_fall(&self, context: &dyn mod_api::LoopContext, event: &mut mod_api::LivingFallEvent<'_>) -> mod_api::Flow {
        let _ = context;
        hooks::living_fall(event)
    }

    fn living_jump(&self, context: &dyn mod_api::LoopContext, event: &mut mod_api::LivingJumpEvent<'_>) {
        let _ = context;
        hooks::living_jump(event);
    }

    fn living_death(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::LivingDeathEvent<'_>) -> mod_api::Flow {
        hooks::living_death(host, event)
    }

    fn living_damage_pre(&self, host: &mut dyn mod_api::HitHost, event: &mut mod_api::LivingDamagePreEvent<'_>) {
        let _ = host;
        hooks::living_damage_pre(event);
    }

    fn living_incoming_damage(&self, host: &mut dyn mod_api::HitHost, event: &mut mod_api::LivingIncomingDamageEvent<'_>) -> mod_api::Flow {
        let _ = host;
        hooks::living_incoming_damage(host, event)
    }

    fn living_experience_drop(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::LivingExperienceDropEvent<'_>) -> mod_api::Flow {
        hooks::living_experience_drop(host, event)
    }

    fn entity_tick_pre(&self, host: mod_api::HostSide, event: &mut mod_api::EntityTickPreEvent<'_>) -> mod_api::Flow {
        hooks::entity_tick_pre(host, event)
    }

    fn entity_tick_post(&self, host: mod_api::HostSide, event: &mut mod_api::EntityTickPostEvent<'_>) {
        hooks::entity_tick_post(host, event)
    }

    fn entity_interact_specific(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::EntityInteractSpecificEvent<'_>) -> mod_api::Flow {
        hooks::entity_interact_specific(host, event)
    }

    fn projectile_impact(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::ProjectileImpactEvent<'_>) -> mod_api::Flow {
        hooks::projectile_impact(host, event)
    }

    fn mob_split(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::MobSplitEvent<'_>) -> mod_api::Flow {
        hooks::mob_split(host, event)
    }

    fn add_server_reload_listeners(&self, _context: &dyn mod_api::ReloadContext, event: &mut mod_api::AddServerReloadListenersEvent<'_>) {
        hooks::add_server_reload_listeners(event)
    }

    fn player_attack_cooldown_reset(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::PlayerAttackCooldownResetEvent<'_>) {
        let _ = host;
        hooks::player_attack_cooldown_reset(event);
    }

    fn player_shear_entity(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::PlayerShearEntityEvent<'_>) -> mod_api::Flow {
        hooks::player_shear_entity(host, event)
    }

    fn loot_drops(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::LootDropsEvent<'_, '_>) {
        hooks::loot_drops(host, event);
    }

    fn menu_links(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::MenuLinksEvent<'_>) {
        hooks::menu_links(host, event);
    }

    fn left_click_block(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::LeftClickBlockEvent) -> mod_api::Flow {
        hooks::left_click_block(host, event)
    }

    fn ingredient(&self, context: &dyn mod_api::DataLoadContext, event: &mut mod_api::IngredientEvent<'_>) -> Option<mod_api::IngredientDefinition> {
        let _ = context;
        hooks::ingredient(event)
    }

    fn player_destroy_item(&self, host: mod_api::HostSide, event: &mut mod_api::PlayerDestroyItemEvent<'_>) {
        let _ = host;
        hooks::player_destroy_item(event);
    }

    fn level_tick_pre(&self, host: mod_api::HostSide, event: &mut mod_api::LevelTickPreEvent) {
        let _ = host;
        hooks::level_tick_pre(event);
    }

    fn level_tick_post(&self, host: mod_api::HostSide, event: &mut mod_api::LevelTickPostEvent) {
        let _ = host;
        hooks::level_tick_post(event);
    }

    fn living_knockback(&self, host: &mut dyn mod_api::LevelRandomHost, event: &mut mod_api::LivingKnockbackEvent<'_>) -> mod_api::Flow {
        let _ = host;
        hooks::living_knockback(event)
    }

    fn player_fish(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::PlayerFishEvent<'_>) -> mod_api::Flow {
        hooks::player_fish(host, event)
    }

    fn register_layer_definitions(&self, context: &RegistrationContext, event: &mut mod_api::RegisterLayerDefinitionsEvent<'_>) {
        let _ = context;
        hooks::register_layer_definitions(event);
    }

    fn register_commands(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::RegisterCommandsEvent<'_>) {
        let _ = host;
        hooks::register_commands(event);
    }

    fn input_key(&self, host: &mut dyn mod_api::ClientHost, event: &mut mod_api::InputKeyEvent<'_>) {
        let _ = host;
        hooks::input_key(event);
    }

    fn movement_input_update(&self, host: &dyn mod_api::ClientHost, event: &mut mod_api::MovementInputUpdateEvent<'_>) {
        let _ = host;
        hooks::movement_input_update(event);
    }

    fn client_player_logging_in(&self, host: &mut dyn mod_api::ClientHost, event: &mut mod_api::ClientPlayerLoggingInEvent) {
        let _ = host;
        hooks::client_player_logging_in();
    }

    fn client_setup(&self, context: &mut dyn mod_api::ClientSetupContext, event: &mut mod_api::ClientSetupEvent) {
        let _ = (context, event);
        hooks::client_setup();
    }

    fn screen_key_pressed_pre(&self, host: &dyn mod_api::ClientHost, event: &mut mod_api::ScreenKeyPressedPreEvent<'_>) -> mod_api::Flow {
        let _ = host;
        hooks::screen_key_pressed_pre(event)
    }

    fn screen_char_typed_pre(&self, host: &dyn mod_api::ClientHost, event: &mut mod_api::ScreenCharTypedPreEvent<'_>) -> mod_api::Flow {
        let _ = host;
        hooks::screen_char_typed_pre(event)
    }

    fn screen_mouse_button_pressed_pre(&self, host: &dyn mod_api::ClientHost, event: &mut mod_api::ScreenMouseButtonPressedPreEvent<'_>) -> mod_api::Flow {
        let _ = host;
        hooks::screen_mouse_button_pressed_pre(event)
    }

    fn screen_mouse_scrolled_pre(&self, host: &dyn mod_api::ClientHost, event: &mut mod_api::ScreenMouseScrolledPreEvent<'_>) -> mod_api::Flow {
        let _ = host;
        hooks::screen_mouse_scrolled_pre(event)
    }

    fn screen_closing(&self, host: &dyn mod_api::ClientHost, event: &mut mod_api::ScreenClosingEvent<'_>) {
        let _ = host;
        hooks::screen_closing(event);
    }
}
