//! The only file that names Horizon types: it turns Horizon's hook points into PlutoLib's Fabric-like events.
//!
//! Hook points are Horizon's stable, only-ever-added surface (`native-hooks.json`). A hook that can veto returns
//! `Flow::Cancel` when the PlutoLib callbacks say so.

use horizon_client::mod_api::{
    AddServerReloadListenersEvent, ServerReloadListenerApplyEvent, NativeReloadListener,
    AttackEntityEvent, ChunkLoadEvent, ClientStartEvent, ClientResourcesReloadedEvent, ServerDataLoadedEvent, LootTableLoadEvent, Text, TooltipEvent, ChunkUnloadEvent, DestroyBlockEvent, EntityJoinLevelEvent, EntityLeaveLevelEvent, Flow, Hooks,
    Host, LevelTickPostEvent, LevelTickPreEvent, LivingIncomingDamageEvent, PlayerLoggedInEvent, PlayerLoggedOutEvent,
    UseAnimation, UseAnimationEvent, UseDurationEvent, NativeCommand, PerformCommandEvent, RegisterCommandsEvent as RegisterCmds, ConfigLoadingEvent, ConfigReloadingEvent, ConfigUnloadingEvent, NativeConfig, NativeConfigFile, NativeConfigKind, RegisterConfigsEvent,
    RightClickBlockEvent, ServerAboutToStartEvent, ServerStoppingEvent, ServerTickEvent, ServerTickPhase, LivingDeathEvent, BreakSpeedEvent, LivingDamagePreEvent, LivingEntityUseItemFinishEvent, BlockCloneItemEvent, UseItemEvent, UseItemOnEvent,
    HearsProjectileImpactsEvent, ProjectileImpactEvent, BlockEntityDraw, BlockEntitySubmitEvent, CubeFace, DisplayContext, MenuButtonV2Event, MenuSlotRulesEvent, MenuQuickMoveEvent, ContainerAcceptsEvent, ModBlockEntity, NativeScreen, NativeStack, ScreenClickEvent, ScreenDrawEvent, ScreenEvent, ScreenTooltipEvent, ScreenRenderPostEvent, ScreenMouseButtonPressedPreEvent, ScreenMouseScrolledPreEvent, ScreenKeyPressedPreEvent, ScreenClosingEvent, BlockEntityContainerEvent, BlockEntityFluidsEvent, ModContainer, ModFluidTanks, BlockEntityEvent, BlockEntityTickEvent, BlockEntityTicksEvent, BlockEntityUpdateTagEvent, CustomBlockFluidsEvent, CustomFluidTanksEvent, CustomFluidFillEvent, CustomFluidDrainEvent, NativeFluidAction, NativeFluidStack, NativeFluidTankView, NativeItemEntity, RightClickBlockV2Event, Hand, EquipmentSlot, AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, Tag,
    PlaceBlockEvent, LivingGetProjectileEvent, KeyEvent, InteractEntityV2Event, PlayerFishV2Event, LeftClickBlockEvent,
    InteractEntityEvent, InventoryTickEvent, EntityTickPreEvent, EntityTickPostEvent,
};

use crate::pure::entity_events_v1::{self, LivingDamage};
use crate::pure::advancement_api_v1;
use crate::pure::player_api::{self, PlayerOps};
use horizon_client::items::ItemStack;
use horizon_client::pluto_host_world as horizon_world;
use crate::pure::base::{Identifier, Nbt, NbtCompound};
use crate::pure::block_entity_api_v1::{self, FluidStack, ItemSlot};
use crate::pure::neighbour_api_v1::{self, NeighbourOps, NeighbourSlot, NeighbourTank};
use crate::pure::entity_api_v1::{self, EntityOps};
use crate::pure::fluid_storage_api_v1::{BlockEntityStore, FluidStorage};
use crate::pure::item_storage_api_v1::ItemStorage;
use crate::pure::block_entity_renderer_api_v1::{self, BlockEntityCanvas, ItemDisplayContext, ModelPart, ModelPolygon, ModelVertex, PartPose};
use crate::pure::screen_handler_api_v1::{self, ScreenCanvas};
use crate::pure::player_api::{EffectInstance, FoodStatus};
use crate::pure::world_api_v1::{self, BlockState, WorldOps};
use crate::pure::book_api_v1::{self, BookOps, BookView};
use crate::pure::{projectile_events_v1, projectile_events_v1::ProjectileImpact};
use crate::pure::block_api_v1;
use crate::pure::command_api_v2;
use crate::pure::config_api::{self, ConfigKind, ModConfig};
use crate::pure::{events_interaction_v0, item_api_v1, lifecycle_events_v1, loot_api_v2, networking_api_v1, resource_loader_v0};

struct StructureQueries<'a>(&'a mut dyn horizon_client::entry::pluto_block::structures::PlanningWorld);
impl crate::pure::worldgen_api_v1::StructureWorld for StructureQueries<'_> {
    fn top_of_chunk_center(&mut self,chunk:[i32;2])->Result<Option<[i32;3]>,String>{self.0.top_of_chunk_center(chunk)}
    fn is_valid_biome(&mut self,pos:[i32;3])->Result<bool,String>{self.0.is_valid_biome(pos)}
    fn first_occupied_height(&mut self,x:i32,z:i32)->Result<i32,String>{self.0.first_occupied_height(x,z)}
    fn generation_depth(&self)->i32{self.0.generation_depth()}
    fn sea_level(&self)->i32{self.0.sea_level()}
    fn random_int(&mut self,bound:i32)->Result<i32,String>{self.0.random_int(bound)}
}

struct StructureMarkerWorld<'a>(&'a mut dyn horizon_client::entry::pluto_block::structures::MarkerWorld);
impl crate::pure::worldgen_api_v1::StructurePieceWorld for StructureMarkerWorld<'_> {
    fn block_state(&mut self,pos:[i32;3])->Result<BlockState,String> {
        let state=self.0.state(pos)?;to_state(&state).ok_or_else(||"invalid structure marker block identifier".into())
    }
    fn set_block_state(&mut self,pos:[i32;3],state:&BlockState,flags:i32)->Result<bool,String>{self.0.set_state(pos,&to_info(state),flags)}
    fn face_sturdy(&mut self,pos:[i32;3],face:block_api_v1::Direction)->Result<bool,String>{self.0.face_sturdy(pos,face as u8)}
    fn can_survive_plant(&mut self,state:&BlockState,pos:[i32;3])->Result<bool,String>{self.0.can_survive_plant(&to_info(state),pos)}
    fn random_int(&mut self,bound:i32)->Result<i32,String>{self.0.random_int(bound)}
    fn random_bool(&mut self)->Result<bool,String>{self.0.random_bool()}
    fn place_feature(&mut self,feature:&Identifier,pos:[i32;3])->Result<bool,String>{self.0.place_feature(&feature.to_string(),pos)}
}
struct ApiStructurePiece(crate::pure::worldgen_api_v1::ConfiguredStructurePiece);
impl horizon_client::entry::pluto_block::structures::PieceCallbacks for ApiStructurePiece {
    fn validate_state(&self,state:&serde_json::Value)->Result<(),String> {self.0.validate_state(state)}
    fn marker(&self,world:&mut dyn horizon_client::entry::pluto_block::structures::MarkerWorld,name:&str,pos:[i32;3],state:&mut serde_json::Value)->Result<(),String> {
        self.0.handle_data_marker(&mut crate::pure::worldgen_api_v1::StructureMarkerContext {
            world:&mut StructureMarkerWorld(world),marker:name,position:pos,state,
        })
    }
}

/// PlutoLib's entry point: the launcher calls it before the hooks of every mod that depends on PlutoLib.
pub fn hooks() -> Box<dyn Hooks> {
    horizon_client::menu::pluto_menu_actions::set_handler(staged_menu_click);
    horizon_client::menu::pluto_menu_actions::set_pick_all_exclusion(|menu, slot| {
        block_entity_api_v1::block_of_menu(menu)
            .is_some_and(|block| screen_handler_api_v1::excluded_from_pick_all(&block, slot))
    });
    crate::pure::i18n_api::set_host(crate::pure::i18n_api::HostLanguage {
        code: horizon_client::lang::code,
        get: |key| horizon_client::lang::has(key).then(|| horizon_client::lang::get(key)),
    });
    horizon_client::mesher::pluto_block_colors::set_provider(
        crate::pure::color_provider_api_v1::has_block_provider,
        crate::pure::color_provider_api_v1::block_color_from_host,
    );
    horizon_client::item_render::pluto_item_colors::set_provider(
        crate::pure::color_provider_api_v1::has_item_provider,
        |item,count,components,tint_index| {
            let count=u32::try_from(count).ok()?;
            let custom_data=match components.get("minecraft:custom_data") {
                None=>NbtCompound::new(),
                Some(value)=>match horizon_client::scoreboard::json_to_tag(value) {
                    Tag::Compound(data)=>compound_from_tag(&data),
                    _=>return None,
                },
            };
            let damage=match components.get("minecraft:damage") {
                None=>0,
                Some(value)=>u32::try_from(value.as_u64()?).ok()?,
            };
            let max_damage=match components.get("minecraft:max_damage") {
                None=>None,
                Some(value)=>Some(i32::try_from(value.as_i64()?).ok()?),
            };
            let custom_model_data=match components.get("minecraft:custom_model_data") {
                None=>None,
                Some(value)=>match horizon_client::scoreboard::json_to_tag(value) {
                    Tag::Compound(data)=>Some(player_api::CustomModelData::from_nbt(&compound_from_tag(&data))),
                    _=>return None,
                },
            };
            crate::pure::color_provider_api_v1::ColorProviderRegistry::item_color(
                &crate::pure::color_provider_api_v1::ItemColorContext {
                    stack:player_api::StackData {item:item.into(),count,damage,max_damage,custom_data,custom_model_data,enchantments:Default::default()},tint_index,
                })
        },
    ).expect("PlutoLib item colour provider installs once before native rendering");
    horizon_client::item_render::pluto_armor_layers::set_provider(
        crate::pure::armor_renderer_api_v1::has_renderer,
        |item,count,components,kind| crate::pure::armor_renderer_api_v1::host_layers(worn_stack(item,count,components)?,kind),
    ).expect("PlutoLib armor layer provider installs once before native rendering");
    horizon_client::mobs::pluto_entity_particles::set_provider(
        crate::pure::particle_factory_registry::entity_particle_from_host,
    ).expect("PlutoLib entity particle provider installs once");
    horizon_client::game::pluto_item_particles::set_provider(|particle| {
        let data=crate::pure::particle_factory_registry::item_from_host(particle)?;
        let mut stack=ItemStack::new(data.item.as_str(),data.count);
        write_stack(&mut stack,&data);
        Some(stack)
    }).expect("PlutoLib item particle factory provider installs once");
    horizon_client::game::pluto_living_drops::set_provider(
        crate::pure::living_entity_drops::has_callbacks,
        |context| {
            let info=context.info().clone();
            let info=crate::pure::living_entity_drops::LivingDropInfo {
                entity:entity_events_v1::EntityInfo {id:info.entity_id,kind:info.entity_type,dimension:info.dimension},
                position:info.position,damage_type:info.damage_type,killer_type:info.killer_type,
            };
            crate::pure::living_entity_drops::dispatch(&mut ApiLivingDrops {context,info})
        },
    ).expect("PlutoLib living drop provider installs once");
    horizon_client::game::pluto_server_stopped::set_dispatcher(lifecycle_events_v1::server_stopped);
    horizon_client::entry::pluto_block_entity_access::set_predicate(block_entity_api_v1::registered_type)
        .expect("PlutoLib block entity predicate installs once before native initialization");
    horizon_client::entry::pluto_block_entity_access::set_cache_generation_provider(crate::pure::render_cache::generation)
        .expect("PlutoLib render cache generation provider installs once");
    horizon_client::entry::pluto_block_entity_access::set_client_tick_sink(
        horizon_client::entry::pluto_block_entity_access::ClientTickSink {
            has: |block| block_entity_api_v1::client_ticks(block) == Some(true),
            tick: |block, pos, saved, transient, view| {
                let saved = compound_from_tag(saved);
                let mut local = compound_from_tag(transient);
                let ran = block_entity_api_v1::client_tick(block, pos, &saved, &mut local, &GameView(view), true);
                if ran { *transient = compound_to_tag(&local); }
                ran
            },
        }).expect("PlutoLib client block entity tick sink installs once");
    horizon_client::entry::pluto_block::set_item_placement_provider(|| {
        Ok(item_api_v1::standing_wall_bindings()?.into_iter().map(|binding|
            horizon_client::entry::pluto_block::ItemPlacementBinding {
                item: binding.item.to_string(), standing: binding.standing.to_string(),
                wall: binding.wall.to_string(), attachment: binding.attachment as u8,
            }).collect())
    }).expect("PlutoLib item placement provider installs once before table loading");
    horizon_client::entry::pluto_living_load_access::set_provider(horizon_client::entry::pluto_living_load_access::Provider {
        has:crate::pure::living_entity_behavior::LivingEntityBehaviors::has_load_bases,
        bases:|host,info| {
        scoped(host,|| {
            use crate::pure::living_entity_behavior::{LivingEntityBehaviors,LivingEntityLoadContext};
            let context=LivingEntityLoadContext {entity:entity_events_v1::EntityInfo {
                id:info.entity,kind:info.kind.clone(),dimension:info.dimension.clone()},slime_size:info.slime_size};
            Ok(LivingEntityBehaviors::attribute_bases_on_load(&context)?.into_iter().map(|base|
                horizon_client::entry::pluto_living_load_access::AttributeBase {attribute:base.attribute.to_string(),value:base.value}).collect())
        })
    }}).expect("PlutoLib living load provider installs once");
    horizon_client::entry::pluto_player_hurt_access::set_sink(horizon_client::entry::pluto_player_hurt_access::Sink {
        hurt: |hurt| {
            let hit = entity_events_v1::WearerHit {
                attacker: hurt.attacker, kind: hurt.attacker_kind.clone(), damage_type: hurt.damage_type.clone(), amount: hurt.amount,
                attacker_state: entity_events_v1::EntityState {
                    kind: hurt.attacker_kind.clone(), sprinting: false, on_ground: hurt.on_ground, in_water: hurt.in_water,
                    fire_ticks: hurt.fire_ticks, fire_immune: hurt.fire_immune, effects: hurt.effects.clone(),
                    ..entity_events_v1::EntityState::default()
                },
                armor: entity_events_v1::armor_of_the_only_wearer(), effects: std::cell::RefCell::new(Vec::new()), self_effects: std::cell::RefCell::new(Vec::new()), counter_damage: std::cell::RefCell::new(Vec::new()), actions: std::cell::RefCell::new(Vec::new()),
            };
            // The damage event that follows carries the attacker's state, for the protection that conditions on it.
            let wearer = entity_events_v1::only_wearer_entity();
            if let Some(wearer) = wearer { entity_events_v1::hit_states(wearer, Some(hit.attacker_state.clone()), None); }
            entity_events_v1::wearer_hit(&hit, wearer).into_iter().map(|grant| (grant.effect, grant.duration, grant.amplifier)).collect()
        },
    }).expect("PlutoLib player hurt sink installs once");
    horizon_client::mobs::pluto_player_visibility_access::set_sink(horizon_client::mobs::pluto_player_visibility_access::Sink {
        factor: entity_events_v1::modify_visibility,
    }).expect("PlutoLib player visibility sink installs once");
    horizon_client::mobs::pluto_living_entity_access::set_sink(horizon_client::mobs::pluto_living_entity_access::Sink {
        attributes:|kind| {
            let kind=Identifier::parse(kind).map_err(|e|e.to_string())?;
            Ok(crate::pure::living_entity_behavior::LivingEntityBehaviors::transient_attribute_modifiers(&kind)?
                .into_iter().map(|m|horizon_client::mobs::pluto_living_entity_access::AttributeModifier {
                    attribute:m.attribute.to_string(),id:m.id.to_string(),amount:m.amount,operation:m.operation as u8,
                }).collect())
        },
        has_fall:crate::pure::living_entity_behavior::LivingEntityBehaviors::has_fall,
        fall:|entity,kind,distance,multiplier,inherited| {
            let entity_type=Identifier::parse(kind).ok()?;
            crate::pure::living_entity_behavior::LivingEntityBehaviors::calculate_fall_damage(
                &crate::pure::living_entity_behavior::CalculateFallDamageContext {entity,entity_type,distance,multiplier,inherited})
        },
        has_jump:crate::pure::living_entity_behavior::LivingEntityBehaviors::has_jump,
        jump:|entity,kind,slime_size,block_jump_factor| {
            let entity_type=Identifier::parse(kind).ok()?;
            crate::pure::living_entity_behavior::LivingEntityBehaviors::jump_power(
                &crate::pure::living_entity_behavior::JumpPowerContext {entity,entity_type,slime_size,block_jump_factor})
        },
        has_attack:crate::pure::living_entity_behavior::LivingEntityBehaviors::has_attack,
        attack:|entity,kind,attribute| {
            let entity_type=Identifier::parse(kind).ok()?;
            crate::pure::living_entity_behavior::LivingEntityBehaviors::attack_damage(
                &crate::pure::living_entity_behavior::AttackDamageContext {entity,entity_type,attribute})
        },
    }).expect("PlutoLib living query sink installs once");
    horizon_client::entry::pluto_block::set_skull_definition_provider(|| {
        Ok(crate::pure::skull_model_registry::definitions()?.into_iter().map(|d|
            horizon_client::entry::pluto_block::SkullDefinition {
                item:d.item.to_string(),layer:d.layer,texture:d.texture.to_string(),animation:d.animation as u8,
            }).collect())
    }).expect("PlutoLib skull definitions install before item models load");
    horizon_client::entry::pluto_block::set_skull_part_provider(|| {
        let definitions=horizon_client::entry::pluto_block::skull_definitions()?;
        let requested=definitions.iter().filter(|d|!d.layer.starts_with("minecraft:"))
            .map(|d|d.layer.clone()).collect();
        let layers=block_entity_renderer_api_v1::model_layer_definitions_for(&requested)?;
        let mut models=Vec::new();
        for definition in definitions.iter().filter(|d|!d.layer.starts_with("minecraft:")) {
            let layer=layers.iter().find(|(id,_)|id==&definition.layer)
                .map(|(_,layer)|layer).ok_or_else(||format!("{}: skull layer not registered",definition.layer))?;
            models.push((definition.item.clone(),skull_model_to_native(&layer_to_native(layer).bake_root())));
        }
        Ok(models)
    }).expect("PlutoLib skull geometry installs before item renderers load");
    horizon_client::entry::pluto_block::set_block_settings_provider(|| {
        Ok(block_api_v1::piston_behavior_declarations()?.into_iter().map(|(block,piston)|
            horizon_client::entry::pluto_block::BlockSettingsDeclaration {block,piston:piston as u8}
        ).collect())
    }).expect("PlutoLib static block settings provider installs before native content loading");
    horizon_client::entry::pluto_block::set_entity_type_provider(|| {
        Ok(crate::pure::entity_type_api::derived_entity_types()?.into_iter().map(|d|
            horizon_client::entry::pluto_block::DerivedEntityDefinition {
                id: d.id.to_string(), factory: d.factory.to_string(), width: d.width, height: d.height,
                tracking_chunks: d.tracking_chunks, velocity_updates: d.velocity_updates,
            }).collect())
    }).expect("PlutoLib entity type provider installs once before native table loading");
    horizon_client::entry::pluto_block::set_mask_dispatcher(|item, wearer, observer| {
        item_api_v1::ender_mask(&item_api_v1::EnderMaskContext {
            item: item.to_owned(), stack: entity_events_v1::worn_piece(wearer, "head").filter(|stack| stack.item == item), wearer_uuid: Some(wearer), observer_entity: Some(observer),
        })
    }).expect("PlutoLib item mask dispatcher installs once");
    horizon_client::entry::pluto_block::set_max_stack_dispatcher(|item, data| {
        item_api_v1::max_stack_size(&item_api_v1::MaxStackSizeContext {
            item: item.to_owned(), custom_data: nbt_from_json(data),
        })
    }).expect("PlutoLib item max stack dispatcher installs once");
    horizon_client::entry::pluto_block::set_enchantability_dispatcher(|item, data| {
        item_api_v1::enchantability(&item_api_v1::EnchantabilityContext {
            item: item.to_owned(), custom_data: nbt_from_json(data),
        })
    }).expect("PlutoLib item enchantability dispatcher installs once");
    horizon_client::entry::pluto_block::set_tool_action_dispatcher(|item, data, action| {
        item_api_v1::can_perform_action(&item_api_v1::ToolActionContext {
            item: item.to_owned(), action: action.to_owned(), custom_data: nbt_from_json(data),
        })
    }).expect("PlutoLib item tool action dispatcher installs once");
    horizon_client::entry::pluto_block::set_schedule_drain(|dimension, edit| {
        world_api_v1::apply_pending_block_ticks(dimension, |pos, block, delay| edit.schedule_tick(pos, block, delay));
        // The extra blocks a break's callbacks asked for go through the game's own destroy path, drops and all.
        events_interaction_v0::apply_pending_extra_breaks(dimension, |pos| edit.break_block(pos, true) == Some(true));
    });
    horizon_client::entry::pluto_entity_spawn_access::set_provider(
        horizon_client::entry::pluto_entity_spawn_access::Provider {
            enabled:crate::pure::entity_spawn_api::has_callbacks,prepare:native_spawn_prepare,
        }).expect("PlutoLib mob spawn dispatcher installs once");
    horizon_client::merchant::pluto_trade_access::set_provider(native_trade_factories)
        .expect("PlutoLib trade factory provider installs once");
    let blocks = block_sink();
    horizon_client::entry::pluto_block::set_survival_dispatcher(blocks.survives);
    horizon_client::entry::pluto_block::set_sink(blocks);
    horizon_client::entry::pluto_block::structures::set_dispatcher(|kind| {
        let id=Identifier::parse(kind).map_err(|_|format!("invalid structure type: {kind}"))?;
        crate::pure::worldgen_api_v1::StructureTypeRegistry::is_registered(&id)
    },|kind,configuration,world,chunk| {
        use crate::pure::worldgen_api_v1::{StructureContext,StructureTypeRegistry};
        use horizon_client::entry::pluto_block::structures::PlanResult;
        let id=Identifier::parse(kind).map_err(|_|format!("invalid structure type: {kind}"))?;
        let Some(structure)=StructureTypeRegistry::decode(&id,configuration)? else {return Ok(PlanResult::Unregistered)};
        let Some(start)=structure.find_generation_point(&mut StructureContext {world:&mut StructureQueries(world),chunk})? else {return Ok(PlanResult::Rejected)};
        // Move opaque JSON without recursively serializing unvalidated captures.
        let pieces=start.pieces.into_iter().map(|piece|serde_json::Value::Object(serde_json::Map::from_iter([
            ("kind".into(),serde_json::Value::String(piece.kind.to_string())),
            ("configuration".into(),piece.configuration),
        ]))).collect();
        let position=serde_json::Value::Array(start.position.into_iter().map(|v|serde_json::Value::from(v)).collect());
        Ok(PlanResult::Start(serde_json::Value::Object(serde_json::Map::from_iter([
            ("position".into(),position),("pieces".into(),serde_json::Value::Array(pieces)),
        ]))))
    });
    horizon_client::entry::pluto_block::structures::set_piece_decoder(|kind,configuration| {
        use crate::pure::worldgen_api_v1::{StructurePieceTypeRegistry,StructureRotation,StructureMirror};
        use horizon_client::entry::pluto_block::structures::{RegisteredTemplate,validate_value};
        let id=Identifier::parse(kind).map_err(|_|format!("invalid structure piece type: {kind}"))?;
        let Some(piece)=StructurePieceTypeRegistry::decode(&id,configuration)? else {return Ok(None)};
        validate_value(piece.initial_state())?;
        let template=piece.template();
        Ok(Some(RegisteredTemplate {
            template:template.template.to_string(),origin:template.origin,
            rotation:match template.rotation {StructureRotation::None=>0,StructureRotation::Clockwise90=>1,StructureRotation::Clockwise180=>2,StructureRotation::Counterclockwise90=>3},
            mirror:match template.mirror {StructureMirror::None=>0,StructureMirror::LeftRight=>1,StructureMirror::FrontBack=>2},
            initial_state:piece.initial_state().clone(),callbacks:std::sync::Arc::new(ApiStructurePiece(piece)),
        }))
    });
    horizon_client::entry::pluto_block::set_feature_dispatcher(|world, kind, configuration, origin| {
        use crate::pure::worldgen_api_v1::{FeatureContext, FeatureTypeRegistry};
        let id = Identifier::parse(kind).ok()?;
        match FeatureTypeRegistry::generate(&id, configuration, &mut FeatureContext { world: &mut GameEditor(world), origin }) {
            Ok(result) => result,
            Err(error) => {
                eprintln!("pluto_lib: feature type {id} rejected configuration: {error}");
                Some(false)
            }
        }
    });
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
    horizon_client::pluto_inventory::advancements::predicates::register_item_predicate(
        horizon_client::pluto_inventory::advancements::predicates::PredicateRegistration {
            id: "pluto_lib:advancement_api_v1",
            callback: advancement_predicate_forward,
            inventory_components: false,
        },
    )
    .expect("PlutoLib advancement predicate forwarder installs once");
    horizon_client::game::pluto_advancement_event_source::set_event_source(advancement_event_source)
        .expect("PlutoLib advancement event source installs once");
    Box::new(PlutoHooks)
}

