//! TinkerWorld's sticky predicates and CongealedSlimeBlock's landing/contact rules (SlimeKnights, MIT).
//!
//! The slime wood sets, slime grass, flora, vines and enderbark roots register nothing here: what of their behaviour is
//! portable is their vanilla templates' (the importer's declarations, `world_w1_blocks`), and their random-tick growth —
//! grass spreading and bonemeal growth, the saplings and fungi growing, the vines advancing their stage, the propagule
//! leaves placing a propagule — is registered in world_growth.rs.
//! Below them, the geodes and heads: CrystalClusterBlock placement and survival, the budding blocks' piston rule,
//! SkullBlock rotation and the wall heads' pick. Budding growth is registered in
//! world_growth.rs, which restates the budding piston rule below as registering replaces.
//!
//! The crystal chime (CrystalBlock/CrystalClusterBlock.onProjectileHit) is registered on PlutoLib's projectile
//! impact event, the heads' models on its model-layer and skull registries (SkullModelHelper's layers and
//! WorldClientEvents' layer definitions and SKIN_BY_TYPE textures), and WorldEvents.creeperKill on its
//! living-drops callback, whose native side owns the creeper's charged state and spent-skull counter.
//! Not expressible yet: the head item placing the wall variant (the content schema's wall blocks are signs) and
//! WorldEvents.livingVisibility (the port's visibility event is heard only for armor carrying custom data,
//! which a plain head item does not, and the game already counts worn heads among its own factors; the
//! enderman stare is modifier-driven, see mob_disguise_hook).
use pluto_lib::{AfterFallContext, BlockBehavior, BlockBehaviors, BlockState, BlockView, CubeDefinition, Direction, EntityInsideMotionContext, EntityModelLayerRegistry, Identifier, ItemStack, LayerDefinition, LivingEntityDrops, LivingEntityDropsCallback, PartDefinition, PartPose, PathNodeType, PistonBehavior, Placement, ProjectileEvents, ProjectileImpact, SkullModelAnimation, SkullModelRegistry, StackData};
use pluto_lib::{ItemTooltipCallback, TooltipLine};

