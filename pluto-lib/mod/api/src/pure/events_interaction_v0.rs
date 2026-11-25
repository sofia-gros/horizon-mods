//! Counterpart of Fabric's `fabric-events-interaction-v0`: callbacks for what a player does with items, blocks and entities.
//!
//! Callback data is reduced and owned (Fabric passes player, world, hand and hit result). Results follow Fabric: the first
//! callback that does not return `ActionResult::Pass` decides; any other result stops the game's own handling.

use std::cell::{Cell, RefCell};

use super::base::{compare_nbt, ActionResult, Event, Identifier, Nbt, NbtCompound};
use super::block_api_v1::Direction;
use super::block_entity_api_v1::ItemSlot;
use super::entity_events_v1::EntityAction;
use super::player_api::{ServerPlayer, StackData};
use super::resource_loader_v0::RegistryLookup;
use super::world_api_v1::BlockState;

/// A player used an item.
#[derive(Clone, Debug, PartialEq)]
pub struct UseItemContext {
    /// The item id, such as `tconstruct:flint_pickaxe`.
    pub item: String,
    pub count: u32,
    /// The stack as used, with its damage and custom data: a tool's stats and modifiers live there, which a
    /// use that depends on them (a fishing module's cast) reads. The host's use event names no hand; the hand
    /// holding this stack is the one whose carried stack equals it.
    pub stack: StackData,
    /// The dimension the use is in, such as `minecraft:overworld`: the acting player's, or the host's own
    /// when the event named no acting player.
    pub dimension: String,
    /// Where the player using the item stands (their feet), when the host named the acting player; None
    /// when it did not, and a use that scales with the caster's height (a dwarven cast) then reads without
    /// a caster, as it did before the context carried one.
    pub position: Option<[f64; 3]>,
    /// Who used it; None when the host named no acting player and the world has not exactly one player.
    pub player: Option<ServerPlayer>,
}

/// A player used (right-clicked) a block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UseBlockContext {
    pub pos: [i32; 3],
    /// The face that was clicked, as a unit offset.
    pub face: [i32; 3],
}

/// A player used an item on a block: the held item, the block it was used on and who did it.
#[derive(Clone, Debug, PartialEq)]
pub struct UseItemOnBlockContext {
    /// The item id, such as `tconstruct:earth_slime_grass_seeds`.
    pub item: String,
    /// How many of the item are held.
    pub count: u32,
    /// The block it was used on.
    pub pos: [i32; 3],
    /// The face that was clicked, as a unit offset.
    pub face: [i32; 3],
    /// The dimension, such as `minecraft:overworld`.
    pub dimension: String,
    /// The player; its methods work inside the callback.
    pub player: ServerPlayer,
    /// Whether the player is in creative mode (nothing is used up).
    pub creative: bool,
    /// Whether the player is sneaking as they click.
    pub sneaking: bool,
    /// The stack that was used, with its damage, max damage and custom data (a tool a mod made keeps its state there).
    pub held: Option<super::player_api::StackData>,
}

/// A player's use finished without the game consuming it: the hand is empty, the player is
/// not a spectator, and nothing the game did took the click — for a click that named an
/// entity, the entity's own interaction did not take it either. The takeover a chestplate
/// module runs on (Tinkers' sleeves) answers here, with the aim the throw uses and the
/// crouch state the module branches on. `consumed` is false at both seams that report a
/// use today (the hand's use, and the entity interaction); a later seam may report a
/// consumed use the same way.
#[derive(Clone, Debug, PartialEq)]
pub struct AfterUseContext {
    /// The player; its methods work inside the callback.
    pub player: ServerPlayer,
    /// The dimension, such as `minecraft:overworld`.
    pub dimension: String,
    /// Where the player's eyes are, in block coordinates.
    pub eye: [f64; 3],
    /// Where the player looks, a unit vector.
    pub look: [f64; 3],
    /// Whether the player is sneaking.
    pub crouching: bool,
    /// Whether the player is in creative mode (nothing is used up).
    pub creative: bool,
    /// Whether the game's own handling consumed the use.
    pub consumed: bool,
}

/// How fast the player digs a block (NeoForge: `PlayerEvent.BreakSpeed`).
#[derive(Debug)]
pub struct BreakSpeedContext {
    /// The block being dug.
    pub pos: [i32; 3],
    /// Its state.
    pub block: BlockState,
    /// The speed as vanilla works it out (the tool, efficiency, haste and fatigue, the water); it does not change.
    pub original: f32,
    /// The speed now, which callbacks may change with `set`; every callback is called in turn, each seeing the last one's.
    pub speed: Cell<f32>,
    /// The stack the player digs with, with its damage, max damage and custom data; None for an empty hand or when the game does not
    /// say who digs.
    pub held: Option<super::player_api::StackData>,
    /// The digging player's state when the game says who digs (Player.onGround, isEyeInFluid(water)).
    pub player: Option<DiggingPlayer>,
    /// The block light and the sky light at the block being dug, each 0 to 15 as the level reports them, and how much of the
    /// sky light the time of day and the weather take off; the lights are None when the game did not report them.
    pub block_light: Option<u8>,
    pub sky_light: Option<u8>,
    pub sky_darken: u8,
    /// The temperature of the biome at the block being dug, as `Biome.getTemperature` reports it there; None for a biome
    /// PlutoLib has no temperature for.
    pub biome_temperature: Option<f32>,
    /// The block's explosion resistance (`BlockState.getExplosionResistance`), for the block property
    /// variables that read it; None when the game did not report it, and a variable over it takes its fallback.
    pub blast_resistance: Option<f32>,
    /// Whether the block needs the correct tool for drops (`BlockState.requiresCorrectToolForDrops`), for the
    /// `mantle:requires_tool` block predicate; None when the game did not report it.
    pub requires_tool: Option<bool>,
    /// The build range of the level the dig is in, [min, max) as `Level.getMinBuildHeight` and
    /// `Level.getMaxBuildHeight` bound it, for the modules that scale with depth under the level's
    /// top (dwarven); None when the game did not report it.
    pub build_height: Option<[i32; 2]>,
}

/// What the world reports about a dig besides the block itself: its light and the temperature of its biome.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DiggingFacts {
    pub block_light: Option<u8>,
    pub sky_light: Option<u8>,
    pub sky_darken: u8,
    pub biome_temperature: Option<f32>,
    pub blast_resistance: Option<f32>,
    pub requires_tool: Option<bool>,
    pub build_height: Option<[i32; 2]>,
}

/// What a break-speed callback may read about the player who digs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DiggingPlayer {
    /// On the ground, as the player's last accepted move said.
    pub on_ground: bool,
    /// The player's eyes are inside water (their block, at eye height, holds water).
    pub eyes_in_water: bool,
    /// The effects on the player, as (effect id, amplifier): the amplifier is the level less one.
    /// Empty when the game did not report them.
    pub effect_levels: Vec<(String, i32)>,
    /// Whether rain falls on the player: it is raining, the sky is open above them and the biome
    /// rains rather than snows or stays dry.
    pub rain_exposed: bool,
}

/// The hand a player used an item in (Minecraft: `InteractionHand`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hand {
    /// The main hand.
    Main,
    /// The off hand.
    Off,
}