/// The native half of the pure advancement API's item predicates: the host's
/// dispatch consults only the native inventory-api registry, so this
/// forwarder carries evaluations into the pure registry, converting the
/// native component facts into the pure snapshot. A pure conflict fails
/// closed, exactly as the host dispatch treats a native one.
fn advancement_predicate_forward(
    request: &horizon_client::pluto_inventory::advancements::predicates::PredicateRequest<'_>,
) -> horizon_client::pluto_inventory::advancements::predicates::PredicateResult {
    use horizon_client::pluto_inventory::advancements::predicates as native;
    // With no pure registrations the native matcher keeps its exact
    // behaviour, including avoiding the conversion work below.
    if !advancement_api_v1::has_item_predicates() {
        return native::PredicateResult::Unsupported;
    }
    struct Tags<'a>(&'a dyn native::TagLookup);
    impl advancement_api_v1::TagLookup for Tags<'_> {
        fn in_tag(&self, registry: &str, tag: &str, id: &str) -> bool {
            self.0.in_tag(registry, tag, id)
        }
    }
    let tags = Tags(request.tags);
    let components = request.item.components.and_then(advancement_components);
    let forwarded = advancement_api_v1::PredicateRequest {
        predicate: request.predicate,
        item: advancement_api_v1::ItemSnapshot {
            id: request.item.id,
            count: request.item.count,
            damage: request.item.damage,
            enchantments: request.item.enchantments,
            potion: request.item.potion,
            custom_name: request.item.custom_name,
            components: components.as_ref(),
        },
        tags: &tags,
    };
    match advancement_api_v1::dispatch_item_predicate(&forwarded) {
        advancement_api_v1::DispatchResult::Unsupported => native::PredicateResult::Unsupported,
        advancement_api_v1::DispatchResult::Handled(matched) => {
            native::PredicateResult::Handled(matched)
        }
        advancement_api_v1::DispatchResult::Conflict { .. } => {
            native::PredicateResult::Handled(false)
        }
    }
}

/// The pure snapshot's component tree is the stack's custom data compound:
/// take it out of the native component patch and convert it. A stack whose
/// patch carries no custom data has an empty compound; a tree that cannot
/// be represented in pure NBT (an end tag, unpaired UTF-16) is unavailable
/// rather than partial, and consumers requiring components fail closed.
fn advancement_components(
    value: &horizon_client::pluto_inventory::advancements::predicates::ComponentValue,
) -> Option<Nbt> {
    use horizon_client::pluto_inventory::advancements::predicates::ComponentValue;
    if !matches!(value, ComponentValue::Compound(_)) {
        return None;
    }
    match value.get("minecraft:custom_data") {
        Some(custom) => advancement_component(custom),
        None => Some(Nbt::Compound(NbtCompound::new())),
    }
}

fn advancement_component(
    value: &horizon_client::pluto_inventory::advancements::predicates::ComponentValue,
) -> Option<Nbt> {
    use horizon_client::pluto_inventory::advancements::predicates::ComponentValue;
    Some(match value {
        ComponentValue::End => return None,
        ComponentValue::Byte(v) => Nbt::Byte(*v),
        ComponentValue::Short(v) => Nbt::Short(*v),
        ComponentValue::Int(v) => Nbt::Int(*v),
        ComponentValue::Long(v) => Nbt::Long(*v),
        ComponentValue::Float(bits) => Nbt::Float(f32::from_bits(*bits)),
        ComponentValue::Double(bits) => Nbt::Double(f64::from_bits(*bits)),
        ComponentValue::ByteArray(v) => Nbt::ByteArray(v.clone()),
        ComponentValue::String(v) => Nbt::String(String::from_utf16(&v.0).ok()?),
        ComponentValue::List(v) => {
            Nbt::List(v.iter().map(advancement_component).collect::<Option<Vec<_>>>()?)
        }
        ComponentValue::Compound(v) => Nbt::Compound(
            v.iter()
                .map(|(key, value)| {
                    Some((String::from_utf16(&key.0).ok()?, advancement_component(value)?))
                })
                .collect::<Option<NbtCompound>>()?,
        ),
        ComponentValue::IntArray(v) => Nbt::IntArray(v.clone()),
        ComponentValue::LongArray(v) => Nbt::LongArray(v.clone()),
    })
}

/// The native advancement drain's per-tick hand-off. Forward the pure
/// registry's component-event request into the shared inventory gate, then
/// take the queued events: the drain runs at the head of the only tick
/// that evaluates inventory criteria, so the request lands before every
/// evaluation, and no event submitted since the previous tick is missed.
fn advancement_event_source() -> Vec<horizon_client::game::pluto_advancement_event_source::QueuedEvent> {
    if advancement_api_v1::inventory_component_events_requested() {
        horizon_client::pluto_inventory::advancements::request_pure_inventory_component_events();
    }
    advancement_api_v1::take_queued_events()
        .into_iter()
        .map(|event| horizon_client::game::pluto_advancement_event_source::QueuedEvent {
            trigger: event.trigger,
            payload: event.payload,
            matches: event.matches,
        })
        .collect()
}

fn resolve_dynamic_screen_slots(
    block: &str,
    data: &[i32],
    stacks: &[Option<ItemStack>],
) -> Option<Vec<Option<[f32; 2]>>> {
    let items = stacks
        .iter()
        .map(|stack| match stack {
            Some(stack) => slot_of(stack).map(Some),
            None => Some(None),
        })
        .collect::<Option<Vec<_>>>()?;
    screen_handler_api_v1::dynamic_slots(block, data, &items)
}

fn refresh_client_resources(host: &dyn Host) {
    let resources = host.client_resources();
    let images = resources.clone();
    resource_loader_v0::set_client_resource_backend(Some(resource_loader_v0::ClientResourceBackend {
        read: std::sync::Arc::new(move |path| resources.read(path)),
        image: std::sync::Arc::new(move |name| images.texture(name).map(|image| resource_loader_v0::ResourceImage {
            width: image.width, height: image.height, rgba: image.rgba,
        })),
    }));
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
/// A pointer to the host that is turned into a reference only for the length of one call, so the player scope and the world scope can both
/// hold it without two live `&mut`.
#[derive(Clone, Copy)]
struct HostHandle(*mut dyn Host);

impl HostHandle {
    fn new(host: &mut dyn Host) -> Self {
        // SAFETY: the handle is only used while the callback that gave the host runs, one call at a time.
        Self(unsafe { std::mem::transmute::<&mut dyn Host, *mut dyn Host>(host) })
    }

    #[allow(clippy::mut_from_ref)]
    fn get(&self) -> &mut dyn Host {
        // SAFETY: see `new`; no reference made here outlives the call that asked for it.
        unsafe { &mut *self.0 }
    }
}

struct ApiLivingDrops<'a> {
    context:&'a mut dyn horizon_client::game::pluto_living_drops::Context,
    info:crate::pure::living_entity_drops::LivingDropInfo,
}
impl crate::pure::living_entity_drops::LivingEntityDrops for ApiLivingDrops<'_> {
    fn info(&self)->&crate::pure::living_entity_drops::LivingDropInfo {&self.info}
    fn can_drop_head(&self)->Result<bool,String> {self.context.can_drop_head()}
    fn claim_head_drop(&mut self)->Result<bool,String> {self.context.claim_head_drop()}
    fn append_stack(&mut self,data:player_api::StackData)->Result<(),String> {
        Identifier::parse(&data.item).map_err(|e|e.to_string())?;
        if data.count==0 || data.count>i32::MAX as u32 {return Err("invalid death drop count".into());}
        let mut stack=ItemStack::new(data.item.as_str(),data.count);
        write_stack(&mut stack,&data);
        self.context.append(stack)
    }
}

/// A stack as PlutoLib's `StackData`: its damage, max damage and the `minecraft:custom_data` the stack carries.
fn stack_data_of(_host: &dyn Host, stack: &ItemStack) -> player_api::StackData {
    read_stack(stack)
}

fn read_stack(stack: &ItemStack) -> player_api::StackData {
    let custom_data = match stack.data().and_then(|d| d.other("minecraft:custom_data")) {
        Some(Tag::Compound(compound)) => compound_from_tag(compound),
        _ => NbtCompound::new(),
    };
    let max_damage = match stack.data().and_then(|d| d.other("minecraft:max_damage")) {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    };
    player_api::StackData { item: stack.item.to_string(), count: stack.count, damage: stack.damage, max_damage, custom_data, custom_model_data: read_custom_model_data(stack), enchantments: read_enchantments(stack).unwrap_or_default() }
}

/// The stack's own `minecraft:custom_model_data` component as PlutoLib's `CustomModelData`, None when it has none.
fn read_custom_model_data(stack: &ItemStack) -> Option<player_api::CustomModelData> {
    match stack.data().and_then(|d| d.other("minecraft:custom_model_data")) {
        Some(Tag::Compound(compound)) => Some(player_api::CustomModelData::from_nbt(&compound_from_tag(compound))),
        _ => None,
    }
}

/// The component value for `data`: None (remove the component) when there is none or it is empty.
fn custom_model_data_tag(data: &Option<player_api::CustomModelData>) -> Option<Tag> {
    data.as_ref().filter(|data| !data.is_empty()).map(|data| Tag::Compound(compound_to_tag(&data.to_nbt())))
}

fn reload_item_prototypes(registries:&std::collections::BTreeMap<Identifier,resource_loader_v0::RegistryLookup>)
    ->Result<Option<crate::pure::item_registry::ItemRegistrySnapshot>,String> {
    let id=Identifier::parse("minecraft:item").unwrap();
    let Some(lookup)=registries.get(&id) else { return Ok(None) };
    let Some(snapshot)=horizon_client::registries::pluto_reload_registry::current() else { return Ok(None) };
    let Some(defaults)=snapshot.iter().find(|r|r.id=="minecraft:item").and_then(|r|r.item_defaults.as_ref()) else { return Ok(None) };
    fn strict(tag:&Tag)->Option<Nbt> {
        match tag {
            Tag::String(value)=>Some(Nbt::String(value.to_string().ok()?)),
            Tag::List(values)=>Some(Nbt::List(values.iter().map(strict).collect::<Option<Vec<_>>>()?)),
            Tag::Compound(values)=>Some(Nbt::Compound(values.iter().map(|(key,value)|
                Some((key.to_string().ok()?,strict(value)?))).collect::<Option<NbtCompound>>()?)),
            _=>nbt_from_tag(tag),
        }
    }
    let stacks=defaults.iter().map(|prototype| {
        let (tag,used)=horizon_world::nbt::read_network(&prototype.custom_data,horizon_world::nbt::Limits::default())
            .map_err(|e|format!("invalid item prototype NBT: {e}"))?;
        if used!=prototype.custom_data.len() { return Err("trailing item prototype NBT".into()); }
        let Some(Nbt::Compound(custom_data))=strict(&tag) else { return Err("unsupported item prototype custom data".into()) };
        Ok(player_api::StackData { item:prototype.id.clone(),count:1,damage:0,max_damage:prototype.max_damage,custom_data,custom_model_data:None,enchantments:Default::default() })
    }).collect::<Result<Vec<_>,String>>()?;
    crate::pure::item_registry::ItemRegistrySnapshot::decode(lookup,stacks)?
        .with_crafting_remainders(defaults.iter().map(|prototype|(prototype.id.clone(),prototype.crafting_remainder.clone())).collect()).map(Some)
}

/// Writes count, damage, max damage, custom data and custom model data of `new` onto `stack` (the item is not changed).
fn write_stack(stack: &mut ItemStack, new: &player_api::StackData) {
    stack.count = new.count;
    stack.damage = new.damage;
    // the stack's own durability: a component the game reads before the item's
    stack.data_mut().set_other("minecraft:max_damage", new.max_damage.filter(|m| *m > 0).map(Tag::Int));
    stack.data_mut().set_other("minecraft:custom_data", (!new.custom_data.is_empty()).then(|| Tag::Compound(compound_to_tag(&new.custom_data))));
    // the stack's own enchantments travel with it too, as `write_slot` keeps a slot's
    if new.enchantments != read_enchantments(stack).unwrap_or_default() {
        stack.data_mut().set_other("minecraft:enchantments", (!new.enchantments.is_empty()).then(|| {
            let compound: NbtCompound = new.enchantments.iter().map(|(id, level)| (id.clone(), Nbt::Int(*level))).collect();
            Tag::Compound(compound_to_tag(&compound))
        }));
    }
    // the stack's own model selection: the item definition reads it, so it must travel with the stack
    stack.data_mut().set_other("minecraft:custom_model_data", custom_model_data_tag(&new.custom_model_data));
    stack.tidy();
}

use horizon_client::entry::pluto_entity_spawn_access as spawn_access;
struct SpawnView<'a> {
    info:crate::pure::entity_spawn_api::MobSpawnInfo,
    random:&'a mut dyn spawn_access::Random,
    equipment:&'a mut dyn spawn_access::Equipment,
}
fn spawn_info(info:&spawn_access::Info)->Result<crate::pure::entity_spawn_api::MobSpawnInfo,String> {
    if !info.position.iter().all(|v|v.is_finite()) { return Err("non-finite mob spawn position".into()); }
    Ok(crate::pure::entity_spawn_api::MobSpawnInfo {
        entity_type:Identifier::parse(&info.entity_type).map_err(|e|e.to_string())?,
        dimension:Identifier::parse(&info.dimension).map_err(|e|e.to_string())?,
        position:info.position,reason:info.reason.clone(),
    })
}
impl crate::pure::entity_spawn_api::MobSpawnPreparation for SpawnView<'_> {
    fn info(&self)->&crate::pure::entity_spawn_api::MobSpawnInfo { &self.info }
    fn next_float(&mut self)->f32 { self.random.next_float() }
    fn next_double(&mut self)->f64 { self.random.next_double() }
    fn next_int(&mut self,bound:i32)->Result<i32,String> {
        if bound<=0 { return Err("mob RNG bound must be positive".into()); }
        self.random.next_int(bound)
    }
    fn equipment(&self,slot:&str)->Result<Option<player_api::StackData>,String> {
        self.equipment.get(slot).map(|stack|stack.as_ref().map(read_stack))
    }
}
impl crate::pure::entity_spawn_api::MobSpawnCompletion for SpawnView<'_> {
    fn set_equipment(&mut self,slot:&str,stack:player_api::StackData)->Result<(),String> {
        let mut native=ItemStack::new(&stack.item,stack.count);
        write_stack(&mut native,&stack);
        self.equipment.set(slot,native)
    }
}
struct SpawnCompletion(crate::pure::entity_spawn_api::PreparedSpawn);
impl spawn_access::Completion for SpawnCompletion {
    fn complete(self:Box<Self>,info:&spawn_access::Info,random:&mut dyn spawn_access::Random,equipment:&mut dyn spawn_access::Equipment)->Result<(),String> {
        let mut view=SpawnView { info:spawn_info(info)?,random,equipment };
        self.0.complete(&mut view)
    }
}
fn native_spawn_prepare(info:&spawn_access::Info,random:&mut dyn spawn_access::Random,equipment:&mut dyn spawn_access::Equipment)
    ->Result<Option<Box<dyn spawn_access::Completion>>,String> {
    let mut view=SpawnView { info:spawn_info(info)?,random,equipment };
    Ok(crate::pure::entity_spawn_api::prepare(&mut view)?.map(|pending|Box::new(SpawnCompletion(pending)) as Box<dyn spawn_access::Completion>))
}

struct NativeTradeWorld<'a>(&'a horizon_client::items::Items);
impl crate::pure::trade_offer_helper::TradeWorld for NativeTradeWorld<'_> {
    fn item_tag(&self, tag: &str) -> Option<Vec<String>> { self.0.item_tag_order.get(tag).cloned() }
    fn item_exists(&self, item: &str) -> bool { self.0.get(item).is_some() }
}
struct NativeTradeRandom<'a>(&'a mut dyn horizon_client::merchant::pluto_trade_access::Random);
impl crate::pure::trade_offer_helper::TradeRandom for NativeTradeRandom<'_> {
    fn next_int(&mut self, bound: i32) -> i32 { self.0.next_int(bound) }
    fn next_float(&mut self) -> f32 { self.0.next_float() }
    fn next_double(&mut self) -> f64 { self.0.next_double() }
}
fn native_trade_stack(items: &horizon_client::items::Items, data: &player_api::StackData) -> Result<ItemStack, String> {
    if items.get(&data.item).is_none() { return Err(format!("trade references unknown item {}", data.item)); }
    let mut made = ItemStack::new(&data.item, data.count);
    write_stack(&mut made, data);
    Ok(made)
}
fn native_trade_factories(set: &str) -> Result<Vec<horizon_client::merchant::pluto_trade_access::Factory>, String> {
    use crate::pure::trade_offer_helper::{TradeOfferContext, TradeOfferHelper};
    use horizon_client::merchant::{ItemCost, MerchantOffer, pluto_trade_access::Factory};
    // Current host's modern pool names. This preserves its native amount and
    // duplicate policy; original 1.20 rare-list distribution is not validated.
    let level = match set {
        "minecraft:wandering_trader/common" => 1,
        "minecraft:wandering_trader/uncommon" => 2,
        _ => return Ok(Vec::new()),
    };
    Ok(TradeOfferHelper::wandering_trader_offers(level)?.into_iter().map(|factory| Factory {
        create: std::sync::Arc::new(move |ctx, random| {
            let world = NativeTradeWorld(ctx.items);
            let context = TradeOfferContext { world: &world, position: ctx.origin, variant: ctx.variant };
            let Some(offer) = factory.create(&context, &mut NativeTradeRandom(random))? else { return Ok(None) };
            let first = native_trade_stack(ctx.items, &offer.first_buy)?;
            let second = offer.second_buy.as_ref().map(|s| native_trade_stack(ctx.items, s)).transpose()?;
            let result = native_trade_stack(ctx.items, &offer.sell)?;
            Ok(Some(MerchantOffer { base_cost_a: ItemCost { stack: first }, cost_b: second.map(|stack| ItemCost { stack }),
                result, uses: 0, max_uses: offer.max_uses, reward_exp: true, special_price_diff: 0,
                demand: 0, price_multiplier: offer.price_multiplier, xp: offer.experience }))
        }),
    }).collect())
}

struct StagedMenu<'a, 'env> {
    menu: &'a mut horizon_client::menu::Menu,
    env: &'a horizon_client::menu::Env<'env>,
}

fn native_slot(slot: Option<ItemSlot>) -> Option<ItemStack> {
    slot.filter(|slot| slot.count > 0).map(|slot| {
        let mut stack = ItemStack::new(&slot.item.to_string(), slot.count);
        write_slot(&mut stack, &slot);
        stack
    })
}

impl screen_handler_api_v1::MenuAccess for StagedMenu<'_, '_> {
    fn slot_count(&self) -> usize { self.menu.slot_count() }
    fn stack(&self, slot: usize) -> Option<ItemSlot> {
        if slot >= self.menu.slot_count() { return None; }
        self.menu.slot(slot).and_then(slot_of)
    }
    fn set_stack(&mut self, slot: usize, stack: Option<ItemSlot>) -> bool {
        if slot >= self.menu.slot_count() { return false; }
        self.menu.set_slot(self.env, slot, native_slot(stack));
        true
    }
    fn swap_stack(&self, button: i32) -> Option<ItemSlot> {
        horizon_client::menu::pluto_menu_actions::swap_stack(self.menu, button).and_then(slot_of)
    }
    fn set_swap_stack(&mut self, button: i32, stack: Option<ItemSlot>) -> bool {
        horizon_client::menu::pluto_menu_actions::set_swap_stack(self.menu, button, native_slot(stack))
    }
    fn carried(&self) -> Option<ItemSlot> { self.menu.carried.as_ref().and_then(slot_of) }
    fn set_carried(&mut self, stack: Option<ItemSlot>) { self.menu.carried = native_slot(stack); }
    fn same_item_and_components(&self, left: &ItemSlot, right: &ItemSlot) -> bool {
        match (native_slot(Some(left.clone())), native_slot(Some(right.clone()))) {
            (Some(left), Some(right)) => left.same_item_same_components(&right),
            _ => false,
        }
    }
    fn max_stack_count(&self, stack: &ItemSlot) -> u32 {
        // A registered per-stack maximum (a filled tank stacks to 16) wins over the item's own.
        if let Some(limit) = item_api_v1::max_stack_size(&item_api_v1::MaxStackSizeContext {
            item: stack.item.to_string(), custom_data: stack.custom_data.clone(),
        }) {
            return limit;
        }
        native_slot(Some(stack.clone())).map_or(0, |stack| {
            horizon_client::menu::pluto_menu_actions::max_stack_count(self.env, &stack)
        })
    }
    fn move_stack(&mut self, stack: &mut ItemSlot, start: usize, end: usize, reverse: bool) -> bool {
        let Some(mut native) = native_slot(Some(stack.clone())) else { return false };
        let moved = horizon_client::menu::pluto_menu_actions::move_stack(self.menu, self.env, &mut native, start, end, reverse);
        if let Some(remainder) = slot_of(&native) { *stack = remainder; }
        moved
    }
    fn crafting_remainder(&self, stack: &ItemSlot) -> Option<ItemSlot> {
        let stack = native_slot(Some(stack.clone()))?;
        horizon_client::menu::pluto_menu_actions::crafting_remainder(self.env, &stack).as_ref().and_then(slot_of)
    }
    fn give_stack(&mut self, stack: ItemSlot) {
        if let Some(mut stack) = native_slot(Some(stack)) {
            horizon_client::menu::pluto_menu_actions::give_stack(self.menu, self.env, &mut stack);
        }
    }
    fn drop_stack(&mut self, stack: &ItemSlot) {
        if let Some(stack) = native_slot(Some(stack.clone())) {
            horizon_client::menu::pluto_menu_actions::drop_stack(self.menu, &stack);
        }
    }
}

fn staged_menu_click(menu: &mut horizon_client::menu::Menu, env: &horizon_client::menu::Env<'_>,
    context: &mut horizon_client::menu::pluto_menu_actions::Context, slot: usize, button: i32,
    input: horizon_client::menu::Input) -> bool {
    use screen_handler_api_v1::{SlotAction, SlotClick};
    use horizon_client::menu::Input;
    // Remote menus use a synthetic ModBlockEntity with an empty entity id.
    // Resolve its declared menu type rather than silently skipping prediction.
    let block = block_entity_api_v1::block_of(&context.entity).or_else(|| {
        let horizon_client::menu::Kind::Mod(kind) = menu.kind() else { return None };
        block_entity_api_v1::block_of_menu(kind.id)
    });
    let Some(block) = block else { return false };
    let action = match input {
        Input::Pickup => SlotAction::Pickup, Input::QuickMove => SlotAction::QuickMove,
        Input::Swap => SlotAction::Swap, Input::Throw => SlotAction::Throw, _ => return false,
    };
    let Some(handler) = screen_handler_api_v1::slot_click(&block, slot, action) else { return false };
    let player = player_api::ServerPlayer { uuid: context.player_uuid, name: context.player_name.clone() };
    let mut data = compound_from_tag(&context.data);
    handler(&mut SlotClick { pos: context.pos, player: &player, authoritative: context.authoritative,
        slot, button, action, data: &mut data, menu: &mut StagedMenu { menu, env } });
    context.data = compound_to_tag(&data);
    true
}

struct HostOps(HostHandle);

impl HostOps {
    fn host(&mut self) -> &mut dyn Host {
        self.0.get()
    }
}

/// A world that only answers block tags, over a host that can only be read.
struct ReadOnlyTags<'a>(&'a dyn Host);

impl WorldOps for ReadOnlyTags<'_> {
    fn block_state(&mut self, _: &str, _: [i32; 3]) -> Option<String> {
        None
    }

    fn set_block_state(&mut self, _: &str, _: [i32; 3], _: &str) -> bool {
        false
    }

    fn play_sound(&mut self, _: &str, _: &str, _: [f64; 3], _: f32, _: f32) -> bool {
        false
    }

    fn block_in_tag(&mut self, block: &str, tag: &str) -> bool {
        self.0.block_name_in_tag(block, tag).unwrap_or(false)
    }
}

/// The world's blocks over the host.
struct HostWorld(HostHandle);

impl WorldOps for HostWorld {
    fn random_float(&mut self, dimension: &str) -> Option<f32> {
        use horizon_client::mod_api::RandomSource;
        self.0.get().level_random(dimension).map(|random| random.next_float())
    }

    fn block_state(&mut self, dimension: &str, pos: [i32; 3]) -> Option<String> {
        let host = self.0.get();
        let level = host.level(dimension)?;
        host.level_block_state(level, pos)
    }

    fn play_sound(&mut self, _dimension: &str, sound: &str, at: [f64; 3], volume: f32, pitch: f32) -> bool {
        // source 4 is the block category
        self.0.get().play_sound(sound, at, 4, volume, pitch)
    }

    fn set_block_state(&mut self, dimension: &str, pos: [i32; 3], state: &str) -> bool {
        let Ok(parsed) = BlockState::parse(state) else { return false };
        let host = self.0.get();
        // The block already there keeps its block entity (the ticking one included): only the named properties change.
        if host.block(pos) == parsed.block.to_string() {
            for (key, value) in &parsed.properties {
                if host.block_property(pos, key).as_deref() != Some(value.as_str()) {
                    host.set_block_property(pos, key, value);
                }
            }
            return parsed.properties.iter().all(|(key, value)| host.block_property(pos, key).as_deref() == Some(value.as_str()));
        }
        let Some(id) = host.default_state(&parsed.block.to_string()) else { return false };
        let changed = host.set_block(dimension, pos, id, 3);
        for (key, value) in &parsed.properties {
            host.set_block_property(pos, key, value);
        }
        changed
    }

    fn schedule_block_tick(&mut self, dimension: &str, pos: [i32; 3], block: &str, delay: i32) -> bool {
        // The mod-facing host has no scheduling call, so the request is queued and the level's
        // own tick seams apply it in that dimension (see world_api_v1::queue_block_tick).
        world_api_v1::queue_block_tick(dimension, pos, block, delay)
    }

    fn block_entity_data(&mut self, dimension: &str, pos: [i32; 3]) -> Option<NbtCompound> {
        let host = self.0.get();
        // `mod_block_entity` reaches the block entities of the level in place; another dimension answers nothing.
        if host.dimension() != dimension {
            return None;
        }
        let entity = host.mod_block_entity(pos)?;
        Some(compound_from_tag(&entity.nbt))
    }