pub(crate) fn register() {
    static TOOLTIPS: std::sync::Once = std::sync::Once::new();
    TOOLTIPS.call_once(|| {
        ItemTooltipCallback::EVENT.register(|tip| {
            if let Some(name @ ("cobalt_shard" | "steel_shard" | "knightmetal_shard")) = tip.item.strip_prefix("tconstruct:") {
                tip.added.borrow_mut().push(TooltipLine::translatable(format!("item.tconstruct.{name}.tooltip")));
            }
        });
    });
    let id = |name| Identifier::new("tconstruct", name).unwrap();
    for name in ["sky_slime", "ichor_slime", "ender_slime"] {
        BlockBehaviors::register(BlockBehavior::new(id(name))
            .is_slime_block(|_| true)
            .is_sticky_block(|_| true)
            .can_stick_to(move |state, other| match name {
                "ichor_slime" => state.block != other.block,
                "ender_slime" => state.block == other.block,
                _ => true,
            }));
    }
    for name in ["earth_congealed_slime", "sky_congealed_slime", "ichor_congealed_slime", "ender_congealed_slime"] {
        BlockBehaviors::register(BlockBehavior::new(id(name))
            .fall_on(|c| c.damage = 0.0)
            .update_entity_after_fall_on(after_fall)
            .entity_inside_motion(inside)
            .path_node_type(|_| Some(PathNodeType::StickyHoney)));
    }
    for name in CLUSTERS {
        BlockBehaviors::register(BlockBehavior::new(id(name))
            .state_for_placement(|c| Placement::State(cluster_placement(c.state.clone(), c.face, c.in_water)))
            .can_survive(|c| cluster_survives(&c.state, c.pos, c.view))
            .piston_behavior(PistonBehavior::Destroy));
    }
    // BuddingCrystalBlock is destroyed by pistons too; world_growth.rs registers its growth and restates
    // this rule, as registering a behaviour replaces the previous one.
    for name in BUDDING {
        BlockBehaviors::register(BlockBehavior::new(id(name)).piston_behavior(PistonBehavior::Destroy));
    }
    for name in HEADS {
        BlockBehaviors::register(BlockBehavior::new(id(name))
            .state_for_placement(|c| Placement::State(head_placement(c.state.clone(), c.yaw)))
            .piston_behavior(PistonBehavior::Destroy));
    }
    // The wall heads carry no item of their own: picking one gives the standing head, the item the original's
    // StandingAndWallBlockItem is registered for.
    for name in WALL_HEADS {
        BlockBehaviors::register(BlockBehavior::new(id(name)).pick_stack(|c| wall_head_pick(&c.state)));
    }
    // CrystalBlock/CrystalClusterBlock.onProjectileHit: the chime (chime_on_impact).
    ProjectileEvents::IMPACT.register(chime_on_impact);
    // WorldEvents.creeperKill: a charged creeper's kill drops the matching head (creeper_head_drop).
    if let Err(error) = LivingEntityDropsCallback::register(id("head_drops"), creeper_head_drop) {
        eprintln!("Tinkers: head drop callback not registered: {error}");
    }
    // The heads' skull models: a model layer and a skull definition per TinkerHeadType, under
    // SkullModelHelper.HEAD_LAYERS' ids (tconstruct:<type>_head#main).
    for (model, item_path) in HEAD_MODELS.into_iter().zip(HEAD_ITEM_PATHS) {
        let layer_id = model.layer_id();
        let layer = model.layer_definition();
        if let Err(error) = EntityModelLayerRegistry::register_model_layer(&layer_id, move || layer.clone()) {
            eprintln!("Tinkers: {} skull layer not registered: {error}", model.name);
        }
        if let Err(error) = SkullModelRegistry::register(id(item_path), &layer_id, Identifier::parse(model.texture).unwrap(), model.animation) {
            eprintln!("Tinkers: {} skull model not registered: {error}", model.name);
        }
    }
}

/// The CrystalClusterBlocks: each geode's cluster and three buds, then the three metal clusters (TinkerWorld).
pub(crate) const CLUSTERS: [&str; 19] = [
    "earth_slime_crystal_cluster", "small_earth_slime_crystal_bud", "medium_earth_slime_crystal_bud", "large_earth_slime_crystal_bud",
    "sky_slime_crystal_cluster", "small_sky_slime_crystal_bud", "medium_sky_slime_crystal_bud", "large_sky_slime_crystal_bud",
    "ichor_slime_crystal_cluster", "small_ichor_slime_crystal_bud", "medium_ichor_slime_crystal_bud", "large_ichor_slime_crystal_bud",
    "ender_slime_crystal_cluster", "small_ender_slime_crystal_bud", "medium_ender_slime_crystal_bud", "large_ender_slime_crystal_bud",
    "steel_cluster", "cobalt_cluster", "knightmetal_cluster",
];

/// The BuddingCrystalBlocks (world_growth.rs registers the growth on their declared random tick).
pub(crate) const BUDDING: [&str; 4] = ["budding_earth_slime_crystal", "budding_sky_slime_crystal", "budding_ichor_slime_crystal", "budding_ender_slime_crystal"];

/// TinkerHeadType's standing heads, in its order.
pub(crate) const HEADS: [&str; 9] = ["blaze_head", "enderman_head", "stray_head", "husk_head", "drowned_head", "spider_head", "cave_spider_head", "piglin_brute_head", "zombified_piglin_head"];

/// The wall variants of HEADS, in the same order.
pub(crate) const WALL_HEADS: [&str; 9] = ["blaze_wall_head", "enderman_wall_head", "stray_wall_head", "husk_wall_head", "drowned_wall_head", "spider_wall_head", "cave_spider_wall_head", "piglin_brute_wall_head", "zombified_piglin_wall_head"];