/// A player finished using an item over time (ate or drank it).
#[derive(Clone, Debug, PartialEq)]
pub struct UseItemFinishContext {
    /// The item id, such as `tconstruct:cheese_ingot`.
    pub item: String,
    /// The player; its methods work inside the callback.
    pub player: ServerPlayer,
    /// The stack that was used, with its damage, max damage and custom data, as it was when the use started.
    pub stack: StackData,
    /// The hand the stack was used in.
    pub hand: Hand,
    /// What that hand holds now, with all its components, after the game's own food and remainder were given; None when the hand is empty
    /// (the last of a stack was eaten). To change it, put the changed copy in `replacement`.
    pub slot: Option<ItemSlot>,
    /// A whole new stack for the hand, which a callback may set (clone `slot`, edit the clone, store it here); the later callbacks see the
    /// one before. A count of 0 empties the hand. Left None, the hand keeps what it holds.
    pub replacement: RefCell<Option<ItemSlot>>,
    /// The dimension the player is in, such as `minecraft:overworld`.
    pub dimension: String,
    /// Where the player is.
    pub position: [f64; 3],
    /// Whether the player is in creative mode (nothing is used up).
    pub creative: bool,
}

/// A player attacked an entity.
#[derive(Clone, Debug, PartialEq)]
pub struct AttackEntityContext {
    pub entity: i32,
    pub kind: String,
    pub pos: [f64; 3],
}

/// A player used an item on an entity (right click, not an attack): Fabric's `UseEntityCallback`, NeoForge's
/// `PlayerInteractEvent.EntityInteract`. Heard once for each hand the game tries, the main hand first; the first callback that
/// does not return `Pass` takes the click and neither the entity's own interaction nor the item's runs.
#[derive(Clone, Debug, PartialEq)]
pub struct UseItemOnEntityContext {
    /// The entity's id.
    pub entity: i32,
    /// The entity's type, such as `minecraft:sheep`.
    pub kind: String,
    /// The dimension the interaction happens in, such as `minecraft:overworld`; empty when the game does not name it.
    pub dimension: String,
    /// The hand the item is used with.
    pub hand: Hand,
    /// The stack in that hand, with its damage, max damage, custom data and enchantments; None for an empty hand.
    pub held: Option<StackData>,
    /// The player; None when the game names none.
    pub player: Option<ServerPlayer>,
    /// The dimension the player is in, such as `minecraft:overworld`; empty when the game does not name it.
    pub player_dimension: String,
    /// Whether the player is in creative mode (nothing is used up).
    pub creative: bool,
    /// What to do to the player (`ActionTarget::Attacker`), which callbacks add to; the game does it right after the callbacks ran.
    pub actions: RefCell<Vec<EntityAction>>,
}

/// A player interacted with an entity, at the game's own generic claim point: the interaction the host offers before
/// any hand-specific handling, so the context names no hand (NeoForge's `PlayerInteractEvent.EntityInteractSpecific`
/// without a target point; the hand-by-hand offer is [`UseItemOnEntityContext`]). The first callback that does not
/// return `Pass` takes the interaction and the game does not handle it. Heard on a server.
#[derive(Clone, Debug, PartialEq)]
pub struct InteractEntityContext {
    /// The entity's id.
    pub entity: i32,
    /// The entity's type, such as `minecraft:armor_stand`.
    pub kind: String,
    /// The stack the interaction is made with, with its damage, max damage, custom data and enchantments; None
    /// when the game names none.
    pub stack: Option<StackData>,
    /// The player; None when the game names none.
    pub player: Option<ServerPlayer>,
}

/// A player's fishing rod changed state (Bukkit: `PlayerFishEvent`): the cast, a bite, and the reel-ins. The state is the game's
/// own number: [`FISHING_CAST`] when the bobber goes out, [`FISHING_BITE`] when a fish bites, and 2 to 4 for the reel-ins (a fish
/// caught, an entity hooked, a plain reel-in). A callback may cancel the state with `cancel` — a cancelled cast throws no bobber,
/// a cancelled reel-in catches nothing. The rod's own catch loot is the game's; a tool that fishes with its own hook entity (a
/// combat rod) starts from the item-use events instead and reads this event for the vanilla rod it replaces.
#[derive(Clone, Debug, PartialEq)]
pub struct FishingContext {
    /// The player fishing.
    pub player: ServerPlayer,
    /// The state's number (see the type's documentation).
    pub state: i32,
    /// The hand holding the rod.
    pub hand: Hand,
    /// The rod stack in that hand, with its damage, max damage, custom data and enchantments; None when the game did not read it.
    pub rod: Option<StackData>,
    /// Set to true to cancel the state.
    pub cancel: Cell<bool>,
    /// What to do to the player (`ActionTarget::Attacker`), which callbacks add to; the game does it right after the callbacks ran.
    pub actions: RefCell<Vec<EntityAction>>,
}

/// `FishingContext.state`: the bobber was cast.
pub const FISHING_CAST: i32 = 0;
/// `FishingContext.state`: a fish bit.
pub const FISHING_BITE: i32 = 1;

/// A player breaking a block: what `PlayerBlockBreakEvents` reports (Fabric: world, player, pos, state; the block entity is not given, see
/// `AFTER`).
#[derive(Clone, Debug, PartialEq)]
pub struct BlockBreakContext {
    /// The block's position.
    pub pos: [i32; 3],
    /// The dimension it is in, such as `minecraft:overworld`; empty when the game does not name it.
    pub dimension: String,
    /// The block as it was before the break; None when the game cannot read it.
    pub state: Option<BlockState>,
    /// The player who breaks it; None when the game names none.
    pub player: Option<ServerPlayer>,
    /// The stack in the player's main hand: before the break for `BEFORE` and `CANCELED`, at the tick that reports it (with the wear the break caused) for `AFTER`.
    /// None for an empty hand.
    pub held: Option<StackData>,
    /// The equipment the breaker wears or holds in the off hand, as (slot, stack): `feet`, `legs`, `chest`, `head`, then
    /// `offhand`, as of their last tick, only the pieces carrying custom data. Empty when the game names no player or the
    /// player is not known. A module that lends the breaking stack its enchantments (a harvest enchantment on armor) reads it.
    pub equipment: Vec<(String, StackData)>,
    /// What to do to the player (`ActionTarget::Attacker`), which callbacks add to. The game does it right after the callbacks ran: for
    /// `BEFORE` even when a callback vetoes the break, for `AFTER` once the break is done. `EditStackData` acts on the main hand as it is then.
    pub actions: RefCell<Vec<EntityAction>>,
    /// The face of the block the player hit, as the dig that broke it started on that face; None when the game did not report one
    /// (the break did not start from a dig the host saw). An area-of-effect tool reads its break plane from this face.
    pub face: Option<Direction>,
    /// Where the player looks, in degrees (`yRot`, `xRot`) as their last move told the server; 0 when the game names no player.
    pub yaw: f32,
    pub pitch: f32,
    /// Whether the breaker is in creative mode (game mode 1), as the break started; false when the game names no player.
    pub creative: bool,
    /// Whether the breaker may build (vanilla `Player.mayBuild` for their game type: survival and creative may, adventure and
    /// spectator may not), as the break started; true when the game names no player, as an unnamed breaker is not an
    /// adventure-mode one.
    pub may_build: bool,
    /// The off hand stack's adventure-mode place tag (`minecraft:can_place_on`), one text entry per block predicate, in the
    /// form the original's own tag strings take: a block id, a `#tag`, or an id with a state suffix
    /// (`minecraft:oak_stairs[facing=east]`). Empty when the off hand carries no such tag (the common case) or the game did
    /// not read it. A predicate carrying a block entity NBT test is not reported here: the text form cannot carry it —
    /// see `offhand_place_predicates` for the structured predicates, which keep it.
    pub offhand_place_on: Vec<String>,
    /// The off hand stack's adventure-mode place tag (`minecraft:can_place_on`) as structured block predicates — the
    /// blocks, state tests and block-entity NBT tests the tag carries, in the form the game's own matcher reads them
    /// (see [`AdventurePlacePredicate`]). Empty when the off hand carries no such tag. A predicate carrying a
    /// block-entity NBT test is kept here, unlike in `offhand_place_on`: the break reads the mined block's block
    /// entity with it (see `mined_block_entity`).
    pub offhand_place_predicates: Vec<AdventurePlacePredicate>,
    /// The mined block's block entity as the break started, as the game saved it (`BlockEntity.saveWithFullMetadata`);
    /// None when the block has no block entity, or the game did not read it. The NBT tests of
    /// `offhand_place_predicates` match against it.
    pub mined_block_entity: Option<NbtCompound>,
    /// More blocks this break takes with it, which `BEFORE` callbacks add to with [`BlockBreakContext::break_extra`]: the game
    /// breaks them through its own destroy path (their drops and loot are the game's, as for the block itself) at the level's next
    /// tick seam, a tick late at most. They do not fire break events of their own. Read by nobody in `AFTER` or `CANCELED`.
    pub extra_breaks: RefCell<Vec<[i32; 3]>>,
}

