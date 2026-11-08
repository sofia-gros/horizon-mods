//! What `pluto_lib::` names. Shared by the real crate (`lib.rs`) and by the host-independent test crate (`tests/pure`), so the documentation examples are compiled against exactly the names a mod author sees.

pub use crate::pure::base;
pub use crate::pure::base::event::{AllowAll, FirstNotPass, Unit};
pub use crate::pure::base::support::{MATRIX, entries_with};
pub use crate::pure::base::{
    ActionResult, Callback, Nbt, NbtCompound, NbtCompoundExt, Combiner, DEFAULT_NAMESPACE, Event, Identifier, IdentifierError, PhaseOrderingCycle, Support, SupportEntry,
    default_phase,
};
pub use crate::pure::block_api_v1::{
    self, BlockBehavior, BlockBehaviors, BlockEditor, BlockView, BonemealContext, PickContext, PlaceOnContext, TickContext, Direction, EntityInsideContext, MemoryBlockView, Placement, PlacementContext, ShapeUpdateContext,
    SurviveContext,
};
pub use crate::pure::block_entity_api_v1::{self, BlockEntityTick, BlockEntityType, BlockEntityTypeRegistry, BlockPos, FluidStack, ItemSlot};
pub use crate::pure::block_entity_renderer_api_v1::{self, BlockEntityCanvas, BlockEntityRender, BlockEntityRendererRegistry};
pub use crate::pure::color_provider_api_v1::{self, BlockColorContext, ColorProviderRegistry};
pub use crate::pure::command_api_v2::{self, CommandContext, CommandRegistrar, CommandRegistrationCallback};
pub use crate::pure::config_api::{self, ConfigEvents, ConfigKind, ConfigRegistry, ConfigSpec, ModConfig};
pub use crate::pure::data_attachment_api_v1::{self, AttachmentRegistry, AttachmentType};
pub use crate::pure::entity_events_v1::{self, EntityInfo, LivingDamage, ServerEntityEvents, ServerLivingEntityEvents};
pub use crate::pure::events_interaction_v0::{
    self, AttackEntityCallback, AttackEntityContext, BreakSpeedCallback, BreakSpeedContext, BlockBreakContext, PlayerBlockBreakEvents, UseBlockCallback, UseBlockContext,
    UseItemCallback, UseItemContext, UseItemFinishCallback, UseItemFinishContext, UseItemOnBlockCallback, UseItemOnBlockContext,
};
pub use crate::pure::lifecycle_events_v1::{
    self, ServerChunk, ServerChunkEvents, ServerInfo, ServerLifecycleEvents, ServerTickEvents, WorldTick,
};
pub use crate::pure::fluid_api_v1::{self, FluidAttributes, FluidDef, FluidRegistry};
pub use crate::pure::i18n_api::{self, FALLBACK_LANGUAGE, I18n, LanguageChange, LanguageEvents, Translations};
pub use crate::pure::item_api_v1::{self, ItemTooltipCallback, ItemTooltipContext, Food, ItemUse, ItemUseRegistry, TooltipLine, UseAction};
pub use crate::pure::item_group_api_v1::{self, ItemGroup, ItemGroupEvents, ItemGroupRegistry, ItemStack, ModifyEntries};
pub use crate::pure::loot_api_v2::{self, LootTableContext, LootTableEvents};
pub use crate::pure::networking_api_v1::{self, PlayerConnection, ServerPlayConnectionEvents};
pub use crate::pure::player_api::{self, EffectInstance, FoodStatus, MemoryPlayerOps, PlayerOps, ServerPlayer, with_ops};
pub use crate::pure::screen_handler_api_v1::{
    self, MenuButton, ScreenCanvas, ScreenClick, ScreenDraw, ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout, ScreenTooltip,
};
pub use crate::pure::projectile_events_v1::{self, ProjectileEvents, ProjectileImpact};
pub use crate::pure::world_api_v1::{self, BlockState, MemoryWorldOps, World, WorldOps, with_world};
pub use crate::pure::resource_loader_v0::{self, ResourceManagerHelper, ResourceType, reload_event};