/// AmethystClusterBlock.getStateForPlacement: the cluster points along the clicked face and keeps the water it
/// was placed in.
pub(crate) fn cluster_placement(state: BlockState, face: Direction, in_water: bool) -> BlockState {
    state.with("facing", face.name()).with("waterlogged", if in_water { "true" } else { "false" })
}

/// AmethystClusterBlock.canSurvive: the block behind the point must support it. The original asks for centre
/// support specifically; the host's view answers the one sturdy-face question it has.
pub(crate) fn cluster_survives(state: &BlockState, pos: [i32; 3], view: &dyn BlockView) -> bool {
    let Some(facing) = state.property("facing").and_then(Direction::parse) else { return false };
    view.is_face_sturdy(facing.opposite().relative(pos), facing)
}

/// RotationSegment.convertToSegment: a segment every 22.5 degrees, centred on the segment's angle.
pub(crate) fn rotation_segment(yaw: f32) -> i32 { ((f64::from(yaw) / 22.5) + 0.5).floor() as i32 & 15 }

/// SkullBlock.getStateForPlacement: the floor head takes the placer's yaw segment.
pub(crate) fn head_placement(state: BlockState, yaw: f32) -> BlockState {
    let segment = rotation_segment(yaw).to_string();
    state.with("rotation", &segment)
}

/// A wall head picks as its standing head.
pub(crate) fn wall_head_pick(state: &BlockState) -> Option<ItemStack> {
    let head = state.block.path().strip_suffix("_wall_head")?;
    Some(ItemStack::new(Identifier::new("tconstruct", &format!("{head}_head")).ok()?, 1))
}

/// The sounds of CrystalBlock.onProjectileHit and CrystalClusterBlock.onProjectileHit, as (hit sound, chime)
/// for a block path; None for a block that does not chime. The geode blocks' sound types are built from the
/// vanilla amethyst events (Sounds.makeCrystalSound / makeClusterSounds), so the crystal blocks and budding
/// blocks hit as an amethyst block and the clusters and buds as an amethyst cluster.
pub(crate) fn chime_sounds(block: &str) -> Option<(&'static str, &'static str)> {
    const BLOCK_HIT: &str = "minecraft:block.amethyst_block.hit";
    const CLUSTER_HIT: &str = "minecraft:block.amethyst_cluster.hit";
    const GEODE_CHIMES: [(&str, &str); 4] = [
        ("earth", "tconstruct:block.earth_crystal.chime"),
        ("sky", "tconstruct:block.sky_crystal.chime"),
        ("ichor", "tconstruct:block.ichor_crystal.chime"),
        ("ender", "tconstruct:block.ender_crystal.chime"),
    ];
    // The metal clusters are CrystalClusterBlocks chiming with a geode's chime (TinkerWorld): steel with the
    // sky chime over SoundType.METAL, cobalt and knightmetal with the ichor and ender chimes over
    // SoundType.NETHERITE_BLOCK.
    match block {
        "steel_cluster" => return Some(("minecraft:block.metal.hit", GEODE_CHIMES[1].1)),
        "cobalt_cluster" => return Some(("minecraft:block.netherite_block.hit", GEODE_CHIMES[2].1)),
        "knightmetal_cluster" => return Some(("minecraft:block.netherite_block.hit", GEODE_CHIMES[3].1)),
        _ => {}
    }
    for (geode, chime) in GEODE_CHIMES {
        if block.strip_suffix("_slime_crystal_block") == Some(geode)
            || block.strip_prefix("budding_").and_then(|rest| rest.strip_suffix("_slime_crystal")) == Some(geode)
        {
            return Some((BLOCK_HIT, chime));
        }
        if block.strip_suffix("_slime_crystal_cluster") == Some(geode)
            || ["small_", "medium_", "large_"].iter().any(|size| block.strip_prefix(size).and_then(|rest| rest.strip_suffix("_slime_crystal_bud")) == Some(geode))
        {
            return Some((CLUSTER_HIT, chime));
        }
    }
    None
}