impl BlockBreakContext {
    /// A context with only a position, for a game that reports nothing else (and for callbacks that only read `pos`).
    pub fn at(pos: [i32; 3]) -> Self {
        Self { pos, dimension: String::new(), state: None, player: None, held: None, equipment: Vec::new(), actions: RefCell::new(Vec::new()), face: None, yaw: 0.0, pitch: 0.0, creative: false, may_build: true, offhand_place_on: Vec::new(), offhand_place_predicates: Vec::new(), mined_block_entity: None, extra_breaks: RefCell::new(Vec::new()) }
    }

    /// Takes the block at `pos` with this break (see `extra_breaks`): the block itself, a duplicate and anything past 64 extra
    /// blocks are refused.
    pub fn break_extra(&self, pos: [i32; 3]) {
        if pos == self.pos { return; }
        let mut extra = self.extra_breaks.borrow_mut();
        if extra.len() < 64 && !extra.contains(&pos) { extra.push(pos); }
    }
}

/// The experience a player's block break is about to drop (the experience half of NeoForge's
/// `BlockEvent.BreakEvent`): heard as the game samples the drop on the player's own break path,
/// before the game's silk-touch rule is applied to the sampled amount, and on the level's removal
/// path for a break that path works out: the break named its breaker there (see `BlockBreaker`),
/// so a guest's break is heard with the guest credited, not left unheard for naming no breaker.
#[derive(Clone, Debug)]
pub struct BlockExperienceContext {
    /// The block's position.
    pub pos: [i32; 3],
    /// The dimension it is in, such as `minecraft:overworld`; empty when the game does not name it.
    pub dimension: String,
    /// The player who breaks it; None when the game names none.
    pub player: Option<ServerPlayer>,
    /// The breaking player's entity id, for the reads keyed by it; 0 when the game names none.
    pub player_entity: i32,
    /// The stack in the player's main hand as the drop is sampled; None for an empty hand.
    pub held: Option<StackData>,
    /// The experience now, which callbacks may change with `set`; each callback sees the last one's.
    pub experience: Cell<u32>,
}

/// One block predicate of an adventure-mode place tag (`minecraft:can_place_on`), as the game's own
/// `BlockPredicate` reads it: the blocks it accepts, the state tests it narrows them with, and the
/// block-entity NBT test it may carry. Parsed from the tag's NBT by [`adventure_place_predicates`]
/// and matched by [`place_predicates_allow`], mirroring the game's `adventure_predicate_at`.
#[derive(Clone, Debug, PartialEq)]
pub struct AdventurePlacePredicate {
    /// The blocks this predicate accepts, as written: block ids or `#tag`s. `None` when the predicate
    /// names no blocks (any block passes the block test); `Some` of an empty list when it names blocks
    /// no entry of which can match (a malformed `blocks` value), so nothing passes.
    pub blocks: Option<Vec<String>>,
    /// The state tests, as (property, matcher) pairs. A property the mined block does not carry fails.
    pub state: Vec<(String, StateMatcher)>,
    /// The block-entity NBT test: a compound pattern, or SNBT text. `None` when the predicate carries
    /// no NBT test.
    pub nbt: Option<Nbt>,
}

/// One state test of a block predicate: an exact property value, or a min/max range over a numeric value.
#[derive(Clone, Debug, PartialEq)]
pub enum StateMatcher {
    /// The property must equal this value.
    Exact(String),
    /// The property's value, read as an integer, must lie between the bounds (each bound absent means
    /// open). A value or bound that is not an integer fails the test, as the game's range test does.
    Range { min: Option<String>, max: Option<String> },
    /// A test that never passes: a state entry the game cannot read (neither a value nor a range).
    Never,
}

impl AdventurePlacePredicate {
    /// A predicate that matches no block: what a malformed predicate becomes, so it is kept in the list
    /// (and fails) rather than dropped (which would let another predicate pass in its place).
    pub fn never() -> Self {
        Self { blocks: Some(Vec::new()), state: Vec::new(), nbt: None }
    }

    /// One predicate from its NBT: a bare string is the block id it accepts; a compound names its blocks
    /// (one id or a list), its state tests and its NBT test. Any other value matches nothing.
    pub fn from_nbt(value: &Nbt) -> Self {
        match value {
            Nbt::String(id) => Self { blocks: Some(vec![id.clone()]), state: Vec::new(), nbt: None },
            Nbt::Compound(predicate) => {
                let blocks = match predicate.get("blocks") {
                    None => None,
                    Some(Nbt::String(id)) => Some(vec![id.clone()]),
                    Some(Nbt::List(ids)) => {
                        Some(ids.iter().filter_map(Nbt::as_str).map(str::to_owned).collect())
                    }
                    Some(_) => Some(Vec::new()),
                };
                let state = match predicate.get("state") {
                    None => Vec::new(),
                    Some(Nbt::Compound(want)) => want
                        .iter()
                        .map(|(key, matcher)| {
                            let matcher = match matcher {
                                Nbt::String(value) => StateMatcher::Exact(value.clone()),
                                Nbt::Compound(range) => StateMatcher::Range {
                                    min: range.get("min").and_then(Nbt::as_str).map(str::to_owned),
                                    max: range.get("max").and_then(Nbt::as_str).map(str::to_owned),
                                },
                                _ => StateMatcher::Never,
                            };
                            (key.clone(), matcher)
                        })
                        .collect(),
                    Some(_) => return Self::never(),
                };
                Self { blocks, state, nbt: predicate.get("nbt").cloned() }
            }
            _ => Self::never(),
        }
    }

    /// Whether this predicate accepts the mined block: its blocks name it, its state tests hold, and its
    /// NBT test matches the mined block's block entity. `blocks` resolves `#tag`s; None means no snapshot,
    /// so a tag names nothing. Mirrors the game's `adventure_predicate_at`.
    pub fn allows(
        &self,
        mined: &BlockState,
        mined_block_entity: Option<&NbtCompound>,
        blocks: Option<&RegistryLookup>,
    ) -> bool {
        let block_ok = match &self.blocks {
            None => true,
            Some(ids) => ids.iter().any(|id| block_in_holder_set(id, &mined.block, blocks)),
        };
        let state_ok = self.state.iter().all(|(key, matcher)| {
            let Some(have) = mined.properties.get(key) else { return false };
            match matcher {
                StateMatcher::Exact(want) => have == want,
                StateMatcher::Range { min, max } => {
                    let in_range = |bound: &Option<String>, above: bool| {
                        bound.as_ref().is_none_or(|bound| {
                            match (have.parse::<i64>(), bound.parse::<i64>()) {
                                (Ok(have), Ok(bound)) if above => have >= bound,
                                (Ok(have), Ok(bound)) => have <= bound,
                                _ => false,
                            }
                        })
                    };
                    in_range(min, true) && in_range(max, false)
                }
                StateMatcher::Never => false,
            }
        });
        let nbt_ok = match &self.nbt {
            None => true,
            Some(Nbt::String(snbt)) => match Nbt::parse_snbt(snbt) {
                Ok(pattern) => {
                    let actual = mined_block_entity.map(|entity| Nbt::Compound(entity.clone()));
                    compare_nbt(&pattern, actual.as_ref())
                }
                Err(_) => false,
            },
            Some(pattern @ Nbt::Compound(_)) => {
                let actual = mined_block_entity.map(|entity| Nbt::Compound(entity.clone()));
                compare_nbt(pattern, actual.as_ref())
            }
            Some(_) => false,
        };
        block_ok && state_ok && nbt_ok
    }
}