    fn merge_block_entity_data(&mut self, dimension: &str, pos: [i32; 3], data: &NbtCompound) -> bool {
        let host = self.0.get();
        if host.dimension() != dimension {
            return false;
        }
        let mut data = Some(data.clone());
        let mut merged = false;
        host.change_mod_block_entity(pos, &mut |entity| {
            let Some(data) = data.take() else { return };
            merged = with_block_entity(entity, |held, _| {
                held.extend(data.iter().map(|(key, value)| (key.clone(), value.clone())));
                true
            });
        });
        if merged {
            host.block_entity_sync(pos);
        }
        merged
    }

    fn summon_entity(&mut self, dimension: &str, kind: &str, pos: [f64; 3], nbt: &NbtCompound) -> Option<u32> {
        // The host's summon is `/summon` itself (API 59): it loads any kind from its data, projectiles
        // included, and answers the new entity's id.
        self.0.get().summon(dimension, kind, pos, compound_to_tag(nbt))
    }

    fn explode(&mut self, dimension: &str, center: [f64; 3], radius: f32, fire: bool, interaction: world_api_v1::ExplosionInteraction) -> bool {
        // The mod-facing host has no explosion call (the game's own `native_explode`, API 66, lives
        // inside the game for native mods), so the request is queued and the level's mob-event seam
        // sets it off in that dimension (see pluto_explosion_access), as block ticks are queued for
        // the tick seams. A centre or radius the game would refuse is refused here instead of queued.
        if !center.iter().all(|value| value.is_finite() && value.abs() <= 30_000_000.0)
            || !radius.is_finite() || radius <= 0.0 || radius > 32.0 {
            return false;
        }
        use horizon_client::entry::pluto_explosion_access::Blast;
        let blast = match interaction {
            world_api_v1::ExplosionInteraction::Keep => Blast::Keep,
            world_api_v1::ExplosionInteraction::Block => Blast::Block,
            world_api_v1::ExplosionInteraction::Tnt => Blast::Tnt,
        };
        horizon_client::entry::pluto_explosion_access::push(dimension, center, radius, fire, blast)
    }
}

/// PlutoLib's own mod id: the id its mod screen opens under (API 111).
const PLUTO_MOD_ID: &str = "pluto_lib";

/// The book the reader is reading now, kept on the host side because the screen hooks that draw it and
/// turn its pages do not run inside a callback scope. Cleared when the book screen closes.
static BOOK_VIEW: std::sync::Mutex<Option<BookView>> = std::sync::Mutex::new(None);

/// PlutoLib's book reach over Horizon's mod screen: opening asks the game to open PlutoLib's own screen
/// over the world, and the screen hooks below draw the current page into it. Opening is a client act —
/// off the client thread the host answers false and nothing opens.
struct HostBookOps(HostHandle);

impl BookOps for HostBookOps {
    fn open_book(&mut self, book: &Identifier) -> bool {
        if book_api_v1::BookRegistry::get(book).is_none() {
            return false;
        }
        if let Ok(mut view) = BOOK_VIEW.lock() {
            *view = Some(BookView::new(book.clone()));
        }
        let opened = self.0.get().mod_screen_open(PLUTO_MOD_ID);
        if !opened {
            if let Ok(mut view) = BOOK_VIEW.lock() {
                *view = None;
            }
        }
        opened
    }

    fn close_book(&mut self) -> bool {
        let host = self.0.get();
        if host.mod_screen() != Some(PLUTO_MOD_ID) {
            return false;
        }
        host.mod_screen_close(PLUTO_MOD_ID);
        if let Ok(mut view) = BOOK_VIEW.lock() {
            *view = None;
        }
        true
    }

    fn open_book_id(&mut self) -> Option<Identifier> {
        if self.0.get().mod_screen() != Some(PLUTO_MOD_ID) {
            return None;
        }
        BOOK_VIEW.lock().ok()?.as_ref().map(|view| view.book.clone())
    }
}

/// Whether the screen showing is PlutoLib's own mod screen.
fn book_screen_is_ours(host: &dyn Host, screen_id: &str) -> bool {
    screen_id == "mod" && host.mod_screen() == Some(PLUTO_MOD_ID)
}

/// The book being read and where the reader is in it, when a book is open.
fn open_book_view() -> Option<(book_api_v1::Book, BookView)> {
    let view = BOOK_VIEW.lock().ok()?.clone()?;
    let book = book_api_v1::BookRegistry::get(&view.book)?;
    Some((book, view))
}

fn set_book_view(view: BookView) {
    if let Ok(mut held) = BOOK_VIEW.lock() {
        *held = Some(view);
    }
}

/// The host's face for a PlutoLib one (the same six, in the same order).
fn host_face(face: block_api_v1::Direction) -> horizon_client::mod_api::Direction {
    horizon_client::mod_api::Direction::ALL[face as usize]
}

fn fluid_stack_of(fluid: &str, amount: i32) -> Option<FluidStack> {
    let fluid = Identifier::parse(fluid).ok()?;
    u32::try_from(amount).ok().filter(|&amount| amount > 0).map(|amount| FluidStack { fluid, amount, tag: None })
}

/// How much of `offered` the block entity at `pos` takes by its own fluid rule (`BlockEntityType::fluid_filler`); `None` when its block
/// has none. With `execute` the filler's changes are kept (and sent to the clients), otherwise they are thrown away.
fn filler_takes(host: &mut dyn Host, pos: [i32; 3], face: Option<block_api_v1::Direction>, offered: &FluidStack, execute: bool) -> Option<u32> {
    let block = host.block(pos);
    if !block_entity_api_v1::has_fluid_filler(&block) {
        return None;
    }
    let mut taken = None;
    if execute {
        let mut fluid = Some(offered.clone());
        host.change_mod_block_entity(pos, &mut |entity| {
            let Some(fluid) = fluid.take() else { return };
            with_block_entity(entity, |data, items| {
                taken = block_entity_api_v1::BlockEntityTypeRegistry::fill_fluid(&block, pos, data, items, fluid, face, true);
                true
            });
        });
        if taken.is_some() {
            host.block_entity_sync(pos);
        }
    } else {
        let mut entity = host.mod_block_entity(pos)?;
        with_block_entity(&mut entity, |data, items| {
            taken = block_entity_api_v1::BlockEntityTypeRegistry::fill_fluid(&block, pos, data, items, offered.clone(), face, false);
            false
        });
    }
    taken
}

/// The block entities of the level in place as a [`BlockEntityStore`]: what [`FluidStorage`] reads and writes when a
/// neighbour's fluid query is answered through a block's own hooks. The block entity being ticked is out of the
/// world (the host keeps it so), so a read of its own position answers nothing, as the other reaches find.
struct HostBlockEntities {
    host: HostHandle,
    dimension: String,
}

impl BlockEntityStore for HostBlockEntities {
    fn block_at(&mut self, pos: [i32; 3]) -> Option<String> {
        let host = self.host.get();
        let level = host.level(&self.dimension)?;
        let text = host.level_block_state(level, pos)?;
        BlockState::parse(&text).ok().map(|state| state.block.to_string())
    }

    fn read(&mut self, pos: [i32; 3]) -> Option<(NbtCompound, Vec<Option<ItemSlot>>)> {
        let mut entity = self.host.get().mod_block_entity(pos)?;
        let mut found = None;
        with_block_entity(&mut entity, |data, items| {
            found = Some((data.clone(), items.clone()));
            false
        });
        found
    }

    fn write(&mut self, pos: [i32; 3], data: NbtCompound, items: Vec<Option<ItemSlot>>) {
        let mut data = Some(data);
        let mut items = Some(items);
        let mut wrote = false;
        self.host.get().change_mod_block_entity(pos, &mut |entity| {
            let (Some(data), Some(items)) = (data.take(), items.take()) else { return };
            wrote = with_block_entity(entity, |held, held_items| {
                *held = data;
                *held_items = items;
                true
            });
        });
        if wrote {
            self.host.get().block_entity_sync(pos);
        }
    }
}

/// The containers, tanks and block states beside a ticking block entity, over the host.
struct HostNeighbours {
    host: HostHandle,
    dimension: String,
}

impl HostNeighbours {
    /// The block entity store of the level in place, for the fluid queries answered through a block's own hooks.
    fn store(&mut self) -> HostBlockEntities {
        HostBlockEntities { host: self.host, dimension: self.dimension.clone() }
    }

    /// Whether the block at `pos` holds its fluid storage in its saved data, read through its type's hooks: those
    /// queries are answered through [`FluidStorage`] over the store, because the host's own tank calls cannot serve
    /// them (see `block_entity_api_v1::fluid_served_from_data`).
    fn served_from_data(&mut self, pos: [i32; 3]) -> bool {
        block_entity_api_v1::fluid_served_from_data(&self.host.get().block(pos))
    }

    /// Whether the block at `pos` holds its item storage in its saved data, answered through its type's hooks (an item
    /// forward, a sided access or a transfer rule): those queries are answered through [`ItemStorage`] over the store,
    /// because the host's own container calls open every slot every way and cannot serve them (see
    /// `block_entity_api_v1::item_served_from_data`).
    fn item_served_from_data(&mut self, pos: [i32; 3]) -> bool {
        block_entity_api_v1::item_served_from_data(&self.host.get().block(pos))
    }
}

impl NeighbourOps for HostNeighbours {
    fn item_slots(&mut self, pos: [i32; 3], face: block_api_v1::Direction) -> Option<Vec<NeighbourSlot>> {
        if self.item_served_from_data(pos) {
            return ItemStorage::new(&mut self.store()).slots(pos, face);
        }
        let slots = self.host.get().container_slots(&self.dimension, pos, host_face(face))?;
        Some(slots.into_iter().map(|(slot, stack, can_take)| NeighbourSlot { slot, stack: stack.as_ref().and_then(slot_of), can_take }).collect())
    }

    fn extract_item(&mut self, pos: [i32; 3], face: block_api_v1::Direction, slot: usize, count: u32) -> Option<ItemSlot> {
        if self.item_served_from_data(pos) {
            return ItemStorage::new(&mut self.store()).extract(pos, face, slot, count);
        }
        self.host.get().extract_item(&self.dimension, pos, host_face(face), slot, count).and_then(|stack| slot_of(&stack))
    }

    fn insert_item(&mut self, pos: [i32; 3], face: block_api_v1::Direction, stack: ItemSlot) -> Option<ItemSlot> {
        if self.item_served_from_data(pos) {
            return ItemStorage::new(&mut self.store()).insert(pos, face, stack);
        }
        let Some(native) = native_slot(Some(stack.clone())) else { return Some(stack) };
        match self.host.get().insert_item(pos, host_face(face), native) {
            None => None,
            Some(left) if left.count == 0 => None,
            Some(left) => {
                let mut rest = stack;
                rest.count = left.count;
                Some(rest)
            }
        }
    }

    fn fluid_tanks(&mut self, pos: [i32; 3], face: block_api_v1::Direction) -> Option<Vec<NeighbourTank>> {
        if self.served_from_data(pos) {
            return FluidStorage::new(&mut self.store()).tanks(pos);
        }
        let tanks = self.host.get().fluid_tanks(&self.dimension, pos, host_face(face))?;
        Some(tanks.into_iter().map(|tank| NeighbourTank {
            fluid: tank.fluid.as_deref().and_then(|fluid| fluid_stack_of(fluid, tank.amount)),
            capacity: u32::try_from(tank.capacity).unwrap_or(0),
        }).collect())
    }

    fn drain_fluid(&mut self, pos: [i32; 3], face: block_api_v1::Direction, amount: u32) -> Option<FluidStack> {
        if self.served_from_data(pos) {
            return FluidStorage::new(&mut self.store()).drain(pos, None, amount, true);
        }
        let amount = i32::try_from(amount).unwrap_or(i32::MAX);
        let (fluid, drained) = self.host.get().fluid_extract(&self.dimension, pos, host_face(face), amount)?;
        fluid_stack_of(&fluid, drained)
    }

    fn fill_fluid(&mut self, pos: [i32; 3], face: block_api_v1::Direction, stack: &FluidStack, execute: bool) -> u32 {
        if self.served_from_data(pos) {
            return FluidStorage::new(&mut self.store()).fill(pos, stack, execute);
        }
        let host = self.host.get();
        let amount = i32::try_from(stack.amount).unwrap_or(i32::MAX);
        if execute {
            // The host asks `custom_fluid_fill` for a block with its own fluid rule and fills the declared tanks of the others.
            let left = host.fluid_insert(pos, host_face(face), &stack.fluid.to_string(), amount);
            return (amount - left).clamp(0, amount) as u32;
        }
        if let Some(taken) = filler_takes(host, pos, Some(face), stack, false) {
            return taken.min(stack.amount);
        }
        let fluid = stack.fluid.to_string();
        let Some(tanks) = host.fluid_tanks(&self.dimension, pos, host_face(face)) else { return 0 };
        let room: i32 = tanks.iter()
            .filter(|tank| tank.can_fill && tank.fluid.as_deref().is_none_or(|held| held == fluid))
            .map(|tank| (tank.capacity - tank.amount).max(0))
            .sum();
        room.min(amount).max(0) as u32
    }

    fn block_state(&mut self, pos: [i32; 3]) -> Option<String> {
        let host = self.host.get();
        let level = host.level(&self.dimension)?;
        host.level_block_state(level, pos)
    }
}

/// Whether an effect id names fire resistance (the host lists a mob's effects with or without the namespace).
fn is_fire_resistance(effect: &str) -> bool {
    effect == "minecraft:fire_resistance" || effect == "fire_resistance"
}

/// The entities of the level in place, for a ticker: item entities with their stacks, and living entities with the
/// facts a caller's own rules decide with (fire immunity, invulnerability, fire resistance).
struct HostEntities {
    host: HostHandle,
    dimension: String,
}

impl EntityOps for HostEntities {
    fn entities_in(&mut self, min: [f64; 3], max: [f64; 3]) -> Vec<entity_api_v1::WorldEntity> {
        let host = self.host.get();
        let items: std::collections::HashMap<u32, ItemStack> = host.item_entities(&self.dimension, min, max)
            .into_iter().map(|entity: NativeItemEntity| (entity.id, entity.stack)).collect();
        let mut players = host.server_players();
        if let Some(acting) = host.acting_player() {
            players.push(acting);
        }
        let mut found = Vec::new();
        for id in host.entities_in(&self.dimension, min, max) {
            if let Some(stack) = items.get(&id) {
                found.push(entity_api_v1::WorldEntity {
                    id: u64::from(id),
                    kind: Identifier::parse("minecraft:item").expect("valid id"),
                    item: slot_of(stack),
                    living: None,
                });
                continue;
            }
            let Some(entity) = host.entity(id) else { continue };
            let Some(kind) = host.entity_kind(entity) else { continue };
            let Ok(kind_id) = Identifier::parse(&kind) else { continue };
            let living = if kind == "minecraft:player" {
                // A player: their game mode is what makes them invulnerable, and their effects are read by uuid.
                // The player list names the owner of a single-player world by entity id 0; they are the fallback.
                let player = players.iter().find(|player| player.entity_id == id as i32)
                    .or_else(|| players.iter().find(|player| player.entity_id == 0));
                player.map(|player| entity_api_v1::LivingFacts {
                    fire_immune: host.entity_fire_immune(entity).unwrap_or(false),
                    invulnerable: matches!(player.game_mode, 1 | 3),
                    fire_resistant: host.effects(player.uuid)
                        .is_some_and(|effects| effects.iter().any(|effect| is_fire_resistance(&effect.id))),
                })
            } else {
                // A mob or another living kind: its saved data says whether it is alive (a health it still has)
                // and whether it is invulnerable; its effects say whether fire resistance is on it.
                let data = host.entity_nbt(entity);
                let alive = data.as_ref().is_some_and(|data| data.iter()
                    .any(|(key, tag)| key.to_string_lossy() == "Health" && matches!(tag, Tag::Float(bits) if f32::from_bits(*bits) > 0.0)));
                alive.then(|| entity_api_v1::LivingFacts {
                    fire_immune: host.entity_fire_immune(entity).unwrap_or(false),
                    invulnerable: data.as_ref().is_some_and(|data| data.iter()
                        .any(|(key, tag)| key.to_string_lossy() == "Invulnerable" && matches!(tag, Tag::Byte(value) if *value != 0))),
                    fire_resistant: host.entity_effects(entity)
                        .is_some_and(|effects| effects.iter().any(|(effect, _, _)| is_fire_resistance(effect))),
                })
            };
            found.push(entity_api_v1::WorldEntity { id: u64::from(id), kind: kind_id, item: None, living });
        }
        found
    }

    fn discard(&mut self, id: u64) -> bool {
        let Ok(id) = u32::try_from(id) else { return false };
        let host = self.host.get();
        let Some(entity) = host.entity(id) else { return false };
        // The host has no call that removes a dropped item; ageing it out is the game's own end for one (it is
        // discarded at 6000 ticks), reached through the same data merge the freezing action uses.
        let mut merge = Compound::new();
        merge.insert("Age".into(), Tag::Short(6000));
        host.entity_merge_nbt(entity, merge)
    }

    fn set_item(&mut self, id: u64, stack: &ItemSlot) -> bool {
        let Ok(id) = u32::try_from(id) else { return false };
        let host = self.host.get();
        let Some(entity) = host.entity(id) else { return false };
        // Only the id and count are merged in: the components the entity's stack already holds stay, as a data
        // merge keeps the keys it is not given (the remainder is the same stack with fewer items).
        let item = stack.item.to_string();
        let mut held = Compound::new();
        held.insert("id".into(), Tag::String(item.as_str().into()));
        held.insert("count".into(), Tag::Int(stack.count.min(i32::MAX as u32) as i32));
        let mut merge = Compound::new();
        merge.insert("Item".into(), Tag::Compound(held));
        host.entity_merge_nbt(entity, merge)
    }

    fn hurt(&mut self, id: u64, amount: f32, damage_type: &str) -> bool {
        let Ok(id) = u32::try_from(id) else { return false };
        self.host.get().hurt_entity(&self.dimension, id, amount, damage_type, None)
    }

    fn position_of(&mut self, id: u64) -> Option<[f64; 3]> {
        let host = self.host.get();
        if let Some(entity) = u32::try_from(id).ok().and_then(|id| host.entity(id)) {
            if let Some(position) = host.entity_position(entity) { return Some(position); }
        }
        // The player list names players by entity id (0 for the owner of a single-player world).
        let wanted = i32::try_from(id).unwrap_or(-1);
        host.acting_player().filter(|acting| acting.entity_id == wanted || acting.entity_id == 0)
            .or_else(|| host.server_players().into_iter().find(|player| player.entity_id == wanted)).map(|player| player.position)
    }

    fn teleport(&mut self, id: u64, pos: [f64; 3]) -> bool {
        let Ok(id) = u32::try_from(id) else { return false };
        let host = self.host.get();
        // A mob is teleported by the game's own call (its navigation stops, its fall distance resets);
        // another entity is moved keeping the way it faces. A player is reached by neither call — the
        // host has no call that moves a player — so a player's teleport answers false here.
        if host.teleport_mob(&self.dimension, id, pos) {
            return true;
        }
        let rotation = host.entity(id).and_then(|entity| host.entity_rotation(entity)).unwrap_or([0.0, 0.0]);
        host.move_entity(&self.dimension, id, pos, rotation[0], rotation[1])
    }

    // Riding crosses to the game through the passenger transport: it keeps the last rider snapshot of this
    // dimension and queues mount and dismount requests for the game's own rider synchronisation to apply, so an
    // accepted request is observable from the next tick and a refusal there is final.
    fn passengers_of(&mut self, vehicle: u64) -> Vec<u64> {
        let Ok(vehicle) = u32::try_from(vehicle) else { return Vec::new() };
        horizon_client::game::pluto_entity_ride_access::passengers_of(&self.dimension, vehicle).into_iter().map(u64::from).collect()
    }

    fn vehicle_of(&mut self, passenger: u64) -> Option<u64> {
        let passenger = u32::try_from(passenger).ok()?;
        horizon_client::game::pluto_entity_ride_access::vehicle_of(&self.dimension, passenger).map(u64::from)
    }

    fn start_riding(&mut self, passenger: u64, vehicle: u64) -> bool {
        let (Ok(passenger), Ok(vehicle)) = (u32::try_from(passenger), u32::try_from(vehicle)) else { return false };
        horizon_client::game::pluto_entity_ride_access::request_mount(&self.dimension, passenger, vehicle)
    }

    fn stop_riding(&mut self, passenger: u64) -> bool {
        let Ok(passenger) = u32::try_from(passenger) else { return false };
        horizon_client::game::pluto_entity_ride_access::request_dismount(&self.dimension, passenger)
    }
}

/// Runs `f` with the player scope and the world scope both open over the host.
fn scoped<R>(host: &mut dyn Host, f: impl FnOnce() -> R) -> R {
    let handle = HostHandle::new(host);
    player_api::with_ops(&mut HostOps(handle), || world_api_v1::with_world(&mut HostWorld(handle), || book_api_v1::with_books(&mut HostBookOps(handle), f)))
}

impl PlayerOps for HostOps {
    fn net_send_to_player(&mut self, player: u128, channel: &str, data: &[u8]) -> Result<(), String> {
        self.host().net_send_to_player(player, channel, data)
    }
    fn net_send_to_server(&mut self, channel: &str, data: &[u8]) -> Result<(), String> {
        self.host().net_send_to_server(channel, data)
    }
    fn tell(&mut self, player: u128, line: &str) -> bool {
        self.host().tell(player, Text::literal(line.to_owned()))
    }

    fn give(&mut self, player: u128, item: &str, count: u32) -> u32 {
        self.host().give_stack(player, ItemStack::new(item, count)).map_or(0, |left| left.count)
    }

    fn stack(&mut self, player: u128, slot: i32) -> Option<player_api::StackData> {
        let _ = player; // the host reaches the acting player, who is the one the handle names
        self.host().carried_stack(slot).filter(|s| s.count > 0).map(|s| read_stack(s))
    }

    fn give_stack(&mut self, player: u128, stack: player_api::StackData) -> Option<player_api::StackData> {
        let mut made = ItemStack::new(&stack.item, stack.count);
        write_stack(&mut made, &stack);
        self.host().give_stack(player, made).map(|left| player_api::StackData { count: left.count, ..stack })
    }

    fn update_stack(&mut self, player: u128, slot: i32, expected_item: &str, new: &player_api::StackData) -> bool {
        let _ = player;
        match self.host().carried_stack(slot).filter(|s| &*s.item == expected_item && s.count > 0) {
            Some(stack) => {
                write_stack(stack, new);
                true
            }
            None => false,
        }
    }

    fn selected_slot(&mut self, _player: u128) -> Option<i32> {
        self.host().acting_player().map(|acting| i32::from(acting.selected_slot))
    }

    fn count_item(&mut self, player: u128, item: &str) -> u32 {
        let _ = player; // inside a callback the host reaches the acting player, who is the one the PlutoLib handle names
        (0..36).filter_map(|slot| self.host().carried_stack(slot).filter(|s| &*s.item == item).map(|s| s.count)).sum()
    }

    fn take_item(&mut self, player: u128, item: &str, count: u32) -> u32 {
        let _ = player;
        self.host().take_items(item, count)
    }

    fn count_in_tag(&mut self, player: u128, tag: &str) -> u32 {
        let _ = player;
        let mut total = 0;
        for slot in 0..36 {
            let Some(stack) = self.host().carried_stack(slot).cloned() else { continue };
            if self.host().server_item_in_tag(&stack, tag).unwrap_or(false) {
                total += stack.count;
            }
        }
        total
    }

    fn take_in_tag(&mut self, player: u128, tag: &str, count: u32) -> u32 {
        let _ = player;
        let mut taken = 0;
        for slot in 0..36 {
            if taken >= count {
                break;
            }
            let Some(stack) = self.host().carried_stack(slot).cloned() else { continue };
            if self.host().server_item_in_tag(&stack, tag).unwrap_or(false) {
                let item = stack.item.to_string();
                taken += self.host().take_items(&item, (count - taken).min(stack.count));
            }
        }
        taken
    }

    fn empty_slots(&mut self, player: u128) -> u32 {
        let _ = player;
        (0..36).filter(|slot| self.host().carried_stack(*slot).is_none()).count() as u32
    }

    fn language(&mut self, player: u128) -> Option<String> {
        self.host().player_client(player).map(|client| client.language)
    }

    fn add_effect(&mut self, player: u128, effect: &EffectInstance) -> bool {
        self.host().add_effect(
            player,
            &horizon_client::game::NativeEffect {
                id: effect.id.clone(),
                amplifier: effect.amplifier,
                duration: effect.duration,
                ambient: effect.ambient,
                particles: effect.particles,
            },
        )
    }

    fn remove_effect(&mut self, player: u128, id: &str) -> bool {
        self.host().remove_effect(player, id)
    }

    fn effects(&mut self, player: u128) -> Option<Vec<EffectInstance>> {
        let effects = self.host().effects(player)?;
        Some(
            effects
                .into_iter()
                .map(|e| EffectInstance { id: e.id, amplifier: e.amplifier, duration: e.duration, ambient: e.ambient, particles: e.particles })
                .collect(),
        )
    }

    fn food(&mut self, player: u128) -> Option<FoodStatus> {
        self.host().player_food(player).map(|(level, saturation, exhaustion)| FoodStatus { level, saturation, exhaustion })
    }

    fn add_food(&mut self, player: u128, nutrition: i32, saturation: f32) -> bool {
        let Some(before) = self.food(player) else { return false };
        let Some(after) = player_api::fed_food(before, nutrition, saturation) else { return false };
        self.host().set_player_status(player, None, Some((after.level, after.saturation, after.exhaustion)), None)
    }