/// onProjectileHit's pitch, drawn per sound: 0.5 + random.nextFloat() * 1.2.
pub(crate) fn chime_pitch(roll: f32) -> f32 { 0.5 + roll * 1.2 }

/// CrystalBlock/CrystalClusterBlock.onProjectileHit, on the projectile impact event: the block the projectile
/// met plays its sound type's hit sound and then its chime at the block's centre, each at its own pitch drawn
/// from the level's stream. The impact event is a server event, as the original checks the server side; the
/// hit itself is never cancelled.
pub(crate) fn chime_on_impact(hit: &ProjectileImpact) -> bool {
    if hit.entity.is_none() {
        let world = hit.world();
        if let Some(state) = world.get_block_state(hit.block) {
            if let Some((hit_sound, chime)) = chime_sounds(state.block.path()) {
                let at = [f64::from(hit.block[0]) + 0.5, f64::from(hit.block[1]) + 0.5, f64::from(hit.block[2]) + 0.5];
                let pitch = || chime_pitch(world.random_float().unwrap_or_else(crate::tool_damage_hook::random));
                world.play_sound(hit_sound, at, 1.0, pitch());
                world.play_sound(chime, at, 1.0, pitch());
            }
        }
    }
    true
}

/// TinkerHeadType in enum order: the serialized name and the entity type whose head it is.
pub(crate) const HEAD_TYPES: [(&str, &str); 9] = [
    ("blaze", "minecraft:blaze"),
    ("enderman", "minecraft:enderman"),
    ("stray", "minecraft:stray"),
    ("husk", "minecraft:husk"),
    ("drowned", "minecraft:drowned"),
    ("spider", "minecraft:spider"),
    ("cave_spider", "minecraft:cave_spider"),
    ("piglin_brute", "minecraft:piglin_brute"),
    ("zombified_piglin", "minecraft:zombified_piglin"),
];

/// TinkerHeadType.fromEntityType: the serialized name of the head a mob of this type drops; None for a type
/// with no Tinkers head.
pub(crate) fn head_for_entity(entity_kind: &str) -> Option<&'static str> {
    HEAD_TYPES.iter().find(|(_, kind)| *kind == entity_kind).map(|(name, _)| *name)
}

/// The head WorldEvents.creeperKill drops for a kill: the dying mob's head when the damage owner is a
/// creeper; None otherwise.
pub(crate) fn head_drop_for(killer_type: Option<&str>, entity_kind: &str) -> Option<&'static str> {
    if killer_type != Some("minecraft:creeper") { return None; }
    head_for_entity(entity_kind)
}

/// WorldEvents.creeperKill: when a charged creeper kills a mob whose type has a head, and the config allows
/// that head, the creeper's one skull is spent on the head item. The native side owns the creeper's charged
/// state and spent-skull counter (Creeper.canDropMobsSkull / increaseDroppedSkulls): eligibility is read,
/// claimed, and only then is the drop appended, in the original's order; a native refusal or error leaves
/// the drops untouched.
pub(crate) fn creeper_head_drop(drops: &mut dyn LivingEntityDrops) -> Result<(), String> {
    let info = drops.info();
    let Some(head) = head_drop_for(info.killer_type.as_deref(), &info.entity.kind) else { return Ok(()); };
    if !crate::config::head_drop(head) { return Ok(()); }
    if drops.can_drop_head()? && drops.claim_head_drop()? {
        drops.append_stack(StackData { item: format!("tconstruct:{head}_head"), count: 1, ..Default::default() })?;
    }
    Ok(())
}

/// How a head's model layer is built (WorldClientEvents.registerRenderers' layer definitions).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum HeadGeometry {
    /// SkullModel.createMobHeadLayer (blaze) / SkullModelHelper.createHeadLayer: the one head cube, reading
    /// its texture at `head`, on a texture of `texture_size`.
    Plain { head: [i32; 2], texture_size: [u32; 2] },
    /// SkullModelHelper.createHeadHatLayer(0, 16, w, h): the head cube plus a hat cube grown by 0.25,
    /// reading its texture at (0, 16).
    Hat { texture_size: [u32; 2] },
    /// PiglinHeadModel.createHeadModel on a 64x64 texture.
    Piglin,
}

