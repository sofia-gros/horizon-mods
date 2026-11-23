//! What `pluto_lib::` names. Shared by the real crate (`lib.rs`) and by the host-independent test crate (`tests/pure`), so the documentation examples are compiled against exactly the names a mod author sees.

pub use crate::pure::base;
pub use crate::pure::base::event::{AllowAll, FirstNotPass, Unit};
pub use crate::pure::base::support::{MATRIX, entries_with};
pub use crate::pure::base::{
    ActionResult, Callback, Nbt, NbtCompound, NbtCompoundExt, Combiner, DEFAULT_NAMESPACE, Event, Identifier, IdentifierError, PhaseOrderingCycle, Support, SupportEntry,
    default_phase,
};
pub use crate::pure::block_api_v1::{
    self, BlockBehavior, BlockBehaviors, BlockEditor, BlockView, BonemealContext, PickContext, PlaceOnContext, TickContext, Direction, EntityInsideContext, EntityInsideMotionContext, FallOnContext, AfterFallContext, MemoryBlockView, Placement, PlacementContext, ShapeUpdateContext,
    SurviveContext, PathNodeType, PathNodeTypeContext, PistonBehavior,
};
pub use crate::pure::block_entity_api_v1::{self, BlockEntityClientTick, BlockEntityDrain, BlockEntityFill, BlockEntityPlace, BlockEntitySignal, BlockEntityTransfer, TransferMode, FluidFilter, FluidForward, ItemFilter, ItemForward, SidedAccess, BlockEntityTick, BlockEntityUse, BlockUseOutcome, BlockEntityType, BlockEntityTypeRegistry, BlockPos, FluidStack, ItemSlot};
pub use crate::pure::block_entity_renderer_api_v1::{self, BlockEntityCanvas, BlockEntityRender, BlockEntityRendererRegistry, ItemDisplayContext, ModelPart, ModelPolygon, ModelVertex, PartPose, CubeDefinition, PartDefinition, LayerDefinition, EntityModelLayerRegistry};
pub use crate::pure::color_provider_api_v1::{self, BlockColorContext, ItemColorContext, ColorProviderRegistry};
pub use crate::pure::armor_renderer_api_v1::{self, ArmorLayer, ArmorLayerKind, ArmorRenderContext, ArmorRenderer};
pub use crate::pure::command_api_v2::{self, CommandContext, CommandRegistrar, CommandRegistrationCallback};
pub use crate::pure::config_api::{self, ConfigEvents, ConfigKind, ConfigRegistry, ConfigSpec, ModConfig};
pub use crate::pure::particle_factory_registry::{self, ParticleFactoryRegistry};
pub use crate::pure::particle_api_v1::{self, ParticleData, ParticleFactory, ParticleTypeRegistry};
pub use crate::pure::skull_model_registry::{self, SkullModelAnimation, SkullModelRegistry};
pub use crate::pure::data_attachment_api_v1::{self, AttachmentRegistry, AttachmentType};
pub use crate::pure::entity_events_v1::{self, sky_darken, effect_category, EffectCategory, StackSlot, StackDataEdit, armor_stack_slot, ActionTarget, CarriedEdit, CarriedSlot, CarriedStack, CounterDamage, EntityAction, KnockbackDirection, DamageModification, EffectGrant, PlayerTick, EntityInfo, EntityState, WeaponHit, WearerHit, ShearAttempt, ItemDamage, KnockbackModification, LivingDamage, VisibilityModification, ServerEntityEvents, ServerLivingEntityEvents, MobSpawnInfo, MobSpawnPreparation, MobSpawnCompletion, MobSpawnAction, MobSpawnCallback};
pub use crate::pure::entity_type_api::{self, EntityTypeRegistry};
pub use crate::pure::living_entity_drops::{self, LivingDropInfo, LivingEntityDrops, LivingEntityDropsCallback};
pub use crate::pure::living_entity_behavior::{self, LivingEntityBehavior, LivingEntityBehaviors, JumpPowerContext, AttackDamageContext, CalculateFallDamageContext, CauseFallDamageContext, FallOutcome, EquipmentDropChanceContext, AttributeModifierOperation, TransientAttributeModifier, AttributeBaseValue, LivingEntityLoadContext};
pub use crate::pure::mob_split_api::{self, MobSplitContext, MobSplitRandom, MobSplitChild, MobSplitInheritance, MobSplitCallback, MobSplitCallbacks};
pub use crate::pure::spawn_placement_api::{self, SpawnPlacementContext, SpawnPlacementRegistry};
pub use crate::pure::hanging_api_v1::{self, FrameVariant, HangingRegistry, StandVariant};
pub use crate::pure::events_interaction_v0::{
    self, AttackEntityCallback, AttackEntityContext, BreakSpeedCallback, BreakSpeedContext, DiggingPlayer, BlockBreakContext, FishingContext, PlayerBlockBreakEvents, PlayerFishEvents, UseBlockCallback, UseBlockContext,
    UseEntityCallback, InteractEntityCallback, InteractEntityContext, UseItemCallback, UseItemContext, UseItemFinishCallback, UseItemFinishContext, UseItemOnBlockCallback, UseItemOnBlockContext, UseItemOnEntityContext, Hand, FISHING_BITE, FISHING_CAST,
};
pub use crate::pure::lifecycle_events_v1::{
    self, ServerChunk, ServerChunkEvents, ServerInfo, ServerLifecycleEvents, ServerTickEvents, WorldTick,
};
pub use crate::pure::fluid_api_v1::{self, FluidAttributes, FluidDef, FluidFlow, FluidRegistry, FluidRegistrySnapshot};
pub use crate::pure::fluid_item_api_v1::{self, Drained, Filled, FluidItemDef, FluidItems, FluidTarget, Interaction, TransferDirection};
pub use crate::pure::fluid_world_api_v1::{self, FluidWorld};
pub use crate::pure::i18n_api::{self, FALLBACK_LANGUAGE, I18n, LanguageChange, LanguageEvents, Translations};
pub use crate::pure::item_api_v1::{self, ItemTooltipCallback, ItemTooltipContext, Food, ItemUse, ItemUseRegistry, StackUse, ItemRegistry, ItemRegistrySnapshot, StackedOnOtherCallback, StackedOnOtherDecision, StackedOnOtherRegistry, StackedOnOtherRequest, TooltipLine, TooltipText, UseAction, StandingAndWallBlockItem, EnchantabilityContext, EnderMaskContext, ItemBehavior, ItemBehaviors, MaxStackSizeContext, ToolActionContext};
pub use crate::pure::item_group_api_v1::{self, ItemGroup, ItemGroupEvents, ItemGroupRegistry, ItemStack, ModifyEntries};
pub use crate::pure::loot_api_v2::{self, LootDrops, LootTableContext, LootTableEvents};
pub use crate::pure::networking_api_v1::{self, PlayerConnection, ServerPlayConnectionEvents, ServerPlayNetworking, ClientPlayNetworking, ClientPlayConnectionEvents, MAX_PLAY_PAYLOAD};
pub use crate::pure::player_api::{self, CustomModelData, EffectInstance, FoodStatus, InventoryTick, KeyPress, MemoryPlayerOps, PlayerInventoryEvents, PlayerKeyEvents, PlayerOps, ServerPlayer, StackData, with_ops};
pub use crate::pure::screen_handler_api_v1::{
    self, ContainerAccept, MenuButton, MenuAccess, SlotAction, SlotClick, QuickMove, ScreenCanvas, ScreenClick, ScreenDraw, ScreenHandlerRegistry, ScreenHandlerType, ScreenLayout, ScreenLayoutContext, ScreenTooltip, SlotRule,
};
pub use crate::pure::trade_offer_helper::{self, TradeOfferHelper, TradeOffer, TradeOfferFactory, TradeOfferContext, TradeRandom, TradeWorld};
pub use crate::pure::projectile_events_v1::{self, AmmoRequest, BowShot, ProjectileEvents, ProjectileImpact, ProjectileLaunch};
pub use crate::pure::fluid_storage_api_v1::{self, BlockEntityStore, FluidStorage, MemoryBlockEntities};
pub use crate::pure::item_storage_api_v1::{self, ItemStorage};
pub use crate::pure::neighbour_api_v1::{self, MemoryNeighbourOps, NeighbourOps, NeighbourSlot, NeighbourTank, Neighbours, with_neighbours};
pub use crate::pure::entity_api_v1::{self, EntityOps, EntityReach, LivingFacts, MemoryEntityOps, WorldEntity, with_entities};
pub use crate::pure::world_api_v1::{self, BlockState, ExplosionInteraction, ExplosionRecord, MemoryWorldOps, ParticleRecord, Region, SummonRecord, World, WorldOps, with_ticking_dimension, with_world};
pub use crate::pure::worldgen_api_v1::{self, ConfiguredFeature, FeatureContext, FeatureTypeRegistry, TreeDecoration, TreeDecorator, TreeDecoratorContext, TreeDecoratorTypeRegistry, WorldgenBindings, RootPlacementWorld, RootPlacementContext, RootPlacement, RootPlacementTypeRegistry, NativeTreeRoute, native_tree_route, ConfiguredStructure, StructureContext, StructurePiece, StructureStart, StructureTypeRegistry, StructureWorld, ConfiguredStructurePiece, StructureMarkerContext, StructureMirror, StructurePieceTemplate, StructurePieceTypeRegistry, StructurePieceWorld, StructureRotation};
pub use crate::pure::resource_loader_v0::{self, ResourceImage, RegistryLookup, JsonResourceStack, ResourceManagerHelper, ResourceType, reload_event};
pub use crate::pure::advancement_api_v1::{self, DispatchResult, EventMatcher, ItemSnapshot, PredicateCallback, PredicateRegistration, PredicateRequest, PredicateResult, QueuedEvent, RegistrationError, TagLookup};
pub use crate::pure::recipe_api_v1::{self, AssemblyCallback, AssemblyDecision, AssemblyEvaluation, AssemblyInterest, AssemblyOutcome, AssemblyRegistration, GridFacts, IngredientKind, RecipeFacts, RecipeForm, SpecialCallback, SpecialRecipeRegistration, SpecialResult, StackView, evaluate_special_recipe, has_assembly_providers, has_ingredient_kinds, has_special_recipes, ingredient_items, ingredient_test, prepare_assembly, register_assembly, register_ingredient_kind, register_special_recipe};
pub use crate::pure::book_api_v1::{self, Book, BookAppearance, BookClick, BookIcon, BookImage, BookOps, BookPage, BookRegistry, BookRow, BookSection, BookText, BookView, GridSize, MemoryBookOps, SectionPageRef, book_click, close_book, current_book, draw_book, open_book, section_page_refs, with_books, wrap_text};
pub use crate::pure::projectile_api_v1::{self, ProjectileSpawn, ThrownType, ThrownTypeRegistry, spawn_projectile, spawn_thrown, uuid_int_array};