/// Whether a holder-set entry (a block id or a `#tag`) names `block`, as the game's `block_in_holder_set`
/// does: the `#` is stripped first, then an id without a namespace reads as a `minecraft:` one; a tag resolves
/// through the block snapshot.
fn block_in_holder_set(entry: &str, block: &Identifier, blocks: Option<&RegistryLookup>) -> bool {
    match entry.strip_prefix('#') {
        Some(tag) => {
            let namespaced = if tag.contains(':') { tag.to_owned() } else { format!("minecraft:{tag}") };
            Identifier::parse(&namespaced).is_ok_and(|tag| {
                blocks
                    .and_then(|blocks| blocks.get_tag(&tag))
                    .is_some_and(|members| members.contains(block))
            })
        }
        None => {
            let namespaced = if entry.contains(':') { entry.to_owned() } else { format!("minecraft:{entry}") };
            Identifier::parse(&namespaced).is_ok_and(|id| id == *block)
        }
    }
}

/// The block predicates of a `minecraft:can_place_on` component, in the order the tag lists them: a list
/// of predicates, one compound predicate, or a bare block id (the shorthand the game's own text path reads
/// as an id). A predicate that is not a compound or a string matches nothing. The game's matcher reads
/// every key of such a value as absent and would call it a match for any block, which no real tag means —
/// a bare string is the only shorthand a tag writes.
pub fn adventure_place_predicates(component: &Nbt) -> Vec<AdventurePlacePredicate> {
    match component {
        Nbt::List(predicates) => predicates.iter().map(AdventurePlacePredicate::from_nbt).collect(),
        Nbt::Compound(_) | Nbt::String(_) => vec![AdventurePlacePredicate::from_nbt(component)],
        _ => Vec::new(),
    }
}

/// Whether any of the place tag's predicates accepts the mined block: the game's `AdventureModePredicate.test`
/// over the structured predicates, with the mined block's block entity for the NBT tests. An empty predicate
/// list accepts nothing.
pub fn place_predicates_allow(
    predicates: &[AdventurePlacePredicate],
    mined: &BlockState,
    mined_block_entity: Option<&NbtCompound>,
    blocks: Option<&RegistryLookup>,
) -> bool {
    predicates.iter().any(|predicate| predicate.allows(mined, mined_block_entity, blocks))
}

static USE_ITEM: Event<UseItemContext, ActionResult> = Event::first_not_pass();
static USE_BLOCK: Event<UseBlockContext, ActionResult> = Event::first_not_pass();
static USE_ITEM_ON_BLOCK: Event<UseItemOnBlockContext, ActionResult> = Event::first_not_pass();
static USE_ITEM_FINISH: Event<UseItemFinishContext, ()> = Event::void();
static BREAK_SPEED: Event<BreakSpeedContext, ()> = Event::void();
static ATTACK_ENTITY: Event<AttackEntityContext, ActionResult> = Event::first_not_pass();
static USE_ITEM_ON_ENTITY: Event<UseItemOnEntityContext, ActionResult> = Event::first_not_pass();
static INTERACT_ENTITY: Event<InteractEntityContext, ActionResult> = Event::first_not_pass();
static FISHING: Event<FishingContext, ()> = Event::void();
static BEFORE_BLOCK_BREAK: Event<BlockBreakContext, bool> = Event::allow_all();
static AFTER_BLOCK_BREAK: Event<BlockBreakContext, ()> = Event::void();
static BLOCK_BREAK_CANCELED: Event<BlockBreakContext, ()> = Event::void();
static AFTER_USE: Event<AfterUseContext, ActionResult> = Event::first_not_pass();
static BLOCK_EXPERIENCE: Event<BlockExperienceContext, ()> = Event::void();

/// Fabric: `UseItemCallback.EVENT`.
pub struct UseItemCallback;

impl UseItemCallback {
    pub const EVENT: &'static Event<UseItemContext, ActionResult> = &USE_ITEM;
}

/// Fabric: `UseBlockCallback.EVENT`.
pub struct UseBlockCallback;

impl UseBlockCallback {
    pub const EVENT: &'static Event<UseBlockContext, ActionResult> = &USE_BLOCK;
}

/// A player used an item on a block (like Fabric's `UseBlockCallback`, which also fires for an empty hand, with the held item and the player
/// in the context). Answer `Success` after doing the job, and the game does not handle it.
pub struct UseItemOnBlockCallback;

impl UseItemOnBlockCallback {
    pub const EVENT: &'static Event<UseItemOnBlockContext, ActionResult> = &USE_ITEM_ON_BLOCK;
}

/// A player finished eating or drinking an item (NeoForge: `LivingEntityUseItemEvent.Finish`). Called after the food and the item's own effects
/// have been given; only for a player on the server. The context names the stack, the hand and the player's place, and a callback may change
/// what the hand holds through `replacement`.
pub struct UseItemFinishCallback;

impl UseItemFinishCallback {
    pub const EVENT: &'static Event<UseItemFinishContext, ()> = &USE_ITEM_FINISH;
}

/// How fast a player digs a block (NeoForge: `PlayerEvent.BreakSpeed`). Not asked in creative mode or for an unbreakable block.
pub struct BreakSpeedCallback;

impl BreakSpeedCallback {
    pub const EVENT: &'static Event<BreakSpeedContext, ()> = &BREAK_SPEED;
}

/// Fabric: `AttackEntityCallback.EVENT`.
pub struct AttackEntityCallback;

impl AttackEntityCallback {
    pub const EVENT: &'static Event<AttackEntityContext, ActionResult> = &ATTACK_ENTITY;
}

/// Fabric: `UseEntityCallback.EVENT`.
pub struct UseEntityCallback;

impl UseEntityCallback {
    /// Fabric: `UseEntityCallback.EVENT`.
    pub const EVENT: &'static Event<UseItemOnEntityContext, ActionResult> = &USE_ITEM_ON_ENTITY;
}

/// The generic entity interaction (see [`InteractEntityContext`]): the game's own claim point, before its
/// hand-specific handling.
pub struct InteractEntityCallback;

impl InteractEntityCallback {
    pub const EVENT: &'static Event<InteractEntityContext, ActionResult> = &INTERACT_ENTITY;
}

/// A player's use finished unconsumed (see [`AfterUseContext`]): the chestplate takeover a
/// worn piece answers on. Answer `Success` after doing the job, and the game does nothing
/// more with the click.
pub struct AfterUseCallback;

impl AfterUseCallback {
    pub const EVENT: &'static Event<AfterUseContext, ActionResult> = &AFTER_USE;
}

/// A player's fishing rod changed state (see [`FishingContext`]).
pub struct PlayerFishEvents;

impl PlayerFishEvents {
    /// The event: every callback runs, and any one of them may cancel the state.
    pub const EVENT: &'static Event<FishingContext, ()> = &FISHING;
}

/// Fabric: `PlayerBlockBreakEvents`.
pub struct PlayerBlockBreakEvents;