    fn attachment_get(&mut self, player: u128, id: &str) -> Option<String> {
        self.attachment_get_with_policy(player, id, false)
    }
    fn attachment_set(&mut self, player: u128, id: &str, json: &str) -> bool {
        self.attachment_set_with_policy(player, id, json, false)
    }
    fn attachment_get_with_policy(&mut self, player: u128, id: &str, sync: bool) -> Option<String> {
        let kind = json_attachment(id, sync)?;
        match kind.get_existing(self.host(), Holder::Player(player)) {
            Ok(Some(json)) if !json.is_empty() => Some(json),
            _ => None,
        }
    }
    fn attachment_set_with_policy(&mut self, player: u128, id: &str, json: &str, sync: bool) -> bool {
        match json_attachment(id, sync) {
            Some(kind) => kind.set(self.host(), Holder::Player(player), json.to_owned()).is_ok(),
            None => false,
        }
    }
    fn entity_attachment_get(&mut self, entity: u32, dimension: &str, id: &str, sync: bool) -> Option<String> {
        let kind = json_attachment(id, sync)?;
        let host = self.host();
        if host.dimension() != dimension { return None; }
        match kind.get_existing(host, Holder::Entity(entity)) {
            Ok(Some(json)) if !json.is_empty() => Some(json),
            _ => None,
        }
    }
    fn entity_attachment_set(&mut self, entity: u32, dimension: &str, id: &str, json: &str, sync: bool) -> bool {
        let Some(kind) = json_attachment(id, sync) else { return false };
        let host = self.host();
        host.dimension() == dimension && kind.set(host, Holder::Entity(entity), json.to_owned()).is_ok()
    }
}

/// Horizon wants attachment keys as `'static` strs and a codec made of plain functions, while PlutoLib attachments are created at run time.
/// So each id gets one leaked `AttachmentType<String>` (made once, kept for the process) whose value is the JSON text in a string tag.
fn json_attachment(id: &str, sync: bool) -> Option<&'static AttachmentType<String>> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static KINDS: OnceLock<Mutex<HashMap<String, &'static AttachmentType<String>>>> = OnceLock::new();
    let (namespace, name) = id.split_once(':')?;
    let mut kinds = KINDS.get_or_init(Default::default).lock().ok()?;
    if let Some(kind) = kinds.get(id) {
        return (kind.sync == sync).then_some(*kind);
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
        sync,
    }));
    kinds.insert(id.to_owned(), kind);
    Some(kind)
}

fn cancel_if(handled: bool) -> Flow {
    if handled { Flow::Cancel } else { Flow::Continue }
}

/// Puts an effect on the player `uuid` through the host, which reaches the world's owner by their uuid as it reaches a guest. An effect
/// with a negative amplifier or no duration is not given.
fn give_effect(host: &mut dyn Host, uuid: u128, effect: String, amplifier: i32, duration: i32) {
    if duration <= 0 || amplifier < 0 { return; }
    host.add_effect(uuid, &horizon_client::game::NativeEffect { id: effect, amplifier: u8::try_from(amplifier).unwrap_or(u8::MAX), duration, ambient: false, particles: true });
}

/// Does `actions` to the player `uuid`, the `Attacker` of each: the effects by uuid, the rest as entity actions when the host can name the
/// player's entity (a guest, or the acting player); those are skipped otherwise.
fn apply_player_actions(host: &mut dyn Host, uuid: u128, actions: Vec<entity_events_v1::EntityAction>) {
    let mut rest = Vec::new();
    for action in actions {
        match action {
            entity_events_v1::EntityAction::AddEffect { target: entity_events_v1::ActionTarget::Attacker, effect, amplifier, duration } => give_effect(host, uuid, effect, amplifier, duration),
            other => rest.push(other),
        }
    }
    if rest.is_empty() { return; }
    let found = host.acting_player().filter(|acting| acting.uuid == uuid).map(|acting| (acting.entity_id, acting.dimension.to_string()))
        .or_else(|| host.server_players().into_iter().find(|player| player.uuid == uuid).map(|player| (player.entity_id, player.dimension.to_string())));
    let Some((entity, dimension)) = found else { return };
    apply_entity_actions(host, &dimension, entity, -1, rest);
}

/// What the world around an entity says about it: the biome it stands in and the temperature there, whether rain falls on it and whether
/// its eyes are in water. `block` is the block it stands in and `eyes` where its eyes are.
fn read_surroundings(host: &mut dyn Host, state: &mut entity_events_v1::EntityState, block: [i32; 3], eyes: Option<[f64; 3]>) {
    let biome = host.biome(block);
    let (rain, thunder) = host.weather();
    let raining = rain > 0.2;
    let (block_light, sky_light) = host.light(block);
    state.block_light = Some(block_light);
    state.sky_light = Some(sky_light);
    state.sky_darken = entity_events_v1::sky_darken(rain, thunder, host.day_time());
    state.y = Some(f64::from(block[1]));
    state.rain_exposed = entity_events_v1::rain_exposed(raining, sky_light, biome.as_deref(), block[1]);
    state.biome_temperature = biome.as_deref().and_then(entity_events_v1::biome_climate).map(|(base, _)| entity_events_v1::temperature_at(base, block[1]));
    state.biome = biome;
    if let Some(eyes) = eyes {
        let eye_block = [eyes[0].floor() as i32, eyes[1].floor() as i32, eyes[2].floor() as i32];
        let waterlogged = host.block_property(eye_block, "waterlogged").as_deref() == Some("true");
        state.eyes_in_water = entity_events_v1::holds_water(&host.block(eye_block), waterlogged);
    }
}

/// The state of a server player: what the player list says, their effects with levels, their health and maximum, the armor they wear and
/// the world around them. A player's eyes are taken 1.62 blocks above their feet.
fn player_entity_state(host: &mut dyn Host, player: &horizon_client::mod_api::NativeServerPlayer) -> entity_events_v1::EntityState {
    let effects = host.effects(player.uuid).unwrap_or_default();
    use horizon_client::mod_api::EquipmentSlot;
    let worn = [host.player_equipment(player.uuid, EquipmentSlot::Head).is_some(), host.player_equipment(player.uuid, EquipmentSlot::Chest).is_some(),
        host.player_equipment(player.uuid, EquipmentSlot::Legs).is_some(), host.player_equipment(player.uuid, EquipmentSlot::Feet).is_some()]
        .into_iter().filter(|worn| *worn).count();
    let mut state = entity_events_v1::EntityState {
        kind: "minecraft:player".to_owned(),
        sprinting: player.sprinting,
        on_ground: player.on_ground,
        effects: effects.iter().map(|effect| effect.id.clone()).collect(),
        effect_levels: effects.iter().map(|effect| (effect.id.clone(), i32::from(effect.amplifier))).collect(),
        health: Some(player.health),
        max_health: host.attribute(player.uuid, "minecraft:max_health").map(|(_, value)| value as f32),
        armor_coverage: Some(entity_events_v1::armor_coverage(worn)),
        crouching: player.sneaking,
        ..entity_events_v1::EntityState::default()
    };
    // The equipment slots that hold something, the hands and the armor alike.
    let mut equipment = Vec::new();
    for (name, slot) in [("mainhand", EquipmentSlot::MainHand), ("offhand", EquipmentSlot::OffHand), ("head", EquipmentSlot::Head), ("chest", EquipmentSlot::Chest), ("legs", EquipmentSlot::Legs), ("feet", EquipmentSlot::Feet)] {
        if host.player_equipment(player.uuid, slot).is_some() { equipment.push(name.to_owned()); }
    }
    state.equipment = Some(equipment);
    let [x, y, z] = player.position;
    let block = [x.floor() as i32, y.floor() as i32, z.floor() as i32];
    read_surroundings(host, &mut state, block, Some([x, y + 1.62, z]));
    // The height of the feet, not of the block they stand in.
    state.y = Some(y);
    // The player list has no flag for water; the block they stand in does.
    state.in_water = entity_events_v1::holds_water(&host.block(block), host.block_property(block, "waterlogged").as_deref() == Some("true"));
    state
}

/// What the break hook read before the block went, kept for the end of the same break.
struct PendingBreak { pos: [i32; 3], dimension: String, state: Option<BlockState>, player: player_api::ServerPlayer, face: Option<block_api_v1::Direction>, yaw: f32, pitch: f32 }
static PENDING_BREAKS: std::sync::Mutex<Vec<PendingBreak>> = std::sync::Mutex::new(Vec::new());

/// The face each player last started digging, by uuid: the break hook itself is not told the face, but the dig's start is
/// (`left_click_block`), and a break belongs to the dig the player last started.
static LAST_DIG_FACE: std::sync::Mutex<std::collections::BTreeMap<u128, block_api_v1::Direction>> = std::sync::Mutex::new(std::collections::BTreeMap::new());

/// The last redstone strength reported to each block entity that hears signal changes, by dimension and position: a
/// change is what is reported, and the first sight of a block entity counts as one.
static LAST_SIGNAL: std::sync::Mutex<std::collections::BTreeMap<(String, [i32; 3]), u8>> = std::sync::Mutex::new(std::collections::BTreeMap::new());

/// The acting player, their dimension and main-hand stack, and the block at `pos`, as the block break events report them.
fn break_context(host: &mut dyn Host, pos: [i32; 3]) -> events_interaction_v0::BlockBreakContext {
    let acting = host.acting_player();
    let dimension = acting.as_ref().map(|player| player.dimension.to_string()).unwrap_or_default();
    let state = match host.level(&dimension) { Some(level) => host.level_block_state(level, pos), None => None }.and_then(|text| BlockState::parse(&text).ok());
    let held = acting.as_ref().and_then(|player| host.held_stack(player.uuid)).map(|stack| read_stack(&stack));
    let face = acting.as_ref().and_then(|player| LAST_DIG_FACE.lock().ok().and_then(|last| last.get(&player.uuid).copied()));
    let (yaw, pitch) = acting.as_ref().map(|player| (player.yaw, player.pitch)).unwrap_or_default();
    events_interaction_v0::BlockBreakContext {
        pos, dimension, state,
        player: acting.map(|player| player_api::ServerPlayer { uuid: player.uuid, name: player.name.clone() }),
        held, actions: std::cell::RefCell::new(Vec::new()),
        face, yaw, pitch, extra_breaks: std::cell::RefCell::new(Vec::new()),
    }
}

/// Reports the earlier breaks of the player `uuid` to `AFTER`: the game has no hook at the end of a break, so each allowed break waits until the
/// player's next tick (or their next break) and is heard then if the block at its position is no longer the one that was there. `held` is the
/// stack the player holds now, and the actions the callbacks queue are done to the player.
fn flush_block_breaks(host: &mut dyn Host, uuid: u128) {
    let due: Vec<PendingBreak> = match PENDING_BREAKS.lock() {
        Ok(mut pending) => { let (due, rest) = std::mem::take(&mut *pending).into_iter().partition(|entry| entry.player.uuid == uuid); *pending = rest; due }
        Err(_) => return,
    };
    for entry in due {
        let now = match host.level(&entry.dimension) { Some(level) => host.level_block_state(level, entry.pos), None => None }.and_then(|text| BlockState::parse(&text).ok());
        // An unreadable block or one that is still the same means the break did not happen (another mod cancelled it later).
        if now.is_none() || now == entry.state { continue; }
        let held = host.held_stack(uuid).map(|stack| read_stack(&stack));
        let context = events_interaction_v0::BlockBreakContext {
            pos: entry.pos, dimension: entry.dimension, state: entry.state, player: Some(entry.player), held, actions: std::cell::RefCell::new(Vec::new()),
            face: entry.face, yaw: entry.yaw, pitch: entry.pitch, extra_breaks: std::cell::RefCell::new(Vec::new()),
        };
        scoped(host, || events_interaction_v0::after_block_break(&context));
        let actions = context.actions.take();
        if !actions.is_empty() { apply_player_actions(host, uuid, actions); }
    }
}

fn nbt_from_tag(tag: &Tag) -> Option<Nbt> {
    Some(match tag {
        Tag::End => return None,
        Tag::Byte(v) => Nbt::Byte(*v),
        Tag::Short(v) => Nbt::Short(*v),
        Tag::Int(v) => Nbt::Int(*v),
        Tag::Long(v) => Nbt::Long(*v),
        Tag::Float(bits) => Nbt::Float(f32::from_bits(*bits)),
        Tag::Double(bits) => Nbt::Double(f64::from_bits(*bits)),
        Tag::ByteArray(v) => Nbt::ByteArray(v.clone()),
        Tag::String(s) => Nbt::String(s.to_string_lossy()),
        Tag::List(items) => Nbt::List(items.iter().filter_map(nbt_from_tag).collect()),
        Tag::Compound(c) => Nbt::Compound(compound_from_tag(c)),
        Tag::IntArray(v) => Nbt::IntArray(v.clone()),
        Tag::LongArray(v) => Nbt::LongArray(v.clone()),
    })
}

fn tag_from_nbt(nbt: &Nbt) -> Tag {
    match nbt {
        Nbt::Byte(v) => Tag::Byte(*v),
        Nbt::Short(v) => Tag::Short(*v),
        Nbt::Int(v) => Tag::Int(*v),
        Nbt::Long(v) => Tag::Long(*v),
        Nbt::Float(v) => Tag::Float(v.to_bits()),
        Nbt::Double(v) => Tag::Double(v.to_bits()),
        Nbt::ByteArray(v) => Tag::ByteArray(v.clone()),
        Nbt::String(s) => Tag::String(s.as_str().into()),
        Nbt::List(items) => Tag::List(items.iter().map(tag_from_nbt).collect()),
        Nbt::Compound(c) => Tag::Compound(compound_to_tag(c)),
        Nbt::IntArray(v) => Tag::IntArray(v.clone()),
        Nbt::LongArray(v) => Tag::LongArray(v.clone()),
    }
}

/// Edits the custom data of the stack the acting player carries in `slot` (`Host::carried_stack` by the inventory's slot numbers: the selected
/// hotbar slot, 100 to 103 for armor, -106 for the off hand), when it is still of `item`; the whole stack goes back through the same
/// `slot_of` / `write_slot` pair the item-wear adapter uses, so every other component stays as the game had it.
fn edit_player_stack(host: &mut dyn Host, selected: u8, slot: entity_events_v1::StackSlot, item: &str, edits: &[entity_events_v1::StackDataEdit]) {
    let number = entity_events_v1::carried_slot_number(slot, selected);
    let Some(stack) = host.carried_stack(number) else { return };
    if &*stack.item != item || stack.count == 0 { return; }
    let Some(mut held) = slot_of(&*stack) else { return };
    if !entity_events_v1::edit_custom_data(&mut held.custom_data, edits) { return; }
    write_slot(stack, &held);
}

/// Every stack the player carries, as [`CarriedStack`](entity_events_v1::CarriedStack)s: their equipment first (hands, then feet to head),
/// then the main inventory. The inventory is only readable for the acting player (the host's carried reach names no player), so another
/// player's answer holds their equipment alone. Empty places are left out.
fn carried_stacks(host: &mut dyn Host, player: u128) -> Vec<entity_events_v1::CarriedStack> {
    let mut carried = Vec::new();
    for (slot, native) in [
        (entity_events_v1::StackSlot::MainHand, EquipmentSlot::MainHand),
        (entity_events_v1::StackSlot::OffHand, EquipmentSlot::OffHand),
        (entity_events_v1::StackSlot::Feet, EquipmentSlot::Feet),
        (entity_events_v1::StackSlot::Legs, EquipmentSlot::Legs),
        (entity_events_v1::StackSlot::Chest, EquipmentSlot::Chest),
        (entity_events_v1::StackSlot::Head, EquipmentSlot::Head),
    ] {
        if let Some(stack) = host.player_equipment(player, native).filter(|stack| stack.count > 0).and_then(|stack| slot_of(&stack)) {
            carried.push(entity_events_v1::CarriedStack { slot: entity_events_v1::CarriedSlot::Equipment(slot), stack });
        }
    }
    if host.acting_player().is_some_and(|acting| acting.uuid == player) {
        for index in 0..36 {
            if let Some(stack) = host.carried_stack(index).filter(|stack| stack.count > 0).and_then(|stack| slot_of(&stack)) {
                carried.push(entity_events_v1::CarriedStack { slot: entity_events_v1::CarriedSlot::Inventory(u32::try_from(index).unwrap_or(0)), stack });
            }
        }
    }
    carried
}

/// Writes the stacks a callback put back through a carried-stack channel (see
/// [`CarriedEdit`](entity_events_v1::CarriedEdit)): an inventory place through the host's carried reach (the acting player's only), an
/// equipment place through the living entity's own equipment write, as the use handlers do. A place the host cannot reach is skipped.
fn apply_carried_edits(host: &mut dyn Host, player: u128, edits: Vec<entity_events_v1::CarriedEdit>) {
    for edit in edits {
        match edit.slot {
            entity_events_v1::CarriedSlot::Inventory(index) => {
                if host.acting_player().is_some_and(|acting| acting.uuid == player) {
                    if let Some(target) = i32::try_from(index).ok().and_then(|index| host.carried_stack(index)) {
                        write_slot(target, &edit.stack);
                    }
                }
            }
            entity_events_v1::CarriedSlot::Equipment(slot) => {
                let native = match slot {
                    entity_events_v1::StackSlot::MainHand => EquipmentSlot::MainHand,
                    entity_events_v1::StackSlot::OffHand => EquipmentSlot::OffHand,
                    entity_events_v1::StackSlot::Feet => EquipmentSlot::Feet,
                    entity_events_v1::StackSlot::Legs => EquipmentSlot::Legs,
                    entity_events_v1::StackSlot::Chest => EquipmentSlot::Chest,
                    entity_events_v1::StackSlot::Head => EquipmentSlot::Head,
                };
                let living = host.entity_by_uuid(player).and_then(|(_, id, _)| host.entity(id)).and_then(|entity| host.living(entity));
                if let Some(living) = living {
                    living.set_equipment(host, native, native_slot(Some(edit.stack)));
                }
            }
        }
    }
}

/// Does the actions callbacks queued on a hit to its parties: `attacker` and `victim` are the entity ids `ActionTarget::Attacker` and
/// `Victim` name. Every action is guarded and one the host cannot do (an entity it cannot reach, a player where only mobs are served) is skipped.
fn apply_entity_actions(host: &mut dyn Host, dimension: &str, attacker: i32, victim: i32, actions: Vec<entity_events_v1::EntityAction>) {
    use entity_events_v1::{ActionTarget, EntityAction, KnockbackDirection};
    let id_of = |target: ActionTarget| if target == ActionTarget::Attacker { attacker } else { victim };
    // Where an entity or a player is: the player list names players by entity id (0 for the owner of a single-player world).
    fn position_of(host: &dyn Host, id: i32) -> Option<[f64; 3]> {
        if let Some(entity) = u32::try_from(id).ok().and_then(|id| host.entity(id)) {
            if let Some(position) = host.entity_position(entity) { return Some(position); }
        }
        host.acting_player().filter(|acting| acting.entity_id == id || acting.entity_id == 0)
            .or_else(|| host.server_players().into_iter().find(|player| player.entity_id == id)).map(|player| player.position)
    }
    for action in actions {
        match action {
            EntityAction::SetFire { target, seconds } => {
                let Some(entity) = u32::try_from(id_of(target)).ok().and_then(|id| host.entity(id)) else { continue };
                let immune = host.entity_fire_immune(entity).unwrap_or(false);
                let current = host.entity_fire_ticks(entity).unwrap_or(0);
                if let Some(ticks) = entity_events_v1::fire_ticks_after(current, seconds, immune) { host.entity_set_fire(entity, ticks); }
            }
            EntityAction::Heal { target, amount } => {
                let id = id_of(target);
                // A player is healed through their status; a mob through its health in its data (the host cuts both to the maximum).
                let player = host.acting_player().filter(|acting| acting.entity_id == id)
                    .or_else(|| host.server_players().into_iter().find(|player| player.entity_id == id));
                if let Some(player) = player {
                    if let Some(health) = entity_events_v1::healed_health(player.health, amount) { host.set_player_status(player.uuid, Some(health), None, None); }
                    continue;
                }
                let Ok(mob) = u32::try_from(id) else { continue };
                let Some(entity) = host.entity(mob) else { continue };
                if host.entity_kind(entity).as_deref() == Some("minecraft:player") { continue; }
                let Some(data) = host.entity_nbt(entity) else { continue };
                let current = data.iter().find(|(key, _)| key.to_string_lossy() == "Health").and_then(|(_, tag)| match tag { Tag::Float(bits) => Some(f32::from_bits(*bits)), _ => None });
                if let Some(health) = current.and_then(|current| entity_events_v1::healed_health(current, amount)) { host.set_mob_health(dimension, mob, health); }
            }
            EntityAction::Freeze { target, ticks } => {
                let Some(entity) = u32::try_from(id_of(target)).ok().and_then(|id| host.entity(id)) else { continue };
                let Some(kind) = host.entity_kind(entity) else { continue };
                if kind == "minecraft:player" || !entity_events_v1::can_freeze(&kind) { continue; }
                let Some(data) = host.entity_nbt(entity) else { continue };
                let current = data.iter().find(|(key, _)| key.to_string_lossy() == "TicksFrozen").and_then(|(_, tag)| match tag { Tag::Int(value) => Some(*value), _ => None }).unwrap_or(0);
                let Some(frozen) = entity_events_v1::frozen_ticks_after(current, ticks) else { continue };
                let mut merge = Compound::new();
                merge.insert("TicksFrozen".into(), Tag::Int(frozen));
                host.entity_merge_nbt(entity, merge);
                // Freezing puts the fire out.
                host.entity_set_fire(entity, 0);
            }
            EntityAction::Knockback { target, strength, direction } => {
                let Some(entity) = u32::try_from(id_of(target)).ok().and_then(|id| host.entity(id)) else { continue };
                if host.entity_kind(entity).as_deref() == Some("minecraft:player") { continue; }
                let (x, z) = match direction {
                    KnockbackDirection::AgainstFacing => match host.entity_rotation(entity) { Some([yaw, _]) => entity_events_v1::facing_direction(yaw), None => continue },
                    KnockbackDirection::AwayFromOther => {
                        let other = if target == ActionTarget::Attacker { victim } else { attacker };
                        let (Some(from), Some(here)) = (position_of(host, other), host.entity_position(entity)) else { continue };
                        (from[0] - here[0], from[2] - here[2])
                    }
                };
                let on_ground = host.entity_on_ground(entity).unwrap_or(false);
                if let Some(delta) = entity_events_v1::knockback_delta(strength, x, z, on_ground) { host.entity_push(entity, delta); }
            }
            EntityAction::EditStackData { target, slot, item, edits } => {
                // Only the player acting in this hook has a stack the host hands out; a hit on a player lands on that player's own tick.
                let id = id_of(target);
                let Some(selected) = host.acting_player().filter(|acting| acting.entity_id == id).map(|acting| acting.selected_slot) else { continue };
                edit_player_stack(host, selected, slot, &item, &edits);
            }
            EntityAction::Hurt { target, amount, damage_type } => {
                // A player or a mob, hurt with no attacker; the host refuses an entity it cannot hurt (a creative player, a dead mob).
                let Ok(entity) = u32::try_from(id_of(target)) else { continue };
                host.hurt_entity(dimension, entity, amount, &damage_type, None);
            }
            EntityAction::Teleport { target, position } => {
                // A mob is teleported by the game's own call (its navigation stops, its fall distance
                // resets); another entity is moved keeping the way it faces. A player is reached by
                // neither — the host has no call that moves a player — so a player's teleport is skipped.
                let Ok(id) = u32::try_from(id_of(target)) else { continue };
                if host.teleport_mob(dimension, id, position) { continue; }
                let rotation = host.entity(id).and_then(|entity| host.entity_rotation(entity)).unwrap_or([0.0, 0.0]);
                host.move_entity(dimension, id, position, rotation[0], rotation[1]);
            }
            EntityAction::SetEffectDuration { target, effect, duration } => {
                if duration <= 0 { continue; }
                let id = id_of(target);
                let player = host.acting_player().filter(|acting| acting.entity_id == id)
                    .or_else(|| host.server_players().into_iter().find(|player| player.entity_id == id));
                let Some(player) = player else { continue };
                // The effect keeps its level and flags: it is taken off and given back with the new time (adding a shorter one over it would keep the longer).
                let Some(kept) = host.effects(player.uuid).and_then(|all| all.into_iter().find(|active| active.id == effect)) else { continue };
                host.remove_effect(player.uuid, &effect);
                host.add_effect(player.uuid, &horizon_client::game::NativeEffect { id: kept.id, amplifier: kept.amplifier, duration, ambient: kept.ambient, particles: kept.particles });
            }
            EntityAction::AddEffect { target, effect, amplifier, duration } => {
                let id = id_of(target);
                // A player gets the effect through the host; a mob through the same hand-off the weapon-hit effects use.
                let player = host.acting_player().filter(|acting| acting.entity_id == id)
                    .or_else(|| host.server_players().into_iter().find(|player| player.entity_id == id));
                if let Some(player) = player { give_effect(host, player.uuid, effect, amplifier, duration); continue; }
                if duration <= 0 || amplifier < 0 { continue; }
                if let Ok(mob) = u32::try_from(id) { horizon_client::entry::pluto_mob_effect_access::push(mob, &effect, duration, amplifier); }
            }
        }
    }
}

fn compound_from_tag(compound: &Compound) -> NbtCompound {
    compound.iter().filter_map(|(key, tag)| Some((key.to_string_lossy(), nbt_from_tag(tag)?))).collect()
}

fn compound_to_tag(compound: &NbtCompound) -> Compound {
    compound.iter().map(|(key, nbt)| (key.as_str().into(), tag_from_nbt(nbt))).collect()
}

/// A custom data compound from its JSON form, for transports that carry data as JSON (the item
/// behaviour transport). JSON has no byte or null: booleans read as 0/1 bytes and null as an
/// empty string; the queries this feeds inspect keys, structure and strings, which JSON keeps.
fn nbt_from_json(value: &serde_json::Value) -> NbtCompound {
    match value {
        serde_json::Value::Object(map) => map.iter().map(|(key, value)| (key.clone(), nbt_value_from_json(value))).collect(),
        _ => NbtCompound::new(),
    }
}

