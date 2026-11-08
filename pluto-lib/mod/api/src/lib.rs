//! PlutoLib: Horizon's Fabric API.
//!
//! An independent mod that other mods call (they declare `pluto_lib` under `[dependencies]` in `horizon-mixin.toml`). Names and
//! behaviour follow Fabric API; see `pluto-lib/docs/design-principles.md` and the specification. Modules mirror Fabric API
//! artifacts (`lifecycle_events_v1` is `fabric-lifecycle-events-v1`). The host-independent part is [`pure`]; [`hooks`] is the
//! one file that connects it to Horizon.

mod hooks;
pub mod pure;

pub use crate::hooks::hooks;
pub use crate::hooks::{no_hooks, ModHooks};
pub use crate::pure::base;
pub use crate::pure::base::{
    ActionResult, Callback, Combiner, DEFAULT_NAMESPACE, Event, Identifier, IdentifierError, PhaseOrderingCycle, Support, SupportEntry,
    default_phase,
};
pub use crate::pure::command_api_v2::{self, CommandContext, CommandRegistrar, CommandRegistrationCallback};
pub use crate::pure::config_api::{self, ConfigEvents, ConfigKind, ConfigRegistry, ConfigSpec, ModConfig};
pub use crate::pure::data_attachment_api_v1::{self, AttachmentRegistry, AttachmentType};
pub use crate::pure::entity_events_v1::{self, EntityInfo, LivingDamage, ServerEntityEvents, ServerLivingEntityEvents};
pub use crate::pure::events_interaction_v0::{
    self, AttackEntityCallback, AttackEntityContext, BlockBreakContext, PlayerBlockBreakEvents, UseBlockCallback, UseBlockContext,
    UseItemCallback, UseItemContext,
};
pub use crate::pure::lifecycle_events_v1::{
    self, ServerChunk, ServerChunkEvents, ServerInfo, ServerLifecycleEvents, ServerTickEvents, WorldTick,
};
pub use crate::pure::item_api_v1::{self, ItemTooltipCallback, ItemTooltipContext, TooltipLine};
pub use crate::pure::loot_api_v2::{self, LootTableContext, LootTableEvents};
pub use crate::pure::networking_api_v1::{self, PlayerConnection, ServerPlayConnectionEvents};
pub use crate::pure::player_api::{self, PlayerOps, ServerPlayer, with_ops};
pub use crate::pure::resource_loader_v0::{self, ResourceManagerHelper, ResourceType};