/// One TinkerHeadType's skull model: its layer, texture and animation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct HeadModel {
    /// The TinkerHeadType serialized name; the head's item is `tconstruct:<name>_head`.
    pub name: &'static str,
    /// SkullBlockRenderer.SKIN_BY_TYPE's texture, as the atlas sprite id the skull registry takes (the
    /// original's `textures/entity/...png` path without the `textures/` prefix or the `.png`).
    pub texture: &'static str,
    pub animation: SkullModelAnimation,
    pub geometry: HeadGeometry,
}

impl HeadModel {
    /// The head item's path.
    pub(crate) fn item(&self) -> String { format!("{}_head", self.name) }

    /// SkullModelHelper.HEAD_LAYERS' location for this type, as a registry id.
    pub(crate) fn layer_id(&self) -> String { format!("tconstruct:{}_head#main", self.name) }

    /// The layer definition WorldClientEvents registers for this type.
    pub(crate) fn layer_definition(&self) -> LayerDefinition {
        let head_cube = |texture_offset: [i32; 2]| CubeDefinition::new([-4.0, -8.0, -4.0], [8.0, 8.0, 8.0], texture_offset);
        match self.geometry {
            HeadGeometry::Plain { head, texture_size } => LayerDefinition::create(
                vec![("head".into(), PartDefinition { cubes: vec![head_cube(head)], ..Default::default() })],
                texture_size[0], texture_size[1]),
            HeadGeometry::Hat { texture_size } => {
                let hat = CubeDefinition { deformation: [0.25; 3], ..CubeDefinition::new([-4.0, -8.0, -4.0], [8.0, 8.0, 8.0], [0, 16]) };
                let head = PartDefinition {
                    cubes: vec![head_cube([0, 0])],
                    children: vec![("hat".into(), PartDefinition { cubes: vec![hat], ..Default::default() })],
                    ..Default::default()
                };
                LayerDefinition::create(vec![("head".into(), head)], texture_size[0], texture_size[1])
            }
            HeadGeometry::Piglin => piglin_head_layer(),
        }
    }
}

/// The head items' paths, in HEAD_MODELS (TinkerHeadType) order; each is its model's name plus
/// "_head", kept as literals because the registry's identifiers borrow their path.
pub(crate) const HEAD_ITEM_PATHS: [&str; 9] = [
    "blaze_head", "enderman_head", "stray_head", "husk_head", "drowned_head",
    "spider_head", "cave_spider_head", "piglin_brute_head", "zombified_piglin_head",
];

/// The nine heads' skull models, in TinkerHeadType order.
pub(crate) const HEAD_MODELS: [HeadModel; 9] = [
    HeadModel { name: "blaze", texture: "minecraft:entity/blaze", animation: SkullModelAnimation::None, geometry: HeadGeometry::Plain { head: [0, 0], texture_size: [64, 64] } },
    HeadModel { name: "enderman", texture: "tconstruct:entity/skull/enderman", animation: SkullModelAnimation::None, geometry: HeadGeometry::Plain { head: [0, 0], texture_size: [32, 16] } },
    HeadModel { name: "stray", texture: "tconstruct:entity/skull/stray", animation: SkullModelAnimation::None, geometry: HeadGeometry::Hat { texture_size: [32, 32] } },
    HeadModel { name: "husk", texture: "minecraft:entity/zombie/husk", animation: SkullModelAnimation::None, geometry: HeadGeometry::Plain { head: [0, 0], texture_size: [64, 64] } },
    HeadModel { name: "drowned", texture: "tconstruct:entity/skull/drowned", animation: SkullModelAnimation::None, geometry: HeadGeometry::Hat { texture_size: [32, 32] } },
    HeadModel { name: "spider", texture: "minecraft:entity/spider/spider", animation: SkullModelAnimation::None, geometry: HeadGeometry::Plain { head: [32, 4], texture_size: [64, 32] } },
    HeadModel { name: "cave_spider", texture: "minecraft:entity/spider/cave_spider", animation: SkullModelAnimation::None, geometry: HeadGeometry::Plain { head: [32, 4], texture_size: [64, 32] } },
    HeadModel { name: "piglin_brute", texture: "minecraft:entity/piglin/piglin_brute", animation: SkullModelAnimation::Piglin, geometry: HeadGeometry::Piglin },
    HeadModel { name: "zombified_piglin", texture: "minecraft:entity/piglin/zombified_piglin", animation: SkullModelAnimation::Piglin, geometry: HeadGeometry::Piglin },
];