fn nbt_value_from_json(value: &serde_json::Value) -> Nbt {
    match value {
        serde_json::Value::Null => Nbt::String(String::new()),
        serde_json::Value::Bool(flag) => Nbt::Byte(i8::from(*flag)),
        serde_json::Value::Number(number) => {
            if let Some(int) = number.as_i64() {
                if let Ok(int) = i32::try_from(int) { Nbt::Int(int) } else { Nbt::Long(int) }
            } else {
                Nbt::Double(number.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(text) => Nbt::String(text.clone()),
        serde_json::Value::Array(values) => Nbt::List(values.iter().map(nbt_value_from_json).collect()),
        serde_json::Value::Object(_) => Nbt::Compound(nbt_from_json(value)),
    }
}

/// A stack as an item slot: item, count, damage, max damage and custom data; None for an item id PlutoLib cannot name.
fn slot_of(stack: &ItemStack) -> Option<ItemSlot> {
    let data = read_stack(stack);
    let mut slot = ItemSlot::new(crate::pure::base::Identifier::parse(&stack.item).ok()?, data.count);
    slot.damage = data.damage;
    slot.max_damage = data.max_damage;
    slot.custom_data = data.custom_data;
    slot.tool = read_tool(stack);
    slot.attribute_modifiers = read_attribute_modifiers(stack);
    slot.enchantments = read_enchantments(stack);
    slot.custom_model_data = data.custom_model_data;
    slot.capture(stack.clone());
    Some(slot)
}

/// Restore the source stack, not the destination's components, then apply public edits.
fn write_slot(stack: &mut ItemStack, slot: &ItemSlot) {
    let item = slot.item.to_string();
    *stack = slot.captured::<ItemStack>().filter(|old| old.item.as_ref() == item)
        .cloned().unwrap_or_else(|| ItemStack::new(&item, slot.count));
    let before = read_stack(stack);
    stack.count = slot.count;
    stack.damage = slot.damage;
    if slot.max_damage != before.max_damage {
        stack.data_mut().set_other("minecraft:max_damage", slot.max_damage.filter(|m| *m > 0).map(Tag::Int));
    }
    if slot.custom_data != before.custom_data {
        stack.data_mut().set_other("minecraft:custom_data", (!slot.custom_data.is_empty()).then(|| Tag::Compound(compound_to_tag(&slot.custom_data))));
    }
    // The stack's own tool component; the native encoder/decoder and mining rules read it from here.
    if slot.tool != read_tool(stack) {
        stack.data_mut().set_other("minecraft:tool", slot.tool.as_ref().map(|tool| Tag::Compound(compound_to_tag(tool))));
    }
    // The stack's own attribute modifiers: the host applies them to a player holding or wearing the stack.
    if slot.attribute_modifiers != read_attribute_modifiers(stack) {
        stack.data_mut().set_other("minecraft:attribute_modifiers", slot.attribute_modifiers.as_ref()
            .map(|list| Tag::List(list.iter().map(|modifier| Tag::Compound(compound_to_tag(modifier))).collect())));
    }
    // The stack's own enchantments: the host applies them (fortune, silk touch, respiration...) to the holder or wearer.
    if slot.enchantments != read_enchantments(stack) {
        stack.data_mut().set_other(enchantments_component(stack), slot.enchantments.as_ref().map(|levels| {
            let compound: NbtCompound = levels.iter().map(|(id, level)| (id.clone(), Nbt::Int(*level))).collect();
            Tag::Compound(compound_to_tag(&compound))
        }));
    }
    // The stack's own model selection: the item definition's `select`/`range_dispatch`/`condition` read it from here.
    if slot.custom_model_data != read_custom_model_data(stack) {
        stack.data_mut().set_other("minecraft:custom_model_data", custom_model_data_tag(&slot.custom_model_data));
    }
    stack.tidy();
}

/// The component a stack's enchantments live in: vanilla keeps an enchanted book's in
/// `minecraft:stored_enchantments` (EnchantedBookItem reads and writes that one), everything
/// else's in `minecraft:enchantments`. A slot's enchantments are the stack's, whichever it is.
fn enchantments_component(stack: &ItemStack) -> &'static str {
    if stack.item.as_ref() == "minecraft:enchanted_book" { "minecraft:stored_enchantments" } else { "minecraft:enchantments" }
}

/// The stack's own enchantments component as enchantment id to level, None when it has none of its own.
fn read_enchantments(stack: &ItemStack) -> Option<std::collections::BTreeMap<String, i32>> {
    match stack.data().and_then(|d| d.other(enchantments_component(stack))) {
        Some(Tag::Compound(levels)) => Some(compound_from_tag(levels).into_iter().filter_map(|(id, level)| match level { Nbt::Int(level) => Some((id, level)), _ => None }).collect()),
        _ => None,
    }
}

/// The stack's own `minecraft:attribute_modifiers` component as public NBT, None when it has none of its own.
fn read_attribute_modifiers(stack: &ItemStack) -> Option<Vec<NbtCompound>> {
    match stack.data().and_then(|d| d.other("minecraft:attribute_modifiers")) {
        Some(Tag::List(items)) => Some(items.iter().filter_map(|item| match item { Tag::Compound(compound) => Some(compound_from_tag(compound)), _ => None }).collect()),
        _ => None,
    }
}

/// The stack's own `minecraft:tool` component as public NBT, None when it has none of its own.
fn read_tool(stack: &ItemStack) -> Option<NbtCompound> {
    match stack.data().and_then(|d| d.other("minecraft:tool")) {
        Some(Tag::Compound(compound)) => Some(compound_from_tag(compound)),
        _ => None,
    }
}

/// Runs `f` on public data/slots and writes back changes, retaining each moved slot's source components.
/// True if `f` did something (its own answer).
fn with_block_entity(entity: &mut ModBlockEntity, f: impl FnOnce(&mut NbtCompound, &mut Vec<Option<ItemSlot>>) -> bool) -> bool {
    let mut data = compound_from_tag(&entity.nbt);
    let before = data.clone();
    let mut items: Vec<Option<ItemSlot>> = entity.items
        .iter()
        .map(|slot| slot.as_ref().and_then(slot_of))
        .collect();
    let before_items = items.clone();
    let ran = f(&mut data, &mut items);
    if ran {
        if let Some(Nbt::IntArray(values)) = data.get("pluto:menu_data") {
            entity.data = values.iter().take(256).map(|v| (*v).clamp(i16::MIN as i32, i16::MAX as i32)).collect();
        }
        if data != before {
            entity.nbt = compound_to_tag(&data);
        }
        if items != before_items {
            entity.items = items
                .iter()
                .map(|now| now.as_ref().filter(|slot| slot.count > 0).map(|slot| {
                    let mut made = ItemStack::new(&slot.item.to_string(), slot.count);
                    write_slot(&mut made, slot);
                    made
                }))
                .collect();
        }
    }
    ran
}

/// Commit the container and the complete remainder only when routing handled the click.
fn with_quick_move(
    entity: &mut ModBlockEntity,
    stack: &mut ItemStack,
    route: impl FnOnce(&mut ItemSlot, &mut NbtCompound, &mut Vec<Option<ItemSlot>>) -> bool,
) -> bool {
    let Some(mut moved) = slot_of(stack) else { return false };
    let handled = with_block_entity(entity, |data, items| route(&mut moved, data, items));
    if handled {
        write_slot(stack, &moved);
    }
    handled
}

fn layer_to_native(layer: &block_entity_renderer_api_v1::LayerDefinition) -> horizon_client::mod_api::LayerDefinition {
    use horizon_client::mod_api::{CubeDeformation, CubeListBuilder, MeshDefinition, PartId};
    fn append(mesh: &mut MeshDefinition, parent: PartId, parts: &[(String, block_entity_renderer_api_v1::PartDefinition)]) {
        for (name, part) in parts {
            let mut cubes = CubeListBuilder::create();
            for cube in &part.cubes {
                let [x, y, z] = cube.deformation;
                cubes = cubes.tex_offs(cube.texture_offset[0], cube.texture_offset[1])
                    .set_mirror(cube.mirror)
                    .add_box_with(cube.origin, cube.dimensions, CubeDeformation::xyz(x, y, z));
            }
            let child = mesh.add_or_replace_child(parent, name, cubes, pose_to_native(part.pose));
            append(mesh, child, &part.children);
        }
    }
    let mut mesh = MeshDefinition::new();
    let root = mesh.root();
    append(&mut mesh, root, &layer.parts);
    horizon_client::mod_api::LayerDefinition::create(mesh, layer.texture_size[0], layer.texture_size[1])
}

fn pose_from_native(pose: horizon_client::mod_api::PartPose) -> PartPose {
    PartPose { translation: [pose.x, pose.y, pose.z], rotation: [pose.x_rot, pose.y_rot, pose.z_rot],
        scale: [pose.x_scale, pose.y_scale, pose.z_scale] }
}
fn pose_to_native(pose: PartPose) -> horizon_client::mod_api::PartPose {
    horizon_client::mod_api::PartPose {
        x: pose.translation[0], y: pose.translation[1], z: pose.translation[2],
        x_rot: pose.rotation[0], y_rot: pose.rotation[1], z_rot: pose.rotation[2],
        x_scale: pose.scale[0], y_scale: pose.scale[1], z_scale: pose.scale[2],
    }
}
fn model_from_native(part: &horizon_client::mod_api::ModelPart) -> ModelPart {
    let polygons = part.polygons.iter().map(|polygon| ModelPolygon {
        vertices: polygon.vertices.map(|v| ModelVertex { pos: v.pos, uv: [v.u, v.v] }),
        normal: polygon.normal,
    }).collect();
    let children = part.children().map(|(name, child)| (name.to_owned(), model_from_native(child))).collect();
    let mut model = ModelPart::new(polygons, children, pose_from_native(part.initial_pose()));
    model.pose = pose_from_native(part.store_pose());
    model.visible = part.visible;
    model.skip_draw = part.skip_draw;
    model
}
/// A worn stack as the item renderer's merged components describe it: the same fields the item colour provider hands a mod.
fn worn_stack(item:&str,count:i32,components:&std::collections::BTreeMap<String,serde_json::Value>)->Option<player_api::StackData> {
    let count=u32::try_from(count).ok()?;
    let custom_data=match components.get("minecraft:custom_data") {
        None=>NbtCompound::new(),
        Some(value)=>match horizon_client::scoreboard::json_to_tag(value) {
            Tag::Compound(data)=>compound_from_tag(&data),
            _=>return None,
        },
    };
    let damage=match components.get("minecraft:damage") {
        None=>0,
        Some(value)=>u32::try_from(value.as_u64()?).ok()?,
    };
    let max_damage=match components.get("minecraft:max_damage") {
        None=>None,
        Some(value)=>Some(i32::try_from(value.as_i64()?).ok()?),
    };
    let custom_model_data=match components.get("minecraft:custom_model_data") {
        None=>None,
        Some(value)=>match horizon_client::scoreboard::json_to_tag(value) {
            Tag::Compound(data)=>Some(player_api::CustomModelData::from_nbt(&compound_from_tag(&data))),
            _=>return None,
        },
    };
    let enchantments=match components.get("minecraft:enchantments") {
        None=>Default::default(),
        Some(value)=>match horizon_client::scoreboard::json_to_tag(value) {
            Tag::Compound(data)=>compound_from_tag(&data).into_iter().filter_map(|(id, level)| match level { Nbt::Int(level) => Some((id, level)), _ => None }).collect(),
            _=>Default::default(),
        },
    };
    Some(player_api::StackData {item:item.into(),count,damage,max_damage,custom_data,custom_model_data,enchantments})
}
fn skull_model_to_native(part:&horizon_client::mod_api::ModelPart)->horizon_client::entry::pluto_block::SkullModelPart {
    let p=part.store_pose();
    horizon_client::entry::pluto_block::SkullModelPart {
        pose:[p.x,p.y,p.z,p.x_rot,p.y_rot,p.z_rot,p.x_scale,p.y_scale,p.z_scale],
        polygons:if part.visible && !part.skip_draw {part.polygons.iter().map(|poly|
            poly.vertices.map(|v|[v.pos[0],v.pos[1],v.pos[2],v.u,v.v])).collect()} else {Vec::new()},
        children:if part.visible {part.children().map(|(name,child)|(name.to_owned(),skull_model_to_native(child))).collect()} else {Vec::new()},
    }
}

fn model_to_native(part: &ModelPart) -> horizon_client::mod_api::ModelPart {
    let polygons = part.polygons.iter().map(|polygon| horizon_client::mod_api::ModelPolygon {
        vertices: polygon.vertices.map(|v| horizon_client::mod_api::ModelVertex { pos: v.pos, u: v.uv[0], v: v.uv[1] }),
        normal: polygon.normal,
    }).collect();
    let children = part.children.iter().map(|(name, child)| (name.clone(), model_to_native(child))).collect();
    let mut model = horizon_client::mod_api::ModelPart::new(polygons, children, pose_to_native(part.initial_pose));
    model.load_pose(pose_to_native(part.pose));
    model.visible = part.visible;
    model.skip_draw = part.skip_draw;
    model
}

/// What a block entity renderer draws on: the game's draw state for this block entity.
struct HostDraw<'a> {
    host: &'a dyn Host,
    draw: &'a mut BlockEntityDraw,
    state: horizon_world::registry::BlockStateId,
    client_data: Option<&'a NbtCompound>,
}

impl BlockEntityCanvas for HostDraw<'_> {
    fn client_data(&self) -> Option<&NbtCompound> { self.client_data }
    fn state_property(&self, name: &str) -> Option<String> {
        self.host.block_state_property(self.state, name).map(str::to_owned)
    }
    fn partial_tick(&self) -> Option<f32> { Some(self.draw.partial) }
    fn game_time(&self) -> Option<i64> { Some(self.draw.game_time) }
    fn push(&mut self) {
        self.draw.push();
    }

    fn pop(&mut self) {
        self.draw.pop();
    }

    fn translate(&mut self, by: [f32; 3]) {
        self.draw.translate(by);
    }

    fn rotate_about(&mut self, axis: [f32; 3], degrees: f32, pivot: [f32; 3]) {
        self.draw.rotate_about(axis, degrees, pivot);
    }

    fn scale(&mut self, by: [f32; 3]) {
        self.draw.scale(by);
    }

    fn quad(&mut self, sprite: &str, corners: [[f32; 3]; 4], uv: [f32; 4], tint: u32) -> bool {
        self.host.draw_quad(self.draw, sprite, corners, uv, tint)
    }

    fn cube(&mut self, from: [f32; 3], to: [f32; 3], sprite: &str, tint: u32) -> bool {
        let face = CubeFace { sprite, uv: None };
        self.host.draw_cube(self.draw, from, to, [Some(face); 6], tint)
    }

    fn block(&mut self, block: &str, properties: &[(&str, &str)]) -> bool {
        self.host.draw_block(self.draw, block, properties)
    }

    fn item(&mut self, item: &ItemSlot) -> bool {
        self.item_with_context(item, ItemDisplayContext::Fixed)
    }

    fn item_with_context(&mut self, item: &ItemSlot, context: ItemDisplayContext) -> bool {
        let native = match context {
            ItemDisplayContext::None => DisplayContext::None,
            ItemDisplayContext::ThirdPersonLeftHand => DisplayContext::ThirdPersonLeftHand,
            ItemDisplayContext::ThirdPersonRightHand => DisplayContext::ThirdPersonRightHand,
            ItemDisplayContext::FirstPersonLeftHand => DisplayContext::FirstPersonLeftHand,
            ItemDisplayContext::FirstPersonRightHand => DisplayContext::FirstPersonRightHand,
            ItemDisplayContext::Head => DisplayContext::Head,
            ItemDisplayContext::Gui => DisplayContext::Gui,
            ItemDisplayContext::Ground => DisplayContext::Ground,
            ItemDisplayContext::Fixed => DisplayContext::Fixed,
            ItemDisplayContext::OnShelf => DisplayContext::OnShelf,
        };
        let mut stack = ItemStack::new(&item.item.to_string(), item.count);
        write_slot(&mut stack, item);
        self.host.draw_item(self.draw, &stack, native, None, 0)
    }

    fn model(&mut self, model: &str, tint: u32) -> bool {
        self.host.draw_model(self.draw, model, tint)
    }

    fn bake_layer(&mut self, layer: &str) -> Option<ModelPart> {
        self.host.bake_layer(layer).map(|part| model_from_native(&part))
    }

    fn model_part(&mut self, part: &ModelPart, sprite: &str, tint: u32) -> bool {
        self.host.draw_model_part(self.draw, &model_to_native(part), sprite, tint)
    }

    fn keep(&mut self) {
        self.draw.keep();
    }
}

/// What a screen draws on: the game's batch for the frame.
struct HostCanvas<'a> {
    host: &'a dyn Host,
    batch: &'a mut horizon_client::hud::HudBatch,
}

impl ScreenCanvas for HostCanvas<'_> {
    fn fill(&mut self, rect: [f32; 4], argb: u32) {
        self.host.fill(self.batch, rect, argb);
    }

    fn sprite(&mut self, name: &str, src: [f32; 4], dst: [f32; 4]) -> bool {
        self.host.sprite(self.batch, name, src, dst)
    }

    fn text(&mut self, text: &str, at: [f32; 2], rgb: u32, shadow: bool) {
        self.host.text(self.batch, text, at, rgb, shadow);
    }

    fn text_width(&self, text: &str) -> f32 {
        self.host.text_width(text)
    }
}

fn model_config(file: &NativeConfigFile) -> ModConfig {
    let kind = match file.kind {
        NativeConfigKind::Client => ConfigKind::Client,
        NativeConfigKind::Common => ConfigKind::Common,
        NativeConfigKind::Server => ConfigKind::Server,
    };
    ModConfig { mod_id: file.mod_id.clone(), kind, file: file.file.clone(), text: file.text.clone() }
}

/// The game's view of the blocks around, as a behaviour sees it.
struct GameView<'a>(&'a dyn horizon_client::entry::pluto_block::LevelView);

fn to_state(info: &horizon_client::entry::pluto_block::StateInfo) -> Option<BlockState> {
    Some(BlockState { block: crate::pure::base::Identifier::parse(&info.block).ok()?, properties: info.properties.clone() })
}

fn to_info(state: &BlockState) -> horizon_client::entry::pluto_block::StateInfo {
    horizon_client::entry::pluto_block::StateInfo { block: state.block.to_string(), properties: state.properties.clone() }
}

impl block_api_v1::BlockView for GameView<'_> {
    fn block_state(&self, pos: [i32; 3]) -> Option<BlockState> {
        self.0.state(pos).as_ref().and_then(to_state)
    }

    fn is_air(&self, pos: [i32; 3]) -> bool {
        self.0.is_air(pos)
    }

    fn is_replaceable(&self, pos: [i32; 3]) -> bool { self.0.replaceable(pos) }
    fn generation_depth(&self) -> Option<i32> { self.0.generation_depth() }
    fn has_neighbor_signal(&self, pos: [i32; 3]) -> Option<bool> { self.0.has_neighbor_signal(pos) }

    fn is_face_sturdy(&self, pos: [i32; 3], face: block_api_v1::Direction) -> bool {
        self.0.face_sturdy(pos, face as u8)
    }

    fn is_in_tag(&self, pos: [i32; 3], tag: &str) -> bool {
        self.0.in_tag(pos, tag)
    }
    fn is_state_in_tag(&self, state: &BlockState, tag: &str) -> Option<bool> {
        self.0.state_in_tag(&to_info(state), tag)
    }
    fn brightness(&self, pos: [i32; 3]) -> u8 {
        self.0.brightness(pos)
    }

    fn light_passes_up(&self, pos: [i32; 3]) -> bool {
        self.0.light_passes_up(pos)
    }

    fn is_water(&self, pos: [i32; 3]) -> bool {
        self.0.is_water(pos)
    }

    fn is_fluid_full(&self, pos: [i32; 3]) -> bool {
        self.0.fluid_full(pos)
    }

    fn is_area_loaded(&self, pos: [i32; 3], radius: i32) -> bool {
        self.0.area_loaded(pos, radius)
    }

    fn is_full_block(&self, pos: [i32; 3]) -> bool {
        self.0.full_block(pos)
    }

    fn build_height(&self) -> [i32; 2] {
        self.0.build_height()
    }
}

/// The game's view of the blocks around, with the right to change them.
struct GameEditor<'a, T: horizon_client::entry::pluto_block::LevelEdit + ?Sized = dyn horizon_client::entry::pluto_block::LevelEdit + 'a>(&'a mut T);

impl<T: horizon_client::entry::pluto_block::LevelEdit + ?Sized> block_api_v1::BlockView for GameEditor<'_, T> {
    fn block_state(&self, pos: [i32; 3]) -> Option<BlockState> {
        self.0.state(pos).as_ref().and_then(to_state)
    }

    fn is_air(&self, pos: [i32; 3]) -> bool {
        self.0.is_air(pos)
    }

    fn is_replaceable(&self, pos: [i32; 3]) -> bool { self.0.replaceable(pos) }
    fn generation_depth(&self) -> Option<i32> { self.0.generation_depth() }
    fn has_neighbor_signal(&self, pos: [i32; 3]) -> Option<bool> { self.0.has_neighbor_signal(pos) }

    fn is_face_sturdy(&self, pos: [i32; 3], face: block_api_v1::Direction) -> bool {
        self.0.face_sturdy(pos, face as u8)
    }

    fn is_in_tag(&self, pos: [i32; 3], tag: &str) -> bool {
        self.0.in_tag(pos, tag)
    }
    fn is_state_in_tag(&self, state: &BlockState, tag: &str) -> Option<bool> {
        self.0.state_in_tag(&to_info(state), tag)
    }
    fn brightness(&self, pos: [i32; 3]) -> u8 {
        self.0.brightness(pos)
    }

    fn light_passes_up(&self, pos: [i32; 3]) -> bool {
        self.0.light_passes_up(pos)
    }

    fn is_water(&self, pos: [i32; 3]) -> bool {
        self.0.is_water(pos)
    }

    fn is_fluid_full(&self, pos: [i32; 3]) -> bool {
        self.0.fluid_full(pos)
    }

    fn is_area_loaded(&self, pos: [i32; 3], radius: i32) -> bool {
        self.0.area_loaded(pos, radius)
    }

    fn is_full_block(&self, pos: [i32; 3]) -> bool {
        self.0.full_block(pos)
    }

    fn build_height(&self) -> [i32; 2] {
        self.0.build_height()
    }
}

impl<T: horizon_client::entry::pluto_block::LevelEdit + ?Sized> block_api_v1::BlockEditor for GameEditor<'_, T> {
    fn set_block_state(&mut self, pos: [i32; 3], state: &BlockState) -> bool {
        self.set_block_state_with_flags(pos, state, 3)
    }

    fn set_block_state_with_flags(&mut self, pos: [i32; 3], state: &BlockState, flags: i32) -> bool {
        self.0.set_state(pos, &to_info(state), flags)
    }

    fn break_block(&mut self, pos: [i32; 3], drop: bool) -> Option<bool> {
        self.0.break_block(pos, drop)
    }

    fn random_int(&mut self, bound: i32) -> i32 {
        self.0.random_int(bound)
    }

    fn random_float(&mut self) -> f32 {
        self.0.random_float()
    }

    fn schedule_tick(&mut self, pos: [i32; 3], block: &str, delay: i32) -> bool {
        self.0.schedule_tick(pos, block, delay)
    }

    fn place_feature(&mut self, feature: &str, pos: [i32; 3]) -> bool {
        self.0.place_feature(feature, pos)
    }

    fn place_tree_with_plan(&mut self, configuration: &serde_json::Value, pos: [i32; 3], roots: Option<&crate::pure::worldgen_api_v1::RootPlacement>, plan: &[crate::pure::worldgen_api_v1::TreeDecoration]) -> bool {
        use crate::pure::worldgen_api_v1::{TreeDecoration, RootPlacementContext, TreeDecoratorContext};
        use horizon_client::entry::pluto_block::{RootBindings, RootCallback, TreeStep};
        let bindings = roots.map(|roots| RootBindings { block_sets: roots.bindings().block_sets.clone(), state_providers: roots.bindings().state_providers.clone() });
        let steps: Vec<_> = plan.iter().enumerate().map(|(index, step)| match step {
            TreeDecoration::Native(native) => TreeStep::Native(*native),
            TreeDecoration::Custom(_) => TreeStep::Custom(index),
        }).collect();
        let mut root_callback = |world: &mut dyn horizon_client::entry::pluto_block::RootEdit, pos| {
            let roots = roots.ok_or_else(|| "root callback unavailable".to_owned())?;
            roots.place(&mut RootPlacementContext { world: &mut GameEditor(world), pos })
                .map(|state| state.as_ref().map(to_info))
        };
        let bound_roots = bindings.as_ref().map(|bindings| (bindings, &mut root_callback as &mut RootCallback<'_>));
        self.0.place_tree_with_plan(configuration, pos, bound_roots, &steps, &mut |index, world, logs, leaves, roots| {
            let Some(TreeDecoration::Custom(decorator)) = plan.get(index) else { return Err("custom tree step unavailable".into()) };
            decorator.decorate(&mut TreeDecoratorContext { world: &mut GameEditor(world), logs, leaves, roots })
        })
    }

    fn place_tree_with_roots(&mut self, configuration: &serde_json::Value, pos: [i32; 3], roots: &crate::pure::worldgen_api_v1::RootPlacement, decorators: &[crate::pure::worldgen_api_v1::TreeDecorator]) -> bool {
        use horizon_client::entry::pluto_block::RootBindings;
        let bindings = RootBindings { block_sets: roots.bindings().block_sets.clone(), state_providers: roots.bindings().state_providers.clone() };
        self.0.place_tree_with_roots(configuration, pos, &bindings, &mut |world, pos| {
            roots.place(&mut crate::pure::worldgen_api_v1::RootPlacementContext { world: &mut GameEditor(world), pos })
                .map(|state| state.as_ref().map(to_info))
        }, &mut |world, logs, leaves, root_positions| {
            let mut editor = GameEditor(world);
            for decorator in decorators {
                decorator.decorate(&mut crate::pure::worldgen_api_v1::TreeDecoratorContext { world: &mut editor, logs, leaves, roots: root_positions })?;
            }
            Ok(())
        })
    }

    fn place_tree(&mut self, configuration: &serde_json::Value, pos: [i32; 3], decorators: &[crate::pure::worldgen_api_v1::TreeDecorator]) -> bool {
        self.0.place_tree(configuration, pos, &mut |world, logs, leaves, roots| {
            let mut editor = GameEditor(world);
            for decorator in decorators {
                decorator.decorate(&mut crate::pure::worldgen_api_v1::TreeDecoratorContext { world: &mut editor, logs, leaves, roots })?;
            }
            Ok(())
        })
    }
}

impl<T: horizon_client::entry::pluto_block::RootEdit + ?Sized> crate::pure::worldgen_api_v1::RootPlacementWorld for GameEditor<'_, T> {
    fn matches_block_set(&mut self, index: usize, pos: [i32; 3]) -> Result<bool, String> { self.0.matches_block_set(index, pos) }
    fn sample_state_provider(&mut self, index: usize, pos: [i32; 3]) -> Result<BlockState, String> {
        let state = self.0.sample_state_provider(index, pos)?;
        to_state(&state).ok_or_else(|| "invalid sampled block identifier".to_owned())
    }
}