impl PlayerBlockBreakEvents {
    /// Fabric: `BEFORE`: return `false` to cancel the break.
    pub const BEFORE: &'static Event<BlockBreakContext, bool> = &BEFORE_BLOCK_BREAK;
    /// Fabric: `AFTER`: a player's break went through and the block is gone. The break's own return tells the bridge it
    /// completed, so it is heard inside the break, before the removal is sent to the players around — the swap a break
    /// triggers never shows air. When the return is not heard (an error after the removal, or a weave that predates
    /// the completion seam) it is reconstructed at the player's next tick (or their next break), at most one tick late,
    /// and only when the block at `pos` is no longer the one `BEFORE` saw. `held` is the tool as of then; the block
    /// entity is not given. Creative breaks are heard too. Not heard for the breaks `BEFORE` does not hear (leaves
    /// decaying, a piston, `/setblock`).
    pub const AFTER: &'static Event<BlockBreakContext, ()> = &AFTER_BLOCK_BREAK;
    /// Fabric: `CANCELED`. Called once when `BEFORE` vetoes a break; the block is kept.
    pub const CANCELED: &'static Event<BlockBreakContext, ()> = &BLOCK_BREAK_CANCELED;
    /// NeoForge: `BlockEvent.BreakEvent` (its experience). The experience the break is about to drop, heard as the game
    /// samples it: change it with `experience.set`. Every player's break is heard, a guest's included.
    pub const EXPERIENCE: &'static Event<BlockExperienceContext, ()> = &BLOCK_EXPERIENCE;
}

/// True when the callbacks' result means the game must not handle the action itself.
fn handled(result: ActionResult) -> bool {
    result != ActionResult::Pass
}

pub(crate) fn use_item(context: &UseItemContext) -> bool {
    handled(USE_ITEM.invoke(context))
}

pub(crate) fn use_block(pos: [i32; 3], face: [i32; 3]) -> bool {
    handled(USE_BLOCK.invoke(&UseBlockContext { pos, face }))
}

pub(crate) fn use_item_on_block(context: &UseItemOnBlockContext) -> bool {
    handled(USE_ITEM_ON_BLOCK.invoke(context))
}

/// Calls the finish callbacks and returns the stack a callback put in the hand, if any.
pub(crate) fn use_item_finish(context: &UseItemFinishContext) -> Option<ItemSlot> {
    USE_ITEM_FINISH.invoke(context);
    context.replacement.borrow_mut().take()
}

/// The dig speed after every callback has had its say.
pub(crate) fn break_speed(pos: [i32; 3], block: BlockState, original: f32, speed: f32, held: Option<super::player_api::StackData>, player: Option<DiggingPlayer>, facts: DiggingFacts) -> f32 {
    let context = BreakSpeedContext { pos, block, original, speed: Cell::new(speed), held, player, block_light: facts.block_light, sky_light: facts.sky_light, sky_darken: facts.sky_darken, biome_temperature: facts.biome_temperature, blast_resistance: facts.blast_resistance, requires_tool: facts.requires_tool, build_height: facts.build_height };
    BREAK_SPEED.invoke(&context);
    context.speed.get()
}

pub(crate) fn attack_entity(entity: i32, kind: &str, pos: [f64; 3]) -> bool {
    handled(ATTACK_ENTITY.invoke(&AttackEntityContext { entity, kind: kind.to_owned(), pos }))
}

/// True when a callback took the click on the entity.
pub(crate) fn use_item_on_entity(context: &UseItemOnEntityContext) -> bool {
    handled(USE_ITEM_ON_ENTITY.invoke(context))
}

/// Whether anyone listens to entity clicks, so the game only reads the player and the stack when someone does.
pub(crate) fn use_item_on_entity_heard() -> bool {
    !USE_ITEM_ON_ENTITY.is_empty()
}

/// True when a callback took the generic interaction with the entity.
pub(crate) fn entity_interact(context: &InteractEntityContext) -> bool {
    handled(INTERACT_ENTITY.invoke(context))
}

/// True when a callback took the finished use.
pub(crate) fn after_use(context: &AfterUseContext) -> bool {
    handled(AFTER_USE.invoke(context))
}

/// Whether anyone listens to generic entity interactions, so the game only reads the player and the stack when
/// someone does.
pub(crate) fn entity_interact_heard() -> bool {
    !INTERACT_ENTITY.is_empty()
}

/// True when a callback cancelled the fishing state.
pub(crate) fn fishing(context: &FishingContext) -> bool {
    FISHING.invoke(context);
    context.cancel.get()
}

/// Whether anyone listens to fishing, so the game only reads the rod when someone does.
pub(crate) fn fishing_heard() -> bool {
    !FISHING.is_empty()
}

/// True when the break may go ahead.
pub(crate) fn before_block_break(context: &BlockBreakContext) -> bool {
    let allowed = BEFORE_BLOCK_BREAK.invoke(context);
    if !allowed { BLOCK_BREAK_CANCELED.invoke(context); }
    allowed
}

/// Calls the `AFTER` callbacks for a break that went through.
pub(crate) fn after_block_break(context: &BlockBreakContext) {
    AFTER_BLOCK_BREAK.invoke(context);
}

/// Whether anyone listens to block breaks, so the game only reads the player and the block when someone does.
pub(crate) fn block_break_heard() -> bool {
    !(BEFORE_BLOCK_BREAK.is_empty() && AFTER_BLOCK_BREAK.is_empty() && BLOCK_BREAK_CANCELED.is_empty())
}

/// Whether anyone listens to `AFTER`.
pub(crate) fn after_block_break_heard() -> bool {
    !AFTER_BLOCK_BREAK.is_empty()
}

/// Whether anyone listens to break experience, so the game only reads the breaker when someone does.
pub(crate) fn block_experience_heard() -> bool {
    !BLOCK_EXPERIENCE.is_empty()
}

/// The break's experience after every callback has had its say.
pub(crate) fn block_experience(context: &BlockExperienceContext) -> u32 {
    BLOCK_EXPERIENCE.invoke(context);
    context.experience.get()
}

/// The player who broke the block at a position, remembered from the break the game named them for:
/// `destroy_block` names every player's break (a LAN guest's included), while the experience the
/// break drops is sampled on paths that name no breaker — the player's own break path, which the
/// acting player names, and the level's removal path, which names none. The note is taken by the
/// first sampling site that runs for the break, so a break is bonused once at most; a break the
/// game never samples (a creative break, a break whose block drops no experience) leaves a note
/// that is dropped unread once it is this old.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BlockBreaker {
    pub uuid: u128,
    pub name: String,
    pub entity: i32,
    /// The stack in the breaker's main hand as the break started; None for an empty hand. The
    /// removal path reads the held tool from this snapshot, where no acting player is in hand.
    pub held: Option<StackData>,
    /// Whether the breaker was in creative mode (their break drops no experience).
    pub creative: bool,
}

/// How long a breaker's name waits for its break's experience to be sampled.
const BLOCK_BREAKER_LIFETIME: std::time::Duration = std::time::Duration::from_secs(5);

/// The most breakers remembered at once; past it the oldest break is forgotten rather than letting
/// a broken caller grow the map without bound.
const MAX_BLOCK_BREAKERS: usize = 4096;

static BLOCK_BREAKERS: std::sync::Mutex<std::collections::BTreeMap<(String, [i32; 3]), (BlockBreaker, std::time::Instant)>> = std::sync::Mutex::new(std::collections::BTreeMap::new());

/// Remembers `breaker` as the player who broke the block at `pos` in `dimension`.
pub(crate) fn note_block_breaker(dimension: &str, pos: [i32; 3], breaker: BlockBreaker) {
    let Ok(mut breakers) = BLOCK_BREAKERS.lock() else { return };
    let now = std::time::Instant::now();
    breakers.retain(|_, (_, at)| at.elapsed() < BLOCK_BREAKER_LIFETIME);
    if breakers.len() >= MAX_BLOCK_BREAKERS {
        if let Some(oldest) = breakers.iter().min_by_key(|(_, (_, at))| *at).map(|(key, _)| key.clone()) { breakers.remove(&oldest); }
    }
    breakers.insert((dimension.to_owned(), pos), (breaker, now));
}