/// PiglinHeadModel.createHeadModel: the 10-wide head, its snout and nostrils, and the two ears, the right
/// one mirrored, each ear posed as the original poses it.
fn piglin_head_layer() -> LayerDefinition {
    let cube = |origin: [f32; 3], dimensions: [f32; 3], texture_offset: [i32; 2], mirror: bool| CubeDefinition { origin, dimensions, texture_offset, deformation: [0.0; 3], mirror };
    let ear = |name: &str, origin: [f32; 3], mirror: bool, translation: [f32; 3], z_rotation: f32| {
        (name.to_string(), PartDefinition {
            pose: PartPose { translation, rotation: [0.0, 0.0, z_rotation], scale: [1.0; 3] },
            cubes: vec![cube(origin, [1.0, 5.0, 4.0], [51, 1], mirror)],
            children: Vec::new(),
        })
    };
    let head = PartDefinition {
        cubes: vec![
            cube([-5.0, -8.0, -4.0], [10.0, 8.0, 8.0], [0, 0], false),
            cube([-2.0, -4.0, -5.0], [4.0, 3.0, 1.0], [31, 1], false),
            cube([2.0, -2.0, -5.0], [1.0, 2.0, 1.0], [2, 4], false),
            cube([-3.0, -2.0, -5.0], [1.0, 2.0, 1.0], [2, 4], true),
        ],
        children: vec![
            ear("left_ear", [0.0, 0.0, -2.0], false, [4.5, -6.0, 0.0], -0.3491),
            ear("right_ear", [-1.0, 0.0, -2.0], true, [-4.5, -6.0, 0.0], 0.3491),
        ],
        ..Default::default()
    };
    LayerDefinition::create(vec![("head".into(), head)], 64, 64)
}

fn after_fall(c: &mut AfterFallContext) {
    if !c.suppressing_bounce && (c.living || c.item) && c.velocity[1] < 0.0 {
        c.velocity[1] *= if c.living { -1.0 } else { -0.8 };
        c.fall_distance = 0.0;
        if c.item { c.on_ground = false; }
    }
}

fn inside(c: &mut EntityInsideMotionContext) {
    let y = c.position[1] - f64::from(c.pos[1]);
    if !c.server || c.suppressing_bounce || !(-0.0625..0.9365).contains(&y) { return; }
    let x = c.position[0] - (f64::from(c.pos[0]) + 0.5);
    let z = c.position[2] - (f64::from(c.pos[2]) + 0.5);
    let (axis, direction, across) = if x.abs() > z.abs() { (0, x, z) } else { (2, z, x) };
    if across.abs() >= 0.495 { return; }
    let speed = c.position.iter().zip(c.previous_position).map(|(now, old)| (now - old).powi(2)).sum::<f64>().sqrt() * 0.95;
    // Java Math.signum(0) is zero; Rust's signum(0) is one.
    c.velocity[axis] = if direction == 0.0 { 0.0 } else { speed.copysign(direction) };
    c.sync_velocity = true;
    if speed > 0.1 { c.step_sound = Some([1.0, 1.0]); }
}