fn direction(face: u8) -> block_api_v1::Direction {
    block_api_v1::Direction::ALL[usize::from(face).min(5)]
}

fn block_sink() -> horizon_client::entry::pluto_block::Sink {
    horizon_world::level::pluto_block_access::set_path_node_dispatcher(|view, state, pos, entity| {
        let state = to_state(state)?;
        let context = block_api_v1::PathNodeTypeContext {
            state, pos, view: &GameView(view),
            entity: entity.map(|(id, _)| id), kind: entity.map(|(_, kind)| kind.to_owned()),
        };
        block_api_v1::path_node_type(&context).map(|kind| kind.name().to_owned())
    });
    use horizon_client::entry::pluto_block::{LevelEdit, LevelView, Placement, PlacementAnswer, ShapeUpdate, Sink, StateInfo};
    fn survives(view: &dyn LevelView, state: &StateInfo, pos: [i32; 3]) -> Option<bool> {
        let state = to_state(state)?;
        let block = state.block.to_string();
        block_api_v1::survives(&block, &block_api_v1::SurviveContext { state, pos, view: &GameView(view) })
    }
    fn placement(view: &dyn LevelView, request: &Placement) -> PlacementAnswer {
        let Some(state) = to_state(&request.state) else { return PlacementAnswer::Default };
        let block = state.block.to_string();
        let context = block_api_v1::PlacementContext {
            state,
            pos: request.pos,
            face: direction(request.face),
            yaw: request.yaw,
            pitch: request.pitch,
            sneaking: request.sneaking,
            in_water: request.in_water,
            item: request.item.clone(),
            replace_clicked: request.replace_clicked,
            view: &GameView(view),
        };
        match block_api_v1::placement(&block, &context) {
            block_api_v1::Placement::Default => PlacementAnswer::Default,
            block_api_v1::Placement::Refuse => PlacementAnswer::Refuse,
            block_api_v1::Placement::State(chosen) => PlacementAnswer::State(to_info(&chosen)),
        }
    }
    fn update_shape(view: &dyn LevelView, update: &ShapeUpdate) -> Option<StateInfo> {
        let state = to_state(&update.state)?;
        let block = state.block.to_string();
        let view = GameView(view);
        let context = block_api_v1::ShapeUpdateContext::new(state, update.pos,
            direction(update.direction), update.neighbour_pos,
            update.neighbour.as_ref().and_then(to_state), &view);
        let changed = block_api_v1::shape_update(&block, &context);
        update.tick_delay.set(context.scheduled_tick_delay());
        changed.map(|changed| to_info(&changed))
    }
    fn random_tick(world: &mut dyn LevelEdit, state: &StateInfo, pos: [i32; 3]) -> bool {
        let Some(state) = to_state(state) else { return false };
        let block = state.block.to_string();
        block_api_v1::random_tick(&block, &mut block_api_v1::TickContext { state, pos, world: &mut GameEditor(world) })
    }
    fn scheduled_tick(world: &mut dyn LevelEdit, state: &StateInfo, pos: [i32; 3]) -> bool {
        let Some(state) = to_state(state) else { return false };
        let block = state.block.to_string();
        block_api_v1::scheduled_tick(&block, &mut block_api_v1::TickContext { state, pos, world: &mut GameEditor(world) })
    }
    fn bonemeal_valid(view: &dyn LevelView, state: &StateInfo, pos: [i32; 3]) -> Option<bool> {
        let state = to_state(state)?;
        let block = state.block.to_string();
        block_api_v1::can_bonemeal(&block, &block_api_v1::BonemealContext { state, pos, view: &GameView(view) })
    }
    fn bonemeal_success(world: &mut dyn LevelEdit, state: &StateInfo, pos: [i32; 3]) -> Option<bool> {
        let state = to_state(state)?;
        let block = state.block.to_string();
        block_api_v1::bonemeal_success(&block, &mut block_api_v1::TickContext { state, pos, world: &mut GameEditor(world) })
    }
    fn bonemeal_perform(world: &mut dyn LevelEdit, state: &StateInfo, pos: [i32; 3]) {
        let Some(state) = to_state(state) else { return };
        let block = state.block.to_string();
        block_api_v1::on_bonemeal(&block, &mut block_api_v1::TickContext { state, pos, world: &mut GameEditor(world) });
    }
    fn may_place_on(view: &dyn LevelView, plant: &StateInfo, ground: &StateInfo, ground_pos: [i32; 3]) -> Option<bool> {
        let (plant, ground) = (to_state(plant)?, to_state(ground)?);
        let block = plant.block.to_string();
        block_api_v1::may_place_on(&block, &block_api_v1::PlaceOnContext { state: plant, ground, ground_pos, view: &GameView(view) })
    }
    Sink {
        has: |block| block_api_v1::BlockBehaviors::get(block).is_some(),
        inside_motion: |state| {
            let state = to_state(state)?;
            let callback = block_api_v1::inside_motion_handler(&state.block.to_string())?;
            Some(std::sync::Arc::new(move |request: &mut horizon_client::entry::pluto_block::InsideMotion| {
                let mut context = block_api_v1::EntityInsideMotionContext {
                    state: state.clone(), pos: request.pos, entity: request.entity, kind: request.kind.clone(),
                    server: request.server, suppressing_bounce: request.suppressing_bounce,
                    position: request.position, previous_position: request.previous_position, velocity: request.velocity,
                    sync_velocity: request.sync_velocity, step_sound: request.step_sound,
                };
                block_api_v1::apply_inside_motion(&callback, &mut context);
                request.velocity = context.velocity;
                request.sync_velocity = context.sync_velocity;
                request.step_sound = context.step_sound;
            }))
        },
        after_fall: |state| {
            let state = to_state(state)?;
            let callback = block_api_v1::after_fall_handler(&state.block.to_string())?;
            Some(std::sync::Arc::new(move |request: &mut horizon_client::entry::pluto_block::AfterFall| {
                let mut context = block_api_v1::AfterFallContext {
                    state: state.clone(), pos: request.pos, entity: request.entity, kind: request.kind.clone(),
                    living: request.living, item: request.item, suppressing_bounce: request.suppressing_bounce,
                    velocity: request.velocity, fall_distance: request.fall_distance, on_ground: request.on_ground,
                };
                block_api_v1::apply_after_fall(&callback, &mut context);
                request.velocity = context.velocity;
                request.fall_distance = context.fall_distance;
                request.on_ground = context.on_ground;
            }))
        },
        fall_on: |request| {
            let Some(state) = to_state(&request.state) else { return };
            let block = state.block.to_string();
            let mut context = block_api_v1::FallOnContext {
                state, pos: request.pos, entity: request.entity, kind: request.kind.clone(),
                distance: request.distance, damage: request.damage,
            };
            block_api_v1::fall_on(&block, &mut context);
            request.damage = context.damage;
        },
        entity_inside: |notice| {
            let Some(state) = to_state(&notice.state) else { return };
            let block = state.block.to_string();
            let actions = block_api_v1::inside(&block, &block_api_v1::EntityInsideContext {
                dimension: notice.dimension.clone(), state, pos: notice.pos, entity: notice.entity, kind: notice.kind.clone(),
                bounds: notice.bounds, fall_distance: notice.fall_distance,
                actions: std::cell::RefCell::new(Vec::new()),
            });
            // The seams run inside entity movement, where the entity may not be changed: what the callback
            // did to it travels in plain data to where the game may change it (pluto_contact_access,
            // drained where the mob effects land, after hits and on player ticks).
            if let Ok(entity) = u32::try_from(notice.entity) {
                use horizon_client::entry::pluto_contact_access::Contact;
                for action in actions {
                    match action {
                        entity_events_v1::EntityAction::SetFire { target: entity_events_v1::ActionTarget::Victim, seconds } => {
                            horizon_client::entry::pluto_contact_access::push(entity, Contact::Fire(seconds));
                        }
                        entity_events_v1::EntityAction::AddEffect { target: entity_events_v1::ActionTarget::Victim, effect, amplifier, duration } => {
                            horizon_client::entry::pluto_contact_access::push(entity, Contact::Effect(effect, duration, amplifier));
                        }
                        entity_events_v1::EntityAction::Hurt { target: entity_events_v1::ActionTarget::Victim, amount, damage_type } => {
                            horizon_client::entry::pluto_contact_access::push(entity, Contact::Hurt(damage_type, amount));
                        }
                        _ => {}
                    }
                }
            }
        },
        survives,
        placement,
        update_shape,
        random_tick,
        scheduled_tick,
        bonemeal_valid,
        bonemeal_success,
        bonemeal_perform,
        may_place_on,
    }
}

impl Hooks for PlutoHooks {
    fn client_player_network_logging_in(&self, host: &mut dyn Host, _event: &mut horizon_client::mod_api::ClientPlayerNetworkLoggingInEvent) {
        scoped(host, networking_api_v1::client_joined);
    }
    fn client_disconnecting(&self, _host: &dyn Host, _event: &mut horizon_client::mod_api::ClientDisconnectingEvent) {
        networking_api_v1::client_disconnecting();
    }
    fn net_from_client(&self, host: &mut dyn Host, event: &mut horizon_client::mod_api::NetFromClientEvent<'_>) {
        scoped(host, || networking_api_v1::from_client(event.player, event.channel, event.data));
    }
    fn net_from_server(&self, host: &mut dyn Host, event: &mut horizon_client::mod_api::NetFromServerEvent<'_>) {
        scoped(host, || networking_api_v1::from_server(event.channel, event.data));
    }
    fn layer_definitions(&self, event: &mut horizon_client::mod_api::LayerDefinitionsEvent<'_>) {
        match block_entity_renderer_api_v1::model_layer_definitions() {
            Ok(definitions) => event.layers.extend(definitions.into_iter().map(|(id, layer)| (id, layer_to_native(&layer)))),
            Err(error) => eprintln!("pluto_lib: model layer collection rejected: {error}"),
        }
    }

    fn use_duration(&self, _host: &dyn Host, event: &mut UseDurationEvent<'_>) {
        // An item with a per-stack use answers from the stack's own data (read only for those items: this is asked every frame a use is drawn).
        event.duration = if item_api_v1::has_stack_use(&event.stack.item) {
            item_api_v1::stack_use_duration(&read_stack(event.stack)).unwrap_or(event.duration)
        } else {
            item_api_v1::use_duration(&event.stack.item, event.duration)
        };
    }

    /// A per-stack use refuses a stack it cannot use, and a hungry-only one refuses a full player who is not in creative mode.
    fn living_entity_use_item_start(&self, host: &mut dyn Host, event: &mut horizon_client::mod_api::LivingEntityUseItemStartEvent<'_>) -> Flow {
        if event.entity.is_some() || !item_api_v1::has_stack_use(&event.stack.item) { return Flow::Continue; }
        let stack = read_stack(event.stack);
        let acting = host.acting_player();
        let (hungry, creative) = match &acting {
            Some(player) => (host.player_food(player.uuid).is_none_or(|(level, _, _)| level < 20), player.game_mode == 1),
            None => (true, false),
        };
        if item_api_v1::stack_use_may_start(&stack, hungry, creative) { Flow::Continue } else { Flow::Cancel }
    }