/// The breaker `note_block_breaker` named for the block at `pos` in `dimension`, taken; a stale
/// one is dropped.
pub(crate) fn take_block_breaker(dimension: &str, pos: [i32; 3]) -> Option<BlockBreaker> {
    let Ok(mut breakers) = BLOCK_BREAKERS.lock() else { return None };
    match breakers.remove(&(dimension.to_owned(), pos)) {
        Some((breaker, at)) if at.elapsed() < BLOCK_BREAKER_LIFETIME => Some(breaker),
        _ => None,
    }
}

/// Forgets the breaker `note_block_breaker` named for the block at `pos` in `dimension` (a break
/// `BEFORE` vetoed, whose block stays and whose experience must not be credited to them later).
pub(crate) fn clear_block_breaker(dimension: &str, pos: [i32; 3]) {
    let Ok(mut breakers) = BLOCK_BREAKERS.lock() else { return };
    breakers.remove(&(dimension.to_owned(), pos));
}

/// An extra break a `BEFORE` callback asked for, waiting for a level seam that can break blocks: the host adapter's break
/// handler has no call that breaks a block, so it queues the request here and the level's own tick seams — the callbacks that
/// run with the level's block editor in hand — apply the queued breaks for their dimension (see
/// `apply_pending_extra_breaks`). A request is therefore honoured a tick late at most.
struct PendingExtraBreak {
    dimension: String,
    pos: [i32; 3],
}

static PENDING_EXTRA_BREAKS: std::sync::Mutex<Vec<PendingExtraBreak>> = std::sync::Mutex::new(Vec::new());

/// The most requests held at once; past it new requests are refused rather than letting a broken caller grow the queue
/// without bound.
const MAX_PENDING_EXTRA_BREAKS: usize = 4096;

/// Queues an extra break for the level seams to apply; a break of the same position in the same dimension already queued
/// counts as queued. False when the queue is full.
pub(crate) fn queue_extra_break(dimension: &str, pos: [i32; 3]) -> bool {
    let Ok(mut pending) = PENDING_EXTRA_BREAKS.lock() else { return false };
    if pending.iter().any(|queued| queued.dimension == dimension && queued.pos == pos) {
        return true;
    }
    if pending.len() >= MAX_PENDING_EXTRA_BREAKS {
        return false;
    }
    pending.push(PendingExtraBreak { dimension: dimension.to_owned(), pos });
    true
}