    fn use_animation(&self, _host: &dyn Host, event: &mut UseAnimationEvent<'_>) {
        use item_api_v1::UseAction;
        let action = if item_api_v1::has_stack_use(&event.stack.item) {
            item_api_v1::stack_use_action(&read_stack(event.stack))
        } else {
            item_api_v1::use_animation(&event.stack.item)
        };
        if let Some(action) = action {
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

    fn block_entity(&self, _host: &mut dyn Host, event: &mut BlockEntityEvent<'_>) -> Option<(String, Compound)> {
        block_entity_api_v1::create(event.block).map(|(id, data)| (id, compound_to_tag(&data)))
    }

    fn place_block(&self, host: &mut dyn Host, event: &mut PlaceBlockEvent<'_>) {
        let pos = event.pos;
        let block = host.block(pos);
        if !block_entity_api_v1::has_place_hook(&block) {
            return;
        }
        let Some(stack) = slot_of(event.stack) else { return };
        let player = host.acting_player().map(|player| player_api::ServerPlayer { uuid: player.uuid, name: player.name.clone() });
        // The placement hook sees the same reach as a use: its dimension, the world, and the neighbours' containers and tanks.
        let dimension = host.dimension().to_owned();
        let handle = HostHandle::new(host);
        let mut ran = false;
        world_api_v1::with_ticking_dimension(&dimension, || {
            world_api_v1::with_world(&mut HostWorld(handle), || {
                neighbour_api_v1::with_neighbours(&mut HostNeighbours { host: handle, dimension: dimension.clone() }, || {
                    host.change_mod_block_entity(pos, &mut |entity| {
                        ran = with_block_entity(entity, |data, items| block_entity_api_v1::place(&block, pos, stack.clone(), player.clone(), data, items));
                    });
                });
            });
        });
        if ran {
            host.block_entity_sync(pos);
        }
    }

    fn block_entity_container(&self, _host: &dyn Host, event: &mut BlockEntityContainerEvent<'_>) -> Option<ModContainer> {
        let slots = block_entity_api_v1::item_slots(event.block)? as usize;
        // A type with sided access declares the faces it opens; every other opens every slot every way.
        if let Some((faces, inputs, outputs)) = block_entity_api_v1::container_access(event.block) {
            return Some(ModContainer { slots, faces: std::array::from_fn(|face| faces.get(face).cloned().unwrap_or_default()), inputs, outputs, menu: block_entity_api_v1::menu(event.block) });
        }
        let all: Vec<usize> = (0..slots).collect();
        Some(ModContainer { slots, faces: std::array::from_fn(|_| all.clone()), inputs: all.clone(), outputs: all, menu: block_entity_api_v1::menu(event.block) })
    }

    fn block_entity_fluids(&self, _host: &dyn Host, event: &mut BlockEntityFluidsEvent<'_>) -> Option<ModFluidTanks> {
        let tanks = block_entity_api_v1::fluid_tanks(event.block)?;
        let all: Vec<usize> = (0..tanks.len()).collect();
        Some(ModFluidTanks { tanks: tanks.iter().map(|&c| c as i32).collect(), faces: std::array::from_fn(|_| all.clone()), inputs: all.clone(), outputs: all })
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        if block_entity_api_v1::ticks(event.block) {
            event.ticks = true;
        }
    }

    fn block_entity_tick(&self, host: &mut dyn Host, event: &mut BlockEntityTickEvent<'_>) {
        let (pos, id) = (event.pos, event.entity.id.clone());
        let dimension = host.dimension().to_owned();
        // The signal reaching the block, for a type that hears it change: read before the host handle wraps the host.
        let block = block_entity_api_v1::block_of(&id);
        let signal = block.as_deref().is_some_and(block_entity_api_v1::has_signal_hook)
            .then(|| host.neighbor_signal(&dimension, pos).clamp(0, 15) as u8)
            .filter(|strength| {
                // Only a change is reported (the first sight of a block entity counts as one, so a hook can settle its state).
                let key = (dimension.clone(), pos);
                match LAST_SIGNAL.lock() {
                    Ok(mut last) if last.get(&key) != Some(strength) => { last.insert(key, *strength); true }
                    Ok(_) => false,
                    Err(_) => true,
                }
            });
        let handle = HostHandle::new(host);
        let entity = &mut *event.entity;
        // The ticker sees the world of its dimension (reads, and block state writes), its neighbours' containers and tanks, the
        // entities around it, and which dimension it ticks in. The ticking entity itself is out of the world, as the host documents.
        world_api_v1::with_ticking_dimension(&dimension, || {
            world_api_v1::with_world(&mut HostWorld(handle), || {
                neighbour_api_v1::with_neighbours(&mut HostNeighbours { host: handle, dimension: dimension.clone() }, || {
                    entity_api_v1::with_entities(&mut HostEntities { host: handle, dimension: dimension.clone() }, || {
                        if let Some(strength) = signal {
                            with_block_entity(entity, |data, items| block_entity_api_v1::signal(&id, pos, data, items, strength));
                        }
                        with_block_entity(entity, |data, items| block_entity_api_v1::tick(&id, pos, data, items));
                    });
                });
            });
        });
    }

    fn custom_block_fluids(&self, _host: &dyn Host, event: &mut CustomBlockFluidsEvent<'_>) {
        // A block with its own fluid rule (`fluid_filler`) answers fills itself; every other mod block keeps the host's tank storage.
        if block_entity_api_v1::has_fluid_filler(event.block) {
            event.custom = true;
        }
    }

    fn custom_fluid_tanks(&self, host: &dyn Host, event: &mut CustomFluidTanksEvent) -> Option<Vec<NativeFluidTankView>> {
        let entity = host.mod_block_entity(event.pos)?;
        let block = block_entity_api_v1::block_of(&entity.id)?;
        let tanks = block_entity_api_v1::tank_contents(&block, &compound_from_tag(&entity.nbt))?;
        Some(tanks.into_iter().enumerate().map(|(tank, (content, capacity))| NativeFluidTankView {
            tank,
            content: content.map(|stack| NativeFluidStack { fluid: stack.fluid.to_string(), amount: stack.amount.min(i32::MAX as u32) as i32, components: stack.tag.as_ref().map(compound_to_tag).unwrap_or_default() }),
            capacity: capacity.min(i32::MAX as u32) as i32,
            can_fill: true,
            can_drain: false,
        }).collect())
    }

    fn custom_fluid_fill(&self, host: &mut dyn Host, event: &mut CustomFluidFillEvent<'_>) -> Option<i32> {
        let fluid = Identifier::parse(&event.resource.fluid).ok()?;
        let tag = (!event.resource.components.is_empty()).then(|| compound_from_tag(&event.resource.components));
        let offered = FluidStack { fluid, amount: u32::try_from(event.resource.amount).ok()?, tag };
        let face = event.side.map(|side| direction(side as u8));
        let execute = matches!(event.action, NativeFluidAction::Execute);
        let taken = filler_takes(host, event.pos, face, &offered, execute).unwrap_or(0);
        Some(taken.min(offered.amount) as i32)
    }

    fn custom_fluid_drain(&self, _host: &mut dyn Host, _event: &mut CustomFluidDrainEvent<'_>) -> Option<NativeFluidStack> {
        // A block with its own fluid rule only takes fluid in; nothing is drained out of it by another block.
        None
    }

    fn right_click_block_v2(&self, host: &mut dyn Host, event: &mut RightClickBlockV2Event) -> Flow {
        use crate::pure::base::action_result::ActionResult;
        let pos = event.pos;
        let block = host.block(pos);
        if !block_entity_api_v1::has_use_hook(&block) {
            return Flow::Continue;
        }
        let Some(player) = host.acting_player() else { return Flow::Continue };
        let slot = if matches!(event.hand, Hand::Off) { EquipmentSlot::OffHand } else { EquipmentSlot::MainHand };
        let held = host.player_equipment(player.uuid, slot).filter(|stack| stack.count > 0).and_then(|stack| slot_of(&stack));
        let sneaking = host.sneaking();
        let mut outcome = None;
        let mut hand = Some(held.clone());
        // The use handler sees the same reach as a ticker: its dimension, the world, and the neighbours' containers and
        // tanks. Outside that reach every neighbour query answers "nothing there", so a faucet could never find the tank
        // behind it or the table below it, and a click on it would change nothing.
        let dimension = host.dimension().to_owned();
        let handle = HostHandle::new(host);
        world_api_v1::with_ticking_dimension(&dimension, || {
            world_api_v1::with_world(&mut HostWorld(handle), || {
                neighbour_api_v1::with_neighbours(&mut HostNeighbours { host: handle, dimension: dimension.clone() }, || {
                    host.change_mod_block_entity(pos, &mut |entity| {
                        let Some(hand) = hand.take() else { return };
                        with_block_entity(entity, |data, items| {
                            outcome = block_entity_api_v1::BlockEntityTypeRegistry::use_block(&block, pos, data, items, hand, sneaking);
                            true
                        });
                    });
                });
            });
        });
        let Some(outcome) = outcome else { return Flow::Continue };
        host.block_entity_sync(pos);
        if outcome.held != held {
            let living = host.entity_by_uuid(player.uuid).and_then(|(_, id, _)| host.entity(id)).and_then(|entity| host.living(entity));
            if let Some(living) = living {
                living.set_equipment(host, slot, native_slot(outcome.held.clone()));
            }
        }
        for given in outcome.give {
            let Some(stack) = native_slot(Some(given)) else { continue };
            let Some(left) = host.give_stack(player.uuid, stack) else { continue };
            // The inventory is full: the rest lands at the player's feet.
            let mut item = NbtCompound::new();
            item.insert("id".to_owned(), Nbt::String(left.item.to_string()));
            item.insert("count".to_owned(), Nbt::Int(left.count.min(i32::MAX as u32) as i32));
            let mut nbt = NbtCompound::new();
            nbt.insert("Item".to_owned(), Nbt::Compound(item));
            host.summon(&player.dimension, "minecraft:item", player.position, compound_to_tag(&nbt));
        }
        if matches!(outcome.result, ActionResult::Pass) { Flow::Continue } else { Flow::Cancel }
    }

    fn menu_button_v2(&self, _host: &mut dyn Host, event: &mut MenuButtonV2Event<'_>) {
        let (pos, button) = (event.pos, event.id);
        let Some(block) = block_entity_api_v1::block_of(&event.entity.id) else { return };
        let mut carried: Option<ItemSlot> = event.carried.as_ref().and_then(slot_of);
        let before = carried.clone();
        with_block_entity(event.entity, |data, items| screen_handler_api_v1::press(&block, pos, button, data, items, &mut carried));
        if carried != before {
            *event.carried = carried.filter(|s| s.count > 0).map(|s| {
                let mut stack = ItemStack::new(&s.item.to_string(), s.count);
                write_slot(&mut stack, &s);
                stack
            });
        }
    }

    fn menu_slot_rules(&self, event: &mut MenuSlotRulesEvent<'_>) {
        let Some(block) = block_entity_api_v1::block_of_menu(event.menu) else { return };
        screen_handler_api_v1::slot_rules(&block, event.slot, &event.stack.item, event.stack.count, &mut event.may_place, &mut event.max_stack);
    }

    fn menu_quick_move(&self, _host: &mut dyn Host, event: &mut MenuQuickMoveEvent<'_>) -> Flow {
        let Some(block) = block_entity_api_v1::block_of(&event.entity.id) else { return Flow::Continue };
        let (pos, slot, container_slots) = (event.pos, event.slot, event.container_slots);
        let handled = with_quick_move(event.entity, event.stack, |moved, data, items| {
            screen_handler_api_v1::quick_move(&block, pos, slot, container_slots, moved, data, items)
        });
        if handled {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn container_accepts(&self, _host: &dyn Host, event: &mut ContainerAcceptsEvent<'_>) -> Flow {
        let Some(block) = block_entity_api_v1::block_of(&event.entity.id) else { return Flow::Continue };
        let face = event.face.map(|d| direction(d as u8));
        if !screen_handler_api_v1::accepts(&block, event.pos, event.slot, &event.stack.item, event.stack.count, face) {
            return Flow::Cancel;
        }
        // The block entity type's own transfer rule answers too, with its saved data at hand (a menu's rules cannot see it).
        // The event only lends the entity, so the rule is asked over a snapshot, as the renderer reads one.
        if block_entity_api_v1::item_served_from_data(&block) {
            if let Some(stack) = slot_of(event.stack) {
                let data = compound_from_tag(&event.entity.nbt);
                let items: Vec<Option<ItemSlot>> = event.entity.items.iter().map(|slot| slot.as_ref().and_then(slot_of)).collect();
                if !block_entity_api_v1::BlockEntityTypeRegistry::item_transfer_allowed(&block, event.pos, &data, &items, event.slot, face, block_entity_api_v1::TransferMode::Insert, stack) {
                    return Flow::Cancel;
                }
            }
        }
        Flow::Continue
    }

    fn block_entity_submit(&self, host: &dyn Host, event: &mut BlockEntitySubmitEvent<'_>) {
        if !block_entity_renderer_api_v1::BlockEntityRendererRegistry::has(event.block) {
            return;
        }
        let data = compound_from_tag(&event.entity.nbt);
        // A snapshot from this exact native entity, never its saved NBT or a
        // global position cache. Busy/poisoned stays unavailable. The generated
        // state getter and this adapter await native attachment plan/native validation.
        let client_data = event.entity.__horizon_mixins_pluto_lib_pluto_block_entity_client_state()
            .snapshot().ok().map(|snapshot| compound_from_tag(&snapshot));
        let mut canvas = HostDraw { host, draw: &mut *event.draw, state: event.state,
            client_data: client_data.as_ref() };
        // The renderer also sees the entity's item slots: they live outside the saved data, and a renderer that
        // draws the contents (a melter's items, a cast on a table) has no other way to reach them.
        let items: Vec<Option<ItemSlot>> = event.entity.items
            .iter()
            .map(|slot| slot.as_ref().and_then(slot_of))
            .collect();
        block_entity_renderer_api_v1::render(event.block, event.pos, &data, &items, &mut canvas);
    }

    fn screen(&self, _host: &dyn Host, event: &mut ScreenEvent<'_>) -> Option<NativeScreen> {
        let layout = screen_handler_api_v1::layout(event.block)?;
        let mut screen = NativeScreen::default();
        screen.width = layout.width;
        screen.height = layout.height;
        screen.slots = layout.slots;
        screen.inventory_y = layout.inventory_y;
        screen.inventory_x = layout.inventory_x;
        screen.own_labels = layout.own_labels;
        screen.own_background = layout.own_background;
        if screen_handler_api_v1::has_dynamic_slots(event.block) {
            horizon_client::game::pluto_dynamic_screen_slots::set_resolver(resolve_dynamic_screen_slots);
        }
        Some(screen)
    }

    fn screen_draw(&self, host: &dyn Host, event: &mut ScreenDrawEvent<'_>) {
        let mut canvas = HostCanvas { host, batch: &mut *event.batch };
        screen_handler_api_v1::draw(event.block, event.origin, event.data, &mut canvas);
    }

    fn screen_click(&self, _host: &dyn Host, event: &mut ScreenClickEvent<'_>) -> Option<i32> {
        screen_handler_api_v1::click(event.block, event.at, event.right, event.data)
    }

    fn screen_tooltip(&self, _host: &dyn Host, event: &mut ScreenTooltipEvent<'_>) {
        screen_handler_api_v1::tooltip(event.block, event.at, event.data, event.lines);
    }

    fn screen_render_post(&self, host: &dyn Host, event: &mut ScreenRenderPostEvent<'_>) {
        // PlutoLib's own mod screen is where its books open: draw the page the reader is on.
        if !book_screen_is_ours(host, event.screen.id) {
            return;
        }
        let Some((book, view)) = open_book_view() else { return };
        let mut canvas = HostCanvas { host, batch: &mut *event.batch };
        book_api_v1::draw_book(&mut canvas, &book, &view, event.screen.rect);
    }

    fn screen_mouse_button_pressed_pre(&self, host: &dyn Host, event: &mut ScreenMouseButtonPressedPreEvent<'_>) -> Flow {
        if event.button != 0 || !book_screen_is_ours(host, event.screen.id) {
            return Flow::Continue;
        }
        let Some((book, mut view)) = open_book_view() else { return Flow::Continue };
        let turned = match book_api_v1::book_click(&book, &view, event.screen.rect, event.screen.mouse) {
            book_api_v1::BookClick::NextPage => view.next_page(&book),
            book_api_v1::BookClick::PrevPage => view.prev_page(&book),
            book_api_v1::BookClick::Section(section) => view.goto_section(&book, section),
            book_api_v1::BookClick::None => return Flow::Continue,
        };
        if turned {
            set_book_view(view);
        }
        Flow::Cancel
    }

    fn screen_mouse_scrolled_pre(&self, host: &dyn Host, event: &mut ScreenMouseScrolledPreEvent<'_>) -> Flow {
        if !book_screen_is_ours(host, event.screen.id) {
            return Flow::Continue;
        }
        let Some((book, mut view)) = open_book_view() else { return Flow::Continue };
        // A negative vertical (the wheel rolled down) turns forward, a positive one back.
        let turned = if event.scroll[1] < 0.0 {
            view.next_page(&book)
        } else if event.scroll[1] > 0.0 {
            view.prev_page(&book)
        } else {
            false
        };
        if turned {
            set_book_view(view);
        }
        Flow::Cancel
    }

    fn screen_key_pressed_pre(&self, host: &dyn Host, event: &mut ScreenKeyPressedPreEvent<'_>) -> Flow {
        if !book_screen_is_ours(host, event.screen.id) {
            return Flow::Continue;
        }
        let Some((book, mut view)) = open_book_view() else { return Flow::Continue };
        let key = event.key.to_ascii_lowercase();
        let turned = match key.as_str() {
            "arrowright" | "right" | "pagedown" | "space" => view.next_page(&book),
            "arrowleft" | "left" | "pageup" => view.prev_page(&book),
            "home" => {
                let moved = view.page != 0;
                view.page = 0;
                moved
            }
            "end" => {
                let last = book.page_count().saturating_sub(1);
                let moved = view.page != last;
                view.page = last;
                moved
            }
            _ => return Flow::Continue,
        };
        if turned {
            set_book_view(view);
        }
        Flow::Cancel
    }

    fn screen_closing(&self, host: &dyn Host, event: &mut ScreenClosingEvent<'_>) {
        if event.screen.id != "mod" {
            return;
        }
        // The reader closed the book (Escape, or the game moved on): forget where they were. While the
        // screen closes the game may already report no mod screen, so a held view with no other owner
        // named is treated as ours.
        if host.mod_screen().is_none() || host.mod_screen() == Some(PLUTO_MOD_ID) {
            if let Ok(mut view) = BOOK_VIEW.lock() {
                *view = None;
            }
        }
    }

    fn block_entity_update_tag(&self, event: &mut BlockEntityUpdateTagEvent<'_>) -> Option<Compound> {
        block_entity_api_v1::update_tag(&event.entity.id, &compound_from_tag(&event.entity.nbt)).map(|tag| compound_to_tag(&tag))
    }

    fn hears_projectile_impacts(&self, _host: &dyn Host, event: &mut HearsProjectileImpactsEvent) {
        if projectile_events_v1::heard() {
            event.hears = true;
        }
    }

    fn projectile_impact(&self, host: &mut dyn Host, event: &mut ProjectileImpactEvent<'_>) -> Flow {
        // The native telling names the hit, not the projectile's flight: its motion and its saved data
        // are read off the live entity while it is still here.
        let entity = host.entity(event.projectile);
        let hit = ProjectileImpact {
            dimension: event.dimension.to_owned(),
            projectile: event.projectile,
            kind: event.kind.to_owned(),
            owner: i64::from(event.owner),
            entity: event.entity,
            entity_kind: event.entity_kind.to_owned(),
            block: event.block,
            face: event.face,
            at: event.at,
            motion: entity.and_then(|entity| host.entity_velocity(entity)).unwrap_or([0.0; 3]),
            data: entity.and_then(|entity| host.entity_nbt(entity)).map(|data| compound_from_tag(&data)).unwrap_or_default(),
            discard: std::cell::Cell::new(false),
            motion_after: std::cell::RefCell::new(None),
            actions: std::cell::RefCell::new(Vec::new()),
        };
        // The callbacks see the world and the entities around the hit, as a block entity tick does;
        // afterwards the actions they queued are done to the hit's parties (the entity it hit as the
        // victim, the thrower as the attacker; a block hit has no victim, so victim actions go
        // nowhere), whether the hit itself is cancelled or not.
        let handle = HostHandle::new(host);
        let dimension = event.dimension.to_owned();
        let allowed = scoped(host, || entity_api_v1::with_entities(&mut HostEntities { host: handle, dimension }, || projectile_events_v1::impact(&hit)));
        let actions: Vec<entity_events_v1::EntityAction> = hit.actions.borrow_mut().drain(..).collect();
        if !actions.is_empty() {
            let victim = event.entity.and_then(|id| i32::try_from(id).ok()).unwrap_or(-1);
            apply_entity_actions(host, event.dimension, event.owner, victim, actions);
        }
        // What the callbacks settled on the projectile itself, done as the hook returns: a discard
        // removes it (the game's own remove call, which discards projectiles), a new motion sends it
        // flying on — the bounce a cancelled hit turns into.
        if hit.discard.get() {
            host.remove_entity(event.dimension, event.projectile);
        } else if let Some(motion) = hit.motion_after.borrow_mut().take() {
            if let Some(entity) = entity {
                host.entity_set_velocity(entity, motion);
            }
        }
        if allowed { Flow::Continue } else { Flow::Cancel }
    }

    fn projectile_launch(&self, host: &mut dyn Host, event: &mut horizon_client::mod_api::ProjectileLaunchEvent<'_>) {
        if !projectile_events_v1::launch_heard() { return; }
        let entity = host.entity(event.entity);
        let launch = projectile_events_v1::ProjectileLaunch {
            player: event.player, name: event.name.to_owned(), projectile: event.entity, kind: event.kind.to_owned(),
            motion: entity.and_then(|entity| host.entity_velocity(entity)).unwrap_or([0.0; 3]),
            data: entity.and_then(|entity| host.entity_nbt(entity)).map(|data| compound_from_tag(&data)).unwrap_or_default(),
        };
        scoped(host, || projectile_events_v1::launched(&launch));
    }

    fn entity_shoot_bow(&self, host: &mut dyn Host, event: &mut horizon_client::mod_api::EntityShootBowEvent<'_>) {
        if !projectile_events_v1::shot_heard() { return; }
        let player = event.player;
        // The bow is in one of the shooter's hands: the callbacks get both stacks, as the knockback hook gives them.
        let main_hand = host.held_stack(player).map(|stack| read_stack(&stack));
        let off_hand = host.player_equipment(player, horizon_client::mod_api::EquipmentSlot::OffHand).map(|stack| read_stack(&stack));
        let shot = projectile_events_v1::BowShot { player, name: event.name.to_owned(), projectile: event.projectile, force: event.force, main_hand, off_hand, actions: std::cell::RefCell::new(Vec::new()) };
        let actions = scoped(host, || projectile_events_v1::shot(&shot));
        if !actions.is_empty() { apply_player_actions(host, player, actions); }
    }

    fn player_shear_entity(&self, host: &mut dyn Host, event: &mut horizon_client::mod_api::PlayerShearEntityEvent<'_>) -> Flow {
        let player = event.player;
        let held = if event.off_hand { host.player_equipment(player, horizon_client::mod_api::EquipmentSlot::OffHand) } else { host.held_stack(player) };
        let attempt = entity_events_v1::ShearAttempt {
            player, name: event.name.to_owned(), entity: event.entity_id, off_hand: event.off_hand, tool: held.map(|stack| read_stack(&stack)),
            cancel: std::cell::Cell::new(false), actions: std::cell::RefCell::new(Vec::new()),
        };
        let (allowed, actions) = scoped(host, || entity_events_v1::shear(&attempt));
        if !actions.is_empty() { apply_player_actions(host, player, actions); }
        cancel_if(!allowed)
    }

    fn living_get_projectile(&self, host: &mut dyn Host, event: &mut LivingGetProjectileEvent<'_>) {
        if !projectile_events_v1::ammo_heard() {
            return;
        }
        let request = projectile_events_v1::AmmoRequest {
            player: event.player,
            name: event.name.to_owned(),
            entity: event.entity,
            kind: event.kind.to_owned(),
            weapon: read_stack(event.weapon),
            vanilla: event.projectile.as_ref().and_then(|stack| slot_of(stack)),
            // Only the acting player's inventory is readable (the host's carried reach names no player); a mob's request, or
            // another player's, carries no stacks and can only be answered from what the callback itself knows.
            carried: event.player.filter(|uuid| host.acting_player().is_some_and(|acting| acting.uuid == *uuid)).map(|uuid| carried_stacks(host, uuid)).unwrap_or_default(),
            supplied: std::cell::RefCell::new(None),
            edits: std::cell::RefCell::new(Vec::new()),
        };
        let (supplied, edits) = scoped(host, || projectile_events_v1::ammo(&request));
        if let Some(supplied) = supplied {
            // The game shrinks nothing when the projectile is a mod's: the callback's edits are its consumption.
            *event.projectile = native_slot(Some(supplied));
        }
        if let Some(player) = event.player {
            apply_carried_edits(host, player, edits);
        }
    }

    fn key(&self, host: &mut dyn Host, event: &mut KeyEvent<'_>) {
        if !player_api::key_heard() {
            return;
        }
        let press = player_api::KeyPress {
            player: host.acting_player().map(|player| player_api::ServerPlayer { uuid: player.uuid, name: player.name.clone() }),
            key: event.key.to_owned(),
            pressed: event.pressed,
        };
        scoped(host, || player_api::key_press(&press));
    }

    fn interact_entity_v2(&self, host: &mut dyn Host, event: &mut InteractEntityV2Event<'_>) -> Flow {
        if !events_interaction_v0::use_item_on_entity_heard() {
            return Flow::Continue;
        }
        let player = host.acting_player();
        let context = events_interaction_v0::UseItemOnEntityContext {
            entity: event.entity,
            kind: event.kind.to_owned(),
            hand: match event.hand { Hand::Off => events_interaction_v0::Hand::Off, _ => events_interaction_v0::Hand::Main },
            held: event.stack.map(read_stack),
            player: player.as_ref().map(|player| player_api::ServerPlayer { uuid: player.uuid, name: player.name.clone() }),
            actions: std::cell::RefCell::new(Vec::new()),
        };
        let dimension = player.as_ref().map_or_else(|| host.dimension().to_owned(), |player| player.dimension.to_string());
        let handle = HostHandle::new(host);
        let handled = scoped(host, || entity_api_v1::with_entities(&mut HostEntities { host: handle, dimension }, || events_interaction_v0::use_item_on_entity(&context)));
        let actions = context.actions.take();
        if let Some(player) = player {
            if !actions.is_empty() { apply_player_actions(host, player.uuid, actions); }
        }
        cancel_if(handled)
    }

    fn interact_entity(&self, host: &mut dyn Host, event: &mut InteractEntityEvent<'_>) -> Flow {
        if !events_interaction_v0::entity_interact_heard() {
            return Flow::Continue;
        }
        let player = host.acting_player();
        let context = events_interaction_v0::InteractEntityContext {
            entity: event.entity,
            kind: event.kind.to_owned(),
            stack: event.stack.map(read_stack),
            player: player.as_ref().map(|player| player_api::ServerPlayer { uuid: player.uuid, name: player.name.clone() }),
        };
        let dimension = player.as_ref().map_or_else(|| host.dimension().to_owned(), |player| player.dimension.to_string());
        let handle = HostHandle::new(host);
        let handled = scoped(host, || entity_api_v1::with_entities(&mut HostEntities { host: handle, dimension }, || events_interaction_v0::entity_interact(&context)));
        cancel_if(handled)
    }

    fn inventory_tick(&self, host: &mut dyn Host, event: &mut InventoryTickEvent) {
        if !player_api::inventory_tick_heard() {
            return;
        }
        let player = host.acting_player();
        let tick = player_api::InventoryTick {
            player: player.as_ref().map(|player| player_api::ServerPlayer { uuid: player.uuid, name: player.name.clone() }),
            slot: event.slot,
            selected: event.selected,
            stack: host.carried_stack(event.slot).filter(|stack| stack.count > 0).map(read_stack),
        };
        let dimension = player.as_ref().map_or_else(|| host.dimension().to_owned(), |player| player.dimension.to_string());
        let handle = HostHandle::new(host);
        scoped(host, || entity_api_v1::with_entities(&mut HostEntities { host: handle, dimension }, || player_api::inventory_tick(&tick)));
    }

    fn entity_tick_pre(&self, host: &mut dyn Host, event: &mut EntityTickPreEvent<'_>) -> Flow {
        if !entity_events_v1::entity_tick_heard() {
            return Flow::Continue;
        }
        let info = entity_events_v1::EntityInfo { id: event.entity, kind: event.kind.to_owned(), dimension: event.dimension.to_owned() };
        let handle = HostHandle::new(host);
        let allowed = scoped(host, || entity_api_v1::with_entities(&mut HostEntities { host: handle, dimension: info.dimension.clone() }, || entity_events_v1::entity_tick_before(&info)));
        cancel_if(!allowed)
    }

    fn entity_tick_post(&self, host: &mut dyn Host, event: &mut EntityTickPostEvent<'_>) {
        if !entity_events_v1::entity_tick_heard() {
            return;
        }
        let info = entity_events_v1::EntityInfo { id: event.entity, kind: event.kind.to_owned(), dimension: event.dimension.to_owned() };
        let handle = HostHandle::new(host);
        scoped(host, || entity_api_v1::with_entities(&mut HostEntities { host: handle, dimension: info.dimension.clone() }, || entity_events_v1::entity_tick_after(&info)));
    }

    fn player_fish_v2(&self, host: &mut dyn Host, event: &mut PlayerFishV2Event<'_>) -> Flow {
        if !events_interaction_v0::fishing_heard() {
            return Flow::Continue;
        }
        let rod = match event.hand {
            Hand::Off => host.player_equipment(event.player, EquipmentSlot::OffHand),
            _ => host.held_stack(event.player),
        }.map(|stack| read_stack(&stack));
        let context = events_interaction_v0::FishingContext {
            player: player_api::ServerPlayer { uuid: event.player, name: event.name.to_owned() },
            state: event.state,
            hand: match event.hand { Hand::Off => events_interaction_v0::Hand::Off, _ => events_interaction_v0::Hand::Main },
            rod,
            cancel: std::cell::Cell::new(false),
            actions: std::cell::RefCell::new(Vec::new()),
        };
        scoped(host, || events_interaction_v0::fishing(&context));
        let actions = context.actions.take();
        if !actions.is_empty() { apply_player_actions(host, event.player, actions); }
        cancel_if(context.cancel.get())
    }

    fn loot_drops(&self, host: &mut dyn Host, event: &mut horizon_client::mod_api::LootDropsEvent<'_>) {
        // The game's own death-loot path (pluto-living-drops-game) calls the hook for a mob's table drops; heads claimed through the living-drops
        // callbacks are appended outside it, so this hook only sees and changes the stacks the game passes here.
        if !loot_api_v2::drops_heard() { return; }
        let Some(slots) = event.drops.iter().map(slot_of).collect::<Option<Vec<ItemSlot>>>() else { return };
        let before = slots.clone();
        let context = &*event.context;
        let drops = loot_api_v2::LootDrops {
            table: context.table.to_owned(), dimension: context.dimension.to_owned(), origin: context.origin,
            block: context.block.as_ref().map(|(pos, _)| *pos),
            entity: context.entity.as_ref().map(|(id, kind)| (*id, (*kind).to_owned())),
            player: context.player, tool: context.tool.map(|stack| read_stack(stack)), explosion: context.explosion, used: context.used,
            drops: std::cell::RefCell::new(slots), actions: std::cell::RefCell::new(Vec::new()),
        };
        let after = scoped(host, || loot_api_v2::modify_drops(&drops));
        let actions = drops.actions.take();
        if let (Some(player), false) = (drops.player, actions.is_empty()) { apply_player_actions(host, player, actions); }
        if after != before {
            *event.drops = after.into_iter().filter_map(|slot| native_slot(Some(slot))).collect();
        }
    }

    fn register_commands(&self, _host: &mut dyn Host, event: &mut RegisterCmds<'_>) {
        for (usage, level) in command_api_v2::collect() {
            event.commands.push(if level == 0 { NativeCommand::new(&usage) } else { NativeCommand::op(&usage, level) });
        }
    }

    fn perform_command(&self, host: &mut dyn Host, event: &mut PerformCommandEvent<'_>) -> Flow {
        let (uuid, name, line) = (event.player.uuid, event.player.name.clone(), event.command.to_owned());
        let ran = scoped(host, || command_api_v2::execute(uuid, name, &line));
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

    fn server_tick(&self, host: &mut dyn Host, event: &mut ServerTickEvent<'_>) {
        scoped(host, || match event.phase {
            ServerTickPhase::ServerTickStart { .. } => {
                crate::pure::i18n_api::poll_language();
                lifecycle_events_v1::server_tick(true)
            }
            ServerTickPhase::ServerTickEnd { .. } => lifecycle_events_v1::server_tick(false),
            #[allow(unreachable_patterns)]
            _ => {}
        });
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
        remember_player(uuid, &name, true);
        scoped(host, || networking_api_v1::player_joined(uuid, name));
    }

    fn player_tick(&self, host: &mut dyn Host, _event: &mut horizon_client::mod_api::PlayerTickEvent) {
        // Damage hooks get no host, so the armor a player wears is read here each tick for them to find.
        let Some(player) = host.acting_player() else { return };
        flush_block_breaks(host, player.uuid);
        let mut armor = Vec::new();
        for (name, slot) in [("feet", horizon_client::mod_api::EquipmentSlot::Feet), ("legs", horizon_client::mod_api::EquipmentSlot::Legs), ("chest", horizon_client::mod_api::EquipmentSlot::Chest), ("head", horizon_client::mod_api::EquipmentSlot::Head)] {
            let Some(stack) = host.player_equipment(player.uuid, slot) else { continue };
            let data = stack_data_of(host, &stack);
            if !data.custom_data.is_empty() { armor.push((name.to_string(), data)); }
        }
        // Armor that carries custom data may take effects off its wearer (an immunity): the mods hear the tick with the effects.
        let worn = armor.clone();
        // The state of a wearer, for the damage event of a hit on them (their health and surroundings as of this tick).
        let wearer_state = (!armor.is_empty()).then(|| player_entity_state(host, &player));
        entity_events_v1::remember_wearer_state(player.entity_id, wearer_state);
        entity_events_v1::remember_armor(player.uuid, player.entity_id, armor);
        if entity_events_v1::tick_wanted(player.uuid, !worn.is_empty()) {
            let effects = host.effects(player.uuid).unwrap_or_default().into_iter().map(|effect| (effect.id, i32::from(effect.amplifier), effect.duration)).collect();
            let tick = entity_events_v1::PlayerTick { player: player.uuid, entity: player.entity_id, dimension: player.dimension.to_string(), position: player.position, on_ground: player.on_ground, armor: worn, effects, remove_effects: std::cell::RefCell::new(Vec::new()), attribute_additions: std::cell::RefCell::new(Vec::new()), actions: std::cell::RefCell::new(Vec::new()) };
            // The world scope is open while the tick runs, so a callback may work on the blocks around the player.
            for id in scoped(host, || entity_events_v1::player_tick(&tick)) { host.remove_effect(player.uuid, &id); }
            // What the callbacks queued for the ticking player is done to them now, as `Attacker`; the other party is no one.
            let own = entity_events_v1::take_tick_actions(&tick);
            if !own.is_empty() { apply_entity_actions(host, &player.dimension, player.entity_id, -1, own); }
            // The amounts the armor adds to the player's attributes move the base value by the change since the last tick.
            let wanted = tick.attribute_additions.take();
            for (attribute, change, total) in entity_events_v1::attribute_changes(player.uuid, &wanted) {
                let Some((base, _)) = host.attribute(player.uuid, &attribute) else { continue };
                if host.set_attribute_base(player.uuid, &attribute, base + change) { entity_events_v1::attribute_applied(player.uuid, &attribute, total); }
            }
        }
        // Damage the armor's thorns deal to the mobs that hit the wearer lands here, as hurt by the player.
        for (mob, damage) in entity_events_v1::take_counter_damage(player.entity_id) {
            // Thorns hurt the mob as the player does, which reaches the weapon-hit hook; that hit is not the player's weapon.
            if let Ok(target) = u32::try_from(mob) {
                COUNTER_HURT.with(|flag| flag.set(true));
                host.hurt_entity(&player.dimension, target, damage.amount, &damage.damage_type, u32::try_from(player.entity_id).ok());
                COUNTER_HURT.with(|flag| flag.set(false));
            }
        }
        // Actions the armor's counters do to the mob that hit the wearer (fire, freezing, knockback) land here too.
        for (mob, action) in entity_events_v1::take_wearer_actions(player.entity_id) {
            apply_entity_actions(host, &player.dimension, mob, player.entity_id, vec![action]);
        }
        // What the player's own attacks asked for on the attacker (a heal by the damage dealt) lands here, with the damaged entity as the other party.
        for (victim, action) in entity_events_v1::take_attacker_actions(player.entity_id) {
            apply_entity_actions(host, &player.dimension, player.entity_id, victim, vec![action]);
        }
        // Effects the armor's counters put on the wearer land here, where a host is at hand.
        for grant in entity_events_v1::take_self_effects(player.entity_id) {
            if grant.duration > 0 && grant.amplifier >= 0 {
                host.add_effect(player.uuid, &horizon_client::game::NativeEffect { id: grant.effect, amplifier: u8::try_from(grant.amplifier).unwrap_or(u8::MAX), duration: grant.duration, ambient: false, particles: true });
            }
        }
    }

    fn player_logged_out(&self, host: &mut dyn Host, event: &mut PlayerLoggedOutEvent<'_>) {
        let (uuid, name) = (event.player, event.name.to_string());
        remember_player(uuid, &name, false);
        entity_events_v1::forget_armor(uuid);
        // The base values are saved with the player, so what the armor added is taken off before they go (best effort: the host may no longer reach them).
        for (attribute, held) in entity_events_v1::forget_attributes(uuid) {
            if let Some((base, _)) = host.attribute(uuid, &attribute) { host.set_attribute_base(uuid, &attribute, base - held); }
        }
        scoped(host, || networking_api_v1::player_left(uuid, name));
    }

    fn entity_join_level(&self, host: &mut dyn Host, event: &mut EntityJoinLevelEvent<'_>) -> Flow {
        scoped(host, || entity_events_v1::entity_loaded(event.entity, event.kind, event.dimension));
        Flow::Continue
    }

    fn entity_leave_level(&self, host: &mut dyn Host, event: &mut EntityLeaveLevelEvent<'_>) {
        scoped(host, || entity_events_v1::entity_unloaded(event.entity, event.kind, event.dimension));
    }

    fn use_item(&self, host: &mut dyn Host, event: &mut UseItemEvent<'_>) -> Flow {
        // Scoped like use_item_on and the finish event: a callback here may reach for the player, the
        // world and the client reaches (a book item opens its book), which only exist inside the scope.
        cancel_if(scoped(host, || events_interaction_v0::use_item(&event.stack.item, event.stack.count)))
    }

    fn use_item_on(&self, host: &mut dyn Host, event: &mut UseItemOnEvent<'_>) -> Flow {
        // The host names the acting player only for a guest's action; the world's own player is the only one logged in then.
        let (uuid, name, dimension, creative) = match host.acting_player() {
            Some(player) => (player.uuid, player.name.clone(), player.dimension.clone(), player.game_mode == 1),
            None => match sole_player() {
                Some((uuid, name)) => (uuid, name, host.dimension().to_string(), false),
                None => return Flow::Continue,
            },
        };
        let context = events_interaction_v0::UseItemOnBlockContext {
            held: Some(stack_data_of(host, &event.stack)),
            item: event.stack.item.to_string(),
            count: event.stack.count,
            pos: event.pos,
            face: event.face,
            dimension,
            creative,
            player: player_api::ServerPlayer { uuid, name },
        };
        cancel_if(scoped(host, || events_interaction_v0::use_item_on_block(&context)))
    }

    fn living_entity_use_item_finish(&self, host: &mut dyn Host, event: &mut LivingEntityUseItemFinishEvent<'_>) {
        if event.entity.is_some() { return; }
        let Some(player) = host.acting_player() else { return };
        let used = read_stack(event.stack);
        let server_player = player_api::ServerPlayer { uuid: player.uuid, name: player.name.clone() };
        // What a per-stack use feeds is given before the callbacks run, as the game gives a consumable's food before the finish event.
        if let Some(food) = item_api_v1::stack_use_food(&used) {
            scoped(&mut *host, || server_player.add_food(food.nutrition, food.saturation));
        }
        let context = events_interaction_v0::UseItemFinishContext {
            item: event.stack.item.to_string(),
            player: server_player,
            stack: used,
            hand: match event.hand {
                horizon_client::mod_api::Hand::Main => events_interaction_v0::Hand::Main,
                horizon_client::mod_api::Hand::Off => events_interaction_v0::Hand::Off,
            },
            slot: event.result.as_ref().and_then(slot_of),
            replacement: std::cell::RefCell::new(None),
            dimension: player.dimension.to_string(),
            position: player.position,
            creative: player.game_mode == 1,
        };
        // A stack a callback put in the hand replaces what the hand holds; a count of 0 empties it.
        if let Some(replacement) = scoped(host, || events_interaction_v0::use_item_finish(&context)) {
            let mut stack = event.result.clone().unwrap_or_else(|| ItemStack::new(&replacement.item.to_string(), replacement.count.max(1)));
            write_slot(&mut stack, &replacement);
            *event.result = if replacement.count == 0 { None } else { Some(stack) };
        }
    }

    fn break_speed(&self, host: &dyn Host, event: &mut BreakSpeedEvent) {
        let Some(level) = host.level(host.dimension()) else { return };
        let Some(block) = host.level_block_state(level, event.pos).and_then(|text| BlockState::parse(&text).ok()) else { return };
        let acting = host.acting_player();
        let held = acting.as_ref().and_then(|player| host.held_stack(player.uuid)).map(|stack| stack_data_of(host, &stack));
        let player = acting.as_ref().map(|player| {
            // Entity.isEyeInFluid(water): the block at the eyes holds water (a water block or a waterlogged one).
            let eyes = [player.position[0].floor() as i32, (player.position[1] + f64::from(player.eye_height)).floor() as i32, player.position[2].floor() as i32];
            let eyes_in_water = host.level_block_state(level, eyes).is_some_and(|state| state.starts_with("minecraft:water") || state.contains("waterlogged=true"));
            events_interaction_v0::DiggingPlayer { on_ground: player.on_ground, eyes_in_water }
        });
        // The light at the block being dug and the temperature of the biome it stands in, for the variables that read them.
        // The event names no face, so the light is the dug block's own, not the block on the side that was hit.
        let (block_light, sky_light) = host.light(event.pos);
        let (rain, thunder) = host.weather();
        let facts = events_interaction_v0::DiggingFacts {
            block_light: Some(block_light), sky_light: Some(sky_light),
            sky_darken: entity_events_v1::sky_darken(rain, thunder, host.day_time()),
            biome_temperature: host.biome(event.pos).as_deref().and_then(entity_events_v1::biome_climate).map(|(base, _)| entity_events_v1::temperature_at(base, event.pos[1])),
        };
        // the callbacks may ask whether the block is in a tag: a read-only world over the host for the length of the call
        let mut tags = ReadOnlyTags(host);
        event.speed = world_api_v1::with_world(&mut tags, || events_interaction_v0::break_speed(event.pos, block, event.original, event.speed, held, player, facts));
    }

    fn block_clone_item(&self, host: &dyn Host, event: &mut BlockCloneItemEvent<'_>) -> Option<ItemStack> {
        let level = host.level(host.dimension())?;
        let state = BlockState::parse(&host.level_block_state(level, event.pos)?).ok()?;
        let picked = block_api_v1::pick(event.block, &block_api_v1::PickContext { state, pos: event.pos })?;
        // data components of the picked stack are not carried over yet
        Some(ItemStack::new(&picked.item.to_string(), picked.count))
    }

    fn right_click_block(&self, _host: &mut dyn Host, event: &mut RightClickBlockEvent) -> Flow {
        cancel_if(events_interaction_v0::use_block(event.pos, event.face))
    }

    fn attack_entity(&self, host: &mut dyn Host, event: &mut AttackEntityEvent<'_>) -> Flow {
        let held = host.acting_player().and_then(|player| host.held_stack(player.uuid)).map(|stack| stack_data_of(host, &stack));
        entity_events_v1::attack_started(event.entity, held);
        cancel_if(events_interaction_v0::attack_entity(event.entity, event.kind, event.pos))
    }

    fn left_click_block(&self, host: &mut dyn Host, event: &mut LeftClickBlockEvent) -> Flow {
        // The break hook is not told the face; the dig's start is, and a break belongs to the dig the player last started.
        if let Some(player) = host.acting_player() {
            let face = match event.face {
                [0, -1, 0] => Some(block_api_v1::Direction::Down),
                [0, 1, 0] => Some(block_api_v1::Direction::Up),
                [0, 0, -1] => Some(block_api_v1::Direction::North),
                [0, 0, 1] => Some(block_api_v1::Direction::South),
                [-1, 0, 0] => Some(block_api_v1::Direction::West),
                [1, 0, 0] => Some(block_api_v1::Direction::East),
                _ => None,
            };
            if let (Some(face), Ok(mut last)) = (face, LAST_DIG_FACE.lock()) {
                last.insert(player.uuid, face);
            }
        }
        Flow::Continue
    }

    fn destroy_block(&self, host: &mut dyn Host, event: &mut DestroyBlockEvent) -> Flow {
        if !events_interaction_v0::block_break_heard() { return Flow::Continue; }
        let context = break_context(host, event.pos);
        if let Some(player) = &context.player { flush_block_breaks(host, player.uuid); }
        let allowed = scoped(host, || events_interaction_v0::before_block_break(&context));
        let actions = context.actions.take();
        if let (Some(player), false) = (&context.player, actions.is_empty()) { apply_player_actions(host, player.uuid, actions); }
        // The extra blocks an allowed break's callbacks asked for are broken by the level seams (see the schedule drain in `hooks()`).
        if allowed {
            for extra in context.extra_breaks.take() {
                events_interaction_v0::queue_extra_break(&context.dimension, extra);
            }
        }
        // `AFTER` hears an allowed break at the player's next tick (see `flush_block_breaks`) with what was read here.
        let pending = match (allowed, context.player) {
            (true, Some(player)) if events_interaction_v0::after_block_break_heard() => Some(PendingBreak { pos: event.pos, dimension: context.dimension.clone(), state: context.state, player, face: context.face, yaw: context.yaw, pitch: context.pitch }),
            _ => None,
        };
        if let (Some(pending), Ok(mut waiting)) = (pending, PENDING_BREAKS.lock()) {
            if waiting.len() >= 64 { waiting.remove(0); }
            waiting.push(pending);
        }
        cancel_if(!allowed)
    }

    fn tooltip(&self, _host: &dyn Host, event: &mut TooltipEvent<'_>) {
        let lines = event.lines.iter().map(|(text, color)| {
            item_api_v1::TooltipText::existing(text.plain(), text.component(), color.swap_bytes(), text.clone())
        }).collect();
        // Keep the host list intact until callbacks finish, including if one unwinds.
        let edited = item_api_v1::item_tooltip(read_stack(event.stack), lines);
        event.lines.clear();
        for line in edited {
            if let Some(text) = line.native::<Text>() {
                event.lines.push((text.clone(), line.color().swap_bytes()));
            } else if let Some(simple) = line.simple() {
                let text = if simple.translate {
                    Text::literal(crate::pure::i18n_api::I18n::translate(&simple.text))
                        .with_component(serde_json::json!({ "translate": simple.text }))
                } else { Text::literal(simple.text.clone()) };
                // Public colors are RRGGBBAA; Horizon's HUD stores AABBGGRR.
                event.lines.push((text, line.color().swap_bytes()));
            }
        }
    }

    fn loot_table_load(&self, event: &mut LootTableLoadEvent<'_>) {
        let edited = loot_api_v2::modify(event.id, event.table.clone());
        *event.table = edited;
    }

    fn add_server_reload_listeners(&self, _host: &mut dyn Host, event: &mut AddServerReloadListenersEvent<'_>) {
        let listeners = resource_loader_v0::json_reload_listeners(resource_loader_v0::ResourceType::ServerData)
            .expect("JSON reload registry must remain available");
        // Refuse a cross-mod ID collision before adding any of this batch.
        for (id, _) in &listeners {
            assert!(!event.listeners.iter().any(|l| l.id == id.to_string()), "duplicate native JSON listener {id}");
        }
        event.listeners.extend(listeners.into_iter().map(|(id,directory)| NativeReloadListener::json(&id.to_string(),&directory)));
    }

    fn server_reload_listener_apply(&self, _host: &mut dyn Host, event: &mut ServerReloadListenerApplyEvent<'_>) {
        let registries = (|| {
            let Some(snapshot)=horizon_client::registries::pluto_reload_registry::current() else {
                return Ok::<_,String>(std::collections::BTreeMap::new());
            };
            let (registries,errors)=resource_loader_v0::decode_reload_registries(snapshot.iter()
                .filter(|r|r.id!="minecraft:fluid")
                .map(|r|(r.id.as_str(),r.entries.as_slice(),r.tags.as_slice())))?;
            for error in errors { eprintln!("PlutoLib: reload registry unavailable: {error}"); }
            Ok(registries)
        })();
        let registries=match registries {
            Ok(registries)=>registries,
            Err(error)=>{
                eprintln!("PlutoLib: invalid reload registry snapshot: {error}; registry lookups unavailable");
                // Still deliver valid JSON with no registry context: consumers
                // can disable stale state instead of never hearing this reload.
                std::collections::BTreeMap::new()
            }
        };
        let mut resources=std::collections::BTreeMap::new();
        if let Some(folders)=horizon_client::registries::pluto_reload_registry::current_resources() {
            for (directory,records) in folders.iter() {
                match resource_loader_v0::decode_json_stacks(records.iter().map(|r|(r.id.as_str(),r.layers.as_slice()))) {
                    Ok(stacks)=>{ resources.insert(directory.clone(),stacks); },
                    Err(error)=>eprintln!("PlutoLib: JSON resource stacks unavailable for {directory}: {error}"),
                }
            }
        }
        let items=match reload_item_prototypes(&registries) {
            Ok(snapshot)=>snapshot,
            Err(error)=>{ eprintln!("PlutoLib: item prototypes unavailable: {error}"); None }
        };
        let native=horizon_client::registries::pluto_reload_registry::current();
        let native_fluids=native.as_ref().and_then(|records|records.iter().find(|r|r.id=="minecraft:fluid"));
        let base=native_fluids.map(|r|r.entries.as_slice());
        let fluids=match crate::pure::fluid_api_v1::FluidRegistrySnapshot::capture(base) {
            Ok(snapshot)=>snapshot,
            Err(error)=>{ eprintln!("PlutoLib: fluid registry unavailable: {error}"); None }
        };
        // The fluid lookup joins the native fluids with the ones PlutoLib defines and carries the loaded fluid tags,
        // so `reload_registry("minecraft:fluid")` answers for mod fluids too.
        let mut registries=registries;
        if let Some(record)=native_fluids {
            match crate::pure::fluid_api_v1::reload_lookup(&record.entries,&record.tags) {
                Ok(lookup)=>{ registries.insert(Identifier::parse("minecraft:fluid").expect("valid registry ID"),lookup); },
                Err(error)=>eprintln!("PlutoLib: fluid reload registry unavailable: {error}"),
            }
        }
        let result=crate::pure::fluid_api_v1::with_fluid_registry(fluids,||
            crate::pure::item_registry::with_item_registry(items,||
                resource_loader_v0::with_reload_folders(registries,resources,||
                    resource_loader_v0::apply_json_reload_listener(resource_loader_v0::ResourceType::ServerData,
                        &event.listener.id,&event.listener.folder,event.files))));
        if let Err(error) = result {
            eprintln!("PlutoLib: JSON reload listener {} failed: {error}", event.listener.id);
        }
    }

    fn server_data_loaded(&self, _host: &mut dyn Host, _event: &mut ServerDataLoadedEvent) {
        resource_loader_v0::server_data_reloaded();
    }

    fn client_start(&self, host: &mut dyn Host, _event: &mut ClientStartEvent) {
        refresh_client_resources(host);
        resource_loader_v0::client_resources_reloaded();
    }

    fn client_resources_reloaded(&self, host: &mut dyn Host, _event: &mut ClientResourcesReloadedEvent) {
        refresh_client_resources(host);
        resource_loader_v0::client_resources_reloaded();
    }

    fn living_death(&self, host: &mut dyn Host, event: &mut LivingDeathEvent<'_>) -> Flow {
        let death = LivingDamage {
            entity: event.entity as i32,
            kind: event.kind.to_owned(),
            damage_type: event.damage_type.to_owned(),
            attacker: if event.attacker == 0 { -1 } else { event.attacker as i32 },
            direct: -1,
            amount: 0.0,
        };
        scoped(host, || entity_events_v1::after_death(&death));
        Flow::Continue
    }

    fn living_damage_pre(&self, event: &mut LivingDamagePreEvent<'_>) {
        let damage = LivingDamage {
            entity: event.entity,
            kind: event.kind.to_owned(),
            damage_type: event.damage_type.to_owned(),
            attacker: event.causing,
            direct: event.direct,
            amount: event.amount,
        };
        event.amount = entity_events_v1::modify_damage(&damage);
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
        if !entity_events_v1::allow_damage(&damage) { return Flow::Cancel; }
        // The hurt stage, as the damage arrives and before armor and enchantment protection take their share: what the
        // callbacks leave of the amount is what the game goes on with.
        event.amount = entity_events_v1::modify_hurt(&damage);
        Flow::Continue
    }

    fn living_incoming_damage_v2(&self, host: &mut dyn Host, event: &mut horizon_client::mod_api::LivingIncomingDamageV2Event<'_>) -> Flow {
        if COUNTER_HURT.with(|flag| flag.get()) { return Flow::Continue; }
        // A player's weapon hit on a mob: what both were doing is remembered for the damage event that follows in the same call.
        let state_of = |host: &mut dyn Host, id: i32| -> Option<entity_events_v1::EntityState> {
            let entity = host.entity(u32::try_from(id).ok()?)?;
            let effects = host.entity_effects(entity).unwrap_or_default();
            let mut state = entity_events_v1::EntityState {
                kind: host.entity_kind(entity)?,
                sprinting: host.entity_is_sprinting(entity).unwrap_or(false),
                on_ground: host.entity_on_ground(entity).unwrap_or(true),
                in_water: host.entity_is_in_water(entity).unwrap_or(false),
                fire_ticks: host.entity_fire_ticks(entity).unwrap_or(0),
                fire_immune: host.entity_fire_immune(entity).unwrap_or(false),
                effects: effects.iter().map(|(id, _, _)| id.clone()).collect(),
                // The mob's effects come as (id, ticks left, amplifier).
                effect_levels: effects.into_iter().map(|(id, _, amplifier)| (id, amplifier)).collect(),
                ..entity_events_v1::EntityState::default()
            };
            // Health, the maximum and the armor worn are read from the mob's saved data.
            if let Some(data) = host.entity_nbt(entity) {
                let saved = entity_events_v1::saved_facts(&compound_from_tag(&data));
                state.health = saved.health;
                state.max_health = saved.max_health;
                state.armor_coverage = saved.armor_coverage;
                state.equipment = saved.equipment;
            }
            let eyes = host.entity_eye_position(entity);
            if let Some(block) = host.entity_block_position(entity) { read_surroundings(host, &mut state, block, eyes); }
            // The height of the feet, not of the block they stand in.
            if let Some(position) = host.entity_position(entity) { state.y = Some(position[1]); }
            Some(state)
        };
        // The attacking player is not an entity the game's handles reach in a single-player world, so their state comes from the
        // server's list of players: what that list says (sprinting, on the ground, health) and their effects.
        let player_state = |host: &mut dyn Host| -> Option<entity_events_v1::EntityState> {
            // The player list names the attacker by entity id (0 for the owner of a single-player world, who is the acting player).
            let player = host.acting_player().filter(|acting| acting.entity_id == event.causing || acting.entity_id == 0)
                .or_else(|| host.server_players().into_iter().find(|player| player.entity_id == event.causing))?;
            Some(player_entity_state(host, &player))
        };
        let attacker_state = state_of(&mut *host, event.causing).or_else(|| player_state(&mut *host));
        let target_state = state_of(&mut *host, event.entity);
        entity_events_v1::hit_states(event.entity, attacker_state.clone(), target_state.clone());
        // The mods may put status effects on the mob; the game applies them right after this hook (pluto-mob-effect-apply).
        let hit = entity_events_v1::WeaponHit {
            entity: event.entity, kind: event.kind.to_owned(), attacker: event.causing, weapon: event.weapon.map(read_stack),
            attacker_state, target_state, armor: entity_events_v1::armor_worn_by(event.causing), effects: std::cell::RefCell::new(Vec::new()), actions: std::cell::RefCell::new(Vec::new()),
        };
        if let Ok(entity) = u32::try_from(event.entity) {
            for grant in entity_events_v1::weapon_hit(&hit) {
                horizon_client::entry::pluto_mob_effect_access::push(entity, &grant.effect, grant.duration, grant.amplifier);
            }
        }
        // The actions the mods queued (fire, freezing, healing, knockback) are done before the hit's damage lands.
        let actions = entity_events_v1::take_weapon_actions(&hit);
        // The effects a tool puts on its own holder go to the attacking player, named as the player state above names them.
        let (own_effects, actions): (Vec<_>, Vec<_>) = actions.into_iter().partition(|action| matches!(action, entity_events_v1::EntityAction::AddEffect { target: entity_events_v1::ActionTarget::Attacker, .. }));
        if !own_effects.is_empty() {
            let attacker = host.acting_player().filter(|acting| acting.entity_id == event.causing || acting.entity_id == 0).map(|acting| acting.uuid)
                .or_else(|| host.server_players().into_iter().find(|player| player.entity_id == event.causing).map(|player| player.uuid));
            if let Some(uuid) = attacker {
                for action in own_effects {
                    if let entity_events_v1::EntityAction::AddEffect { effect, amplifier, duration, .. } = action { give_effect(host, uuid, effect, amplifier, duration); }
                }
            }
        }
        // Edits of the attacking player's own stacks are done now, to the player named as the player state names them.
        let (own_edits, actions): (Vec<_>, Vec<_>) = actions.into_iter().partition(|action| matches!(action, entity_events_v1::EntityAction::EditStackData { target: entity_events_v1::ActionTarget::Attacker, .. }));
        if !own_edits.is_empty() {
            if let Some(selected) = host.acting_player().filter(|acting| acting.entity_id == event.causing || acting.entity_id == 0).map(|acting| acting.selected_slot) {
                for action in own_edits {
                    if let entity_events_v1::EntityAction::EditStackData { slot, item, edits, .. } = action { edit_player_stack(host, selected, slot, &item, &edits); }
                }
            }
        }
        if !actions.is_empty() {
            if let Some(dimension) = host.acting_player().map(|acting| acting.dimension) {
                apply_entity_actions(host, &dimension, event.causing, event.entity, actions);
            }
        }
        Flow::Continue
    }

    fn living_knockback_v2(&self, event: &mut horizon_client::mod_api::LivingKnockbackV2Event<'_>) -> Flow {
        let push = entity_events_v1::KnockbackModification {
            entity: event.entity,
            kind: event.kind.to_owned(),
            attacker: event.attacker,
            attacker_main: event.attacker_main.map(read_stack),
            attacker_off: event.attacker_off.map(read_stack),
            strength: std::cell::Cell::new(event.strength),
            x: std::cell::Cell::new(event.x),
            z: std::cell::Cell::new(event.z),
            cancel: std::cell::Cell::new(false),
        };
        match entity_events_v1::modify_knockback(&push) {
            Some((strength, x, z)) => { event.strength = strength; event.x = x; event.z = z; Flow::Continue }
            None => Flow::Cancel,
        }
    }

    fn item_damaged(&self, host: &mut dyn Host, event: &mut horizon_client::mod_api::ItemDamagedEvent<'_>) -> Flow {
        // Player 0 is the local player, whose wear the game's own hook skips and PlutoLib's `pluto-item-wear` injection reports.
        let (player, name) = if event.player == 0 {
            match host.acting_player().map(|acting| acting.uuid).or_else(|| sole_player().map(|(uuid, _)| uuid)) {
                Some(uuid) => (uuid, sole_player().filter(|(known, _)| *known == uuid).map(|(_, name)| name).unwrap_or_default()),
                None => (0, String::new()),
            }
        } else { (event.player, event.name.to_owned()) };
        // The host names the item but not the stack; the player's hand and armor are searched for one of that item.
        let mut found = host.held_stack(player).filter(|held| held.item.to_string() == event.item);
        let from_hand = found.is_some();
        let mut worn = from_hand.then_some(entity_events_v1::CarriedSlot::Equipment(entity_events_v1::StackSlot::MainHand));
        for (stack_slot, slot) in [(entity_events_v1::StackSlot::Feet, horizon_client::mod_api::EquipmentSlot::Feet), (entity_events_v1::StackSlot::Legs, horizon_client::mod_api::EquipmentSlot::Legs), (entity_events_v1::StackSlot::Chest, horizon_client::mod_api::EquipmentSlot::Chest), (entity_events_v1::StackSlot::Head, horizon_client::mod_api::EquipmentSlot::Head)] {
            if found.is_some() { break; }
            found = host.player_equipment(player, slot).filter(|stack| stack.item.to_string() == event.item);
            if found.is_some() { worn = Some(entity_events_v1::CarriedSlot::Equipment(stack_slot)); }
        }
        let stack = found.as_ref().map(read_stack);
        let slot = found.as_ref().and_then(slot_of);
        let before = stack.clone();
        // What the player was doing, for a callback that asks (their health, surroundings and effects).
        let holder = host.acting_player().filter(|acting| acting.uuid == player)
            .or_else(|| host.server_players().into_iter().find(|listed| listed.uuid == player))
            .map(|listed| player_entity_state(host, &listed));
        let wear = entity_events_v1::ItemDamage { player, name, item: event.item.to_owned(), stack, slot, amount: std::cell::Cell::new(event.amount), cancel: std::cell::Cell::new(false), replacement: std::cell::RefCell::new(None), holder, worn, carried: carried_stacks(host, player), edits: std::cell::RefCell::new(Vec::new()) };
        match entity_events_v1::modify_item_damage(&wear) {
            Some((amount, replacement, edits)) => {
                event.amount = amount;
                // The edits a callback made to the holder's other stacks (a durability share's shield on the stack that took it).
                apply_carried_edits(host, player, edits);
                // A replacement goes to the main-inventory stack that carried the old data (a held tool, usually); the game then
                // wears it by 0. A worn armor piece is replaced by the game's own wear step, which takes the stack from
                // `pluto_item_wear_access` and wears it by 0; otherwise the wear stays in `amount`.
                if let (Some(replacement), Some(before)) = (replacement, before) {
                    let mut written = false;
                    if from_hand {
                        for index in 0..36 {
                            let Some(target) = host.carried_stack(index) else { continue };
                            if &*target.item == event.item && target.count > 0 && read_stack(target).custom_data == before.custom_data {
                                write_slot(target, &replacement);
                                event.amount = 0;
                                written = true;
                                break;
                            }
                        }
                    } else if let Some(base) = found.as_ref() {
                        let mut native = base.clone();
                        write_slot(&mut native, &replacement);
                        horizon_client::entry::pluto_item_wear_access::set(native);
                    }
                    let _ = written;
                }
                Flow::Continue
            }
            None => Flow::Cancel,
        }
    }
}


thread_local! {
    /// Set while the player's counter damage (thorns) is being dealt: the hurt reaches the weapon-hit hook, which must not treat it as a hit
    /// with the player's weapon.
    static COUNTER_HURT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The players logged in, kept so an action the host does not name a player for can still be given one.
static LOGGED_IN: std::sync::Mutex<Vec<(u128, String)>> = std::sync::Mutex::new(Vec::new());

fn remember_player(uuid: u128, name: &str, joined: bool) {
    let Ok(mut players) = LOGGED_IN.lock() else { return };
    players.retain(|(known, _)| *known != uuid);
    if joined { players.push((uuid, name.to_string())); }
}

/// The only player logged in (a single-player world's), None when there are none or several.
fn sole_player() -> Option<(u128, String)> {
    let players = LOGGED_IN.lock().ok()?;
    if players.len() == 1 { players.first().cloned() } else { None }
}