/// Applies every queued extra break for `dimension` through `apply` (the level seam's own block breaking, which answers
/// false for an unloaded position) and returns how many were broken. Requests the seam refuses are dropped, not kept.
/// Other dimensions' requests stay queued.
pub(crate) fn apply_pending_extra_breaks(dimension: &str, mut apply: impl FnMut([i32; 3]) -> bool) -> usize {
    let Ok(mut pending) = PENDING_EXTRA_BREAKS.lock() else { return 0 };
    let mut rest = Vec::new();
    let mut mine = Vec::new();
    for queued in std::mem::take(&mut *pending) {
        if queued.dimension == dimension { mine.push(queued.pos); } else { rest.push(queued); }
    }
    *pending = rest;
    drop(pending);
    mine.into_iter().filter(|pos| apply(*pos)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_non_pass_result_stops_the_games_handling_and_pass_does_not() {
        UseBlockCallback::EVENT.register(|c| if c.pos == [1, 2, 3] { ActionResult::Success } else { ActionResult::Pass });
        assert!(use_block([1, 2, 3], [0, 1, 0]));
        assert!(!use_block([9, 9, 9], [0, 1, 0]));
    }

    fn finish(item: &str, creative: bool) -> UseItemFinishContext {
        let mut data = StackData { item: item.into(), count: 1, damage: 3, ..Default::default() };
        data.custom_data.insert("bites".into(), super::super::base::Nbt::Int(2));
        let mut slot = ItemSlot::new(super::super::base::Identifier::parse(item).unwrap(), 1);
        slot.damage = 3;
        UseItemFinishContext {
            item: item.into(), player: ServerPlayer { uuid: 7, name: "Alex".into() }, stack: data, hand: Hand::Off, slot: Some(slot),
            replacement: RefCell::new(None), dimension: "minecraft:overworld".into(), position: [1.0, 64.0, 2.0], creative,
        }
    }

    #[test]
    fn a_finish_callback_sees_the_stack_and_hand_and_may_put_a_new_stack_in_the_hand() {
        UseItemFinishCallback::EVENT.register(|done| {
            if done.item != "test:snack_tool" || done.creative { return; }
            assert_eq!((done.hand, done.stack.damage, done.stack.custom_data.len()), (Hand::Off, 3, 1));
            // wear the hand's stack by two, as an edible tool's durability cost does
            let mut worn = done.slot.clone().expect("the hand still holds the tool");
            worn.damage += 2;
            *done.replacement.borrow_mut() = Some(worn);
        });
        UseItemFinishCallback::EVENT.register(|done| {
            // a later callback sees the earlier one's replacement
            if done.item == "test:snack_tool" && !done.creative {
                let seen = done.replacement.borrow().as_ref().map(|slot| slot.damage);
                assert_eq!(seen, Some(5));
            }
        });
        let used = finish("test:snack_tool", false);
        assert_eq!(use_item_finish(&used).map(|slot| slot.damage), Some(5));
        assert!(used.replacement.borrow().is_none(), "the replacement is handed over once");
        assert_eq!(use_item_finish(&finish("test:snack_tool", true)), None, "a callback that sets nothing leaves the hand alone");
        assert_eq!(use_item_finish(&finish("test:other", false)), None);
    }

    #[test]
    fn break_speed_callbacks_read_the_digging_player_and_change_the_speed() {
        BreakSpeedCallback::EVENT.register(|c| {
            if let Some(player) = &c.player {
                if !player.on_ground { c.speed.set(c.speed.get() * 5.0); }
                if player.eyes_in_water { c.speed.set(c.speed.get() + 1.0); }
            }
        });
        let block = BlockState { block: super::super::base::Identifier::parse("test:rock").unwrap(), properties: Default::default() };
        let state = |on_ground, eyes_in_water| Some(DiggingPlayer { on_ground, eyes_in_water, ..DiggingPlayer::default() });
        assert_eq!(break_speed([0, 0, 0], block.clone(), 2.0, 2.0, None, state(true, false), DiggingFacts::default()), 2.0);
        assert_eq!(break_speed([0, 0, 0], block.clone(), 2.0, 2.0, None, state(false, false), DiggingFacts::default()), 10.0);
        assert_eq!(break_speed([0, 0, 0], block.clone(), 2.0, 2.0, None, state(true, true), DiggingFacts::default()), 3.0);
        assert_eq!(break_speed([0, 0, 0], block.clone(), 2.0, 2.0, None, None, DiggingFacts::default()), 2.0, "no player state leaves the speed alone");
        // The facts of the dig reach the callbacks as they were reported.
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        BreakSpeedCallback::EVENT.register(move |c| if c.pos == [9, 9, 9] { log.lock().unwrap().push((c.block_light, c.sky_light, c.sky_darken, c.biome_temperature)); });
        let facts = DiggingFacts { block_light: Some(3), sky_light: Some(15), sky_darken: 4, biome_temperature: Some(0.8), blast_resistance: Some(6.0), requires_tool: Some(true), build_height: None };
        break_speed([9, 9, 9], block, 2.0, 2.0, None, None, facts);
        assert_eq!(*seen.lock().unwrap(), vec![(Some(3), Some(15), 4, Some(0.8))]);
    }

    #[test]
    fn a_veto_before_a_block_break_cancels_it() {
        let canceled = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = canceled.clone();
        PlayerBlockBreakEvents::CANCELED.register(move |c| seen.lock().unwrap().push(c.pos));
        PlayerBlockBreakEvents::BEFORE.register(|c| c.pos[1] >= 0);
        PlayerBlockBreakEvents::BEFORE.register(|c| c.pos[1] >= -1);
        assert!(before_block_break(&BlockBreakContext::at([0, 64, 0])));
        assert!(canceled.lock().unwrap().is_empty(), "allowed breaks are not canceled");
        assert!(!before_block_break(&BlockBreakContext::at([0, -5, 0])));
        assert_eq!(*canceled.lock().unwrap(), [[0, -5, 0]], "two vetoes still report one cancellation");
    }

    #[test]
    fn an_after_callback_sees_the_player_and_the_worn_tool_and_queues_actions_for_the_game() {
        use super::super::entity_events_v1::{ActionTarget, StackSlot, StackDataEdit};
        use super::super::base::Nbt;
        AFTER_BLOCK_BREAK.register(|broken| {
            if broken.dimension != "test:break_after" { return; }
            let held = broken.held.as_ref().expect("the player holds the tool");
            assert_eq!((held.damage, broken.player.as_ref().map(|p| p.uuid)), (4, Some(9)));
            assert_eq!(broken.state.as_ref().map(|s| s.block.to_string()), Some("test:rock".to_owned()));
            broken.actions.borrow_mut().push(EntityAction::EditStackData {
                target: ActionTarget::Attacker, slot: StackSlot::MainHand, item: held.item.clone(),
                edits: vec![StackDataEdit { path: vec!["mined".into()], value: Some(Nbt::Int(1)) }],
            });
        });
        let context = BlockBreakContext {
            pos: [1, 2, 3], dimension: "test:break_after".into(),
            state: Some(BlockState { block: super::super::base::Identifier::parse("test:rock").unwrap(), properties: Default::default() }),
            player: Some(ServerPlayer { uuid: 9, name: "Alex".into() }),
            held: Some(StackData { item: "test:pick".into(), count: 1, damage: 4, ..Default::default() }),
            equipment: Vec::new(),
            actions: RefCell::new(Vec::new()),
            face: Some(Direction::Up), yaw: 90.0, pitch: 0.0, creative: false, may_build: true, offhand_place_on: Vec::new(), offhand_place_predicates: Vec::new(), mined_block_entity: None, extra_breaks: RefCell::new(Vec::new()),
        };
        assert!(after_block_break_heard());
        after_block_break(&context);
        assert_eq!(context.actions.borrow().len(), 1);
        let quiet = BlockBreakContext { dimension: "test:elsewhere".into(), ..BlockBreakContext::at([0, 0, 0]) };
        after_block_break(&quiet);
        assert!(quiet.actions.borrow().is_empty(), "a callback that does not match queues nothing");
    }

    #[test]
    fn an_entity_click_is_taken_by_the_first_callback_that_does_not_pass() {
        UseEntityCallback::EVENT.register(|click| {
            if click.kind == "minecraft:sheep" && click.held.as_ref().is_some_and(|held| held.item == "test:kama") { ActionResult::Success } else { ActionResult::Pass }
        });
        let click = |kind: &str, item: &str| UseItemOnEntityContext {
            entity: 5, kind: kind.into(), dimension: String::new(), hand: Hand::Main,
            held: Some(StackData { item: item.into(), count: 1, ..Default::default() }),
            player: Some(ServerPlayer { uuid: 9, name: "Alex".into() }), player_dimension: "minecraft:overworld".into(), creative: false,
            actions: RefCell::new(Vec::new()),
        };
        assert!(use_item_on_entity_heard());
        assert!(use_item_on_entity(&click("minecraft:sheep", "test:kama")), "the kama takes the click on the sheep");
        assert!(!use_item_on_entity(&click("minecraft:sheep", "minecraft:shears")), "another tool leaves the click to the game");
        assert!(!use_item_on_entity(&click("minecraft:cow", "test:kama")));
    }

    #[test]
    fn an_entity_click_names_the_dimension_and_whether_the_player_is_creative() {
        UseEntityCallback::EVENT.register(|click| {
            if click.kind == "minecraft:creeper" {
                assert_eq!(click.dimension, "minecraft:overworld");
                assert_eq!(click.player_dimension, "minecraft:overworld");
                assert!(click.creative, "the click names a creative player");
            }
            ActionResult::Pass
        });
        let click = UseItemOnEntityContext {
            entity: 5, kind: "minecraft:creeper".into(), hand: Hand::Main,
            held: None, player: None, dimension: "minecraft:overworld".into(), player_dimension: "minecraft:overworld".into(), creative: true,
            actions: RefCell::new(Vec::new()),
        };
        assert!(!use_item_on_entity(&click));
    }

    #[test]
    fn a_finished_use_is_taken_by_the_first_callback_that_does_not_pass() {
        AfterUseCallback::EVENT.register(|used| {
            if used.crouching { ActionResult::Success } else { ActionResult::Pass }
        });
        let used = |crouching: bool| AfterUseContext {
            player: ServerPlayer { uuid: 9, name: "Alex".into() }, dimension: "minecraft:overworld".into(),
            eye: [0.5, 64.0, 0.5], look: [0.0, 0.0, 1.0], crouching, creative: false, consumed: false,
        };
        assert!(after_use(&used(true)), "a crouching use is taken");
        assert!(!after_use(&used(false)), "a standing use leaves the click to the game");
    }

    #[test]
    fn a_fishing_state_can_be_cancelled_by_a_callback() {
        PlayerFishEvents::EVENT.register(|fish| {
            if fish.player.uuid == 21 && fish.state == FISHING_CAST { fish.cancel.set(true); }
        });
        let fish = |state: i32| FishingContext {
            player: ServerPlayer { uuid: 21, name: "Fisher".into() }, state, hand: Hand::Main,
            rod: Some(StackData { item: "minecraft:fishing_rod".into(), count: 1, ..Default::default() }),
            cancel: Cell::new(false), actions: RefCell::new(Vec::new()),
        };
        assert!(fishing_heard());
        assert!(fishing(&fish(FISHING_CAST)), "the cast is cancelled");
        assert!(!fishing(&fish(FISHING_BITE)), "the bite is not");
        assert!(!fishing(&fish(2)), "nor is a reel-in");
    }

    #[test]
    fn extra_breaks_are_deduped_capped_and_applied_per_dimension() {
        PlayerBlockBreakEvents::BEFORE.register(|context| {
            if context.dimension != "test:aoe" { return true; }
            // A 3x3 plane around the block, on the face's plane (the top face here: the x/z plane).
            for dx in -1..=1 { for dz in -1..=1 { context.break_extra([context.pos[0] + dx, context.pos[1], context.pos[2] + dz]); } }
            context.break_extra(context.pos);
            true
        });
        let context = BlockBreakContext { dimension: "test:aoe".into(), face: Some(Direction::Up), ..BlockBreakContext::at([10, 64, 10]) };
        assert!(before_block_break(&context));
        let extra = context.extra_breaks.borrow().clone();
        assert_eq!(extra.len(), 8, "the plane's neighbours, without the block itself or duplicates");
        for pos in &extra { queue_extra_break("test:aoe", *pos); }
        assert!(queue_extra_break("test:aoe", extra[0]), "an already queued break counts as queued");
        queue_extra_break("test:other_dimension", [1, 2, 3]);
        let mut broken = Vec::new();
        let applied = apply_pending_extra_breaks("test:aoe", |pos| { broken.push(pos); pos != extra[0] });
        assert_eq!(applied, 7, "the seam's refusal is dropped, not retried");
        assert_eq!(broken.len(), 8);
        assert_eq!(apply_pending_extra_breaks("test:aoe", |_| true), 0, "the queue for this dimension is drained");
        assert_eq!(apply_pending_extra_breaks("test:other_dimension", |_| true), 1, "other dimensions wait for their own seam");
    }

    #[test]
    fn place_predicates_read_blocks_state_and_block_entity_nbt() {
        use super::super::base::Identifier as Id;
        use super::super::resource_loader_v0::RegistryLookup;
        let component = Nbt::parse_snbt(
            "[{blocks:[\"minecraft:chest\",\"#minecraft:logs\"],state:{facing:\"east\"},nbt:{Lock:\"key\"}},\"minecraft:stone\"]",
        )
        .unwrap();
        // A list component holds one predicate per entry: a compound, then a bare block id.
        let predicates = adventure_place_predicates(&component);
        assert_eq!(predicates.len(), 2);
        let first = &predicates[0];
        assert_eq!(
            first.blocks,
            Some(vec!["minecraft:chest".to_string(), "#minecraft:logs".to_string()])
        );
        assert_eq!(
            first.state,
            vec![("facing".to_string(), StateMatcher::Exact("east".to_string()))]
        );
        assert!(first.nbt.is_some());
        assert_eq!(predicates[1].blocks, Some(vec!["minecraft:stone".to_string()]));

        let mined = BlockState {
            block: Id::parse("minecraft:chest").unwrap(),
            properties: [("facing".to_string(), "east".to_string())].into_iter().collect(),
        };
        let mut locked = NbtCompound::new();
        locked.insert("Lock".into(), Nbt::String("key".into()));
        locked.insert("Items".into(), Nbt::List(vec![]));
        let logs = RegistryLookup::from_parts(
            &["minecraft:oak_log".to_string()],
            &[("minecraft:logs".to_string(), vec!["minecraft:oak_log".to_string()])],
        )
        .unwrap();
        assert!(place_predicates_allow(&predicates, &mined, Some(&locked), Some(&logs)));
        // The NBT test fails without the lock, and without any block entity at all.
        let unlocked = NbtCompound::new();
        assert!(!place_predicates_allow(&predicates, &mined, Some(&unlocked), Some(&logs)));
        assert!(!place_predicates_allow(&predicates, &mined, None, Some(&logs)));
        // The second predicate takes a stone block with no state or NBT test.
        let stone = BlockState { block: Id::parse("minecraft:stone").unwrap(), properties: Default::default() };
        assert!(place_predicates_allow(&predicates, &stone, None, Some(&logs)));
        assert!(!place_predicates_allow(&[], &stone, None, Some(&logs)));
        assert!(!AdventurePlacePredicate::never().allows(&stone, None, Some(&logs)));
    }

    #[test]
    fn place_predicates_match_tags_ranges_and_snbt() {
        use super::super::base::Identifier as Id;
        use super::super::resource_loader_v0::RegistryLookup;
        let mined = BlockState {
            block: Id::parse("minecraft:oak_stairs").unwrap(),
            properties: [("waterlogged".to_string(), "false".to_string()), ("age".to_string(), "3".to_string())]
                .into_iter()
                .collect(),
        };
        let logs = RegistryLookup::from_parts(
            &["minecraft:oak_log".to_string()],
            &[("minecraft:logs".to_string(), vec!["minecraft:oak_log".to_string()])],
        )
        .unwrap();
        let tag = AdventurePlacePredicate::from_nbt(&Nbt::parse_snbt("{blocks:\"#minecraft:logs\"}").unwrap());
        let log = BlockState { block: Id::parse("minecraft:oak_log").unwrap(), properties: Default::default() };
        assert!(tag.allows(&log, None, Some(&logs)));
        assert!(!tag.allows(&log, None, None), "a tag names nothing without the block snapshot");
        let ranged = AdventurePlacePredicate::from_nbt(
            &Nbt::parse_snbt("{blocks:\"minecraft:oak_stairs\",state:{age:{min:\"2\",max:\"4\"}}}").unwrap(),
        );
        assert!(ranged.allows(&mined, None, None));
        let too_old = BlockState {
            block: Id::parse("minecraft:oak_stairs").unwrap(),
            properties: [("age".to_string(), "5".to_string())].into_iter().collect(),
        };
        assert!(!ranged.allows(&too_old, None, None));
        let unnumbered = BlockState {
            block: Id::parse("minecraft:oak_stairs").unwrap(),
            properties: [("age".to_string(), "young".to_string())].into_iter().collect(),
        };
        assert!(!ranged.allows(&unnumbered, None, None), "a non-integer value fails a range test");
        let snbt = AdventurePlacePredicate::from_nbt(
            &Nbt::parse_snbt("{blocks:\"minecraft:chest\",nbt:\"{Lock:'key'}\"}").unwrap(),
        );
        let chest = BlockState { block: Id::parse("minecraft:chest").unwrap(), properties: Default::default() };
        let mut locked = NbtCompound::new();
        locked.insert("Lock".into(), Nbt::String("key".into()));
        assert!(snbt.allows(&chest, Some(&locked), None));
        assert!(!snbt.allows(&chest, None, None));
        // A bare block id is the shorthand for a predicate naming that block.
        let bare = adventure_place_predicates(&Nbt::String("minecraft:stone".into()));
        assert_eq!(bare.len(), 1);
        let stone = BlockState { block: Id::parse("minecraft:stone").unwrap(), properties: Default::default() };
        assert!(place_predicates_allow(&bare, &stone, None, None));
        assert!(!place_predicates_allow(&bare, &chest, None, None));
    }

    #[test]
    fn a_break_names_its_breaker_until_its_experience_is_sampled() {
        let breaker = |name: &str| BlockBreaker { uuid: 5, name: name.into(), entity: 6, held: None, creative: false };
        note_block_breaker("minecraft:overworld", [1, 2, 3], breaker("guest"));
        assert_eq!(take_block_breaker("minecraft:overworld", [1, 2, 3]).map(|breaker| breaker.name), Some("guest".to_string()), "the breaker is taken");
        assert_eq!(take_block_breaker("minecraft:overworld", [1, 2, 3]), None, "taken once");
        note_block_breaker("minecraft:overworld", [1, 2, 3], breaker("first"));
        note_block_breaker("minecraft:overworld", [1, 2, 3], breaker("second"));
        assert_eq!(take_block_breaker("minecraft:overworld", [1, 2, 3]).map(|breaker| breaker.name), Some("second".to_string()), "a later break at the same position overwrites");
        assert_eq!(take_block_breaker("minecraft:overworld", [4, 5, 6]), None, "another position's breaker is not taken");
        assert_eq!(take_block_breaker("minecraft:the_nether", [1, 2, 3]), None, "another dimension's breaker is not taken");
        note_block_breaker("minecraft:overworld", [1, 2, 3], breaker("vetoed"));
        clear_block_breaker("minecraft:overworld", [1, 2, 3]);
        assert_eq!(take_block_breaker("minecraft:overworld", [1, 2, 3]), None, "a vetoed break's breaker is forgotten");
    }

    #[test]
    fn the_generic_entity_interaction_is_claimed_by_the_first_answer() {
        InteractEntityCallback::EVENT.register(|context| {
            if context.entity == 41 { ActionResult::Success } else { ActionResult::Pass }
        });
        assert!(entity_interact_heard());
        let click = |entity: i32| InteractEntityContext { entity, kind: "minecraft:armor_stand".into(), stack: None, player: None };
        assert!(entity_interact(&click(41)), "the callback took this interaction");
        assert!(!entity_interact(&click(42)), "a pass leaves the interaction to the game");
    }
}
