//! The `/tconstruct` commands (the original's TConstructCommand family): they inspect and edit
//! the tool in the executing player's hand, and print modifier reports to the server log.
//!
//! Host reach and the divergences it forces, all deliberate:
//! - PlutoLib's command API registers usages of literals and typed arguments, and the host knows
//!   no custom argument types and no registry tab-completion. The original's seven argument types
//!   are therefore `word` arguments resolved here against the loaded registries; a bad value
//!   fails with the same translated message Brigadier would have produced, delivered as chat
//!   feedback instead of a parse error, and numeric range limits are checked here the same way.
//! - The host reaches only the acting player's stacks, so a `<targets>` argument must be `@s` or
//!   the executing player's own name; anything else fails with the held-item failure message
//!   rather than silently applying to a subset of the named targets.
//! - `generate` is not registered: part textures are produced by a client-side generator the
//!   port does not have, and the melting-recipe / hidden-fluid-tag generators write datapack
//!   files to disk, which no PlutoLib API exposes.
//! - Reports are written to the server log (the port's log channel is `eprintln!`) as a plain
//!   padded table; the original's Forge TablePrinter layout is not reproduced exactly, and the
//!   colours the original puts on stat lines and trait names are dropped (the message channel
//!   is plain text).
//! - Modifier level display: `unique` parameters are honoured; `map_level` falls back to the
//!   default form because modifier decode does not retain its leveling formula.
//! - The reports' per-modifier hook sets come from the port's module bindings; module kinds this
//!   port has not bound contribute no hooks, where the original would list theirs.
//! - `tconstruct:harvest_tier` is a tool stat the port's stat enum does not carry (tiers live on
//!   the stats builder, not in the stat map). Bonus-set and reset handle it exactly as the
//!   original does, but the port's stat-override module reads only numeric overrides, so a
//!   stored tier override is inert until that module learns tiers. Of the capacity stats,
//!   `tank_capacity` resolves (the tank system is ported); `max_energy` belongs to an
//!   unported tool system and does not resolve.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use pluto_lib::{CommandContext, CommandRegistrationCallback, I18n, Identifier, Nbt, NbtCompound, RegistryLookup, ServerPlayer, StackData};

use crate::material_stats::Stats;
use crate::materials;
use crate::modifier_entries;
use crate::modifier_modules::{Display, Module, ZoomStyle};
use crate::modifiers::Modifier;
use crate::tool_materials::{MaterialList, MaterialVariant};
use crate::tool_stats::Stat;
use crate::{common, material_stats, modifier_recipes, tool_definitions, tool_damage, tool_parts, tool_stack, tool_station};

const STAT_OVERRIDE: &str = "tconstruct:stat_override";
const OVERRIDE_BONUS: &str = "tconstruct:override_bonus";
const OVERRIDE_MULTIPLIER: &str = "tconstruct:override_multiplier";
const MODIFIABLE_TAG: &str = "tconstruct:modifiable";
const DURABILITY_TAG: &str = "tconstruct:durability";
const TOOL_TRAITS_HOOK: &str = "tconstruct:tool_traits";
const CORE_SLOTS: [&str; 4] = ["upgrades", "defense", "abilities", "souls"];
/// Mantle's PERMISSION_GAME_COMMANDS / PERMISSION_EDIT_SPAWN.
const PERMISSION_GAME_COMMANDS: u8 = 2;
const PERMISSION_EDIT_SPAWN: u8 = 1;

/// A command's result: the Brigadier return value and the feedback lines for the source.
#[derive(Debug)]
struct Outcome {
    value: i32,
    lines: Vec<String>,
}

impl Outcome {
    fn ok(value: i32, line: String) -> Self { Self { value, lines: vec![line] } }
    fn fail(line: String) -> Self { Self { value: 0, lines: vec![line] } }
}

/// The loaded registries a command runs against, borrowed from the dispatch scope's lock guards.
struct Cx<'a> {
    materials: &'a materials::Definitions,
    modifiers: &'a crate::modifiers::Registry,
    recipes: Option<&'a modifier_recipes::Recipes>,
    item_tags: Option<&'a RegistryLookup>,
    tools: Vec<(Identifier, &'a tool_definitions::Data)>,
    item_max: &'a dyn Fn(&Identifier) -> i32,
    /// The held-item commands need all three registries live, as in `break_speed_hook`.
    ready: bool,
}

impl<'a> Cx<'a> {
    fn context(&self) -> tool_station::Context<'_> {
        tool_station::Context {
            materials: self.materials,
            modifiers: self.modifiers,
            item_tags_loaded: self.item_tags.is_some(),
            item_tags: self.item_tags,
            tools: &self.tools,
            item_max: self.item_max,
        }
    }
}

#[derive(Clone, Copy)]
enum UsageFilter {
    All,
    RecipeBare,
    /// `report modifier_usage recipe <slot>`: the slot word is read from the arguments.
    RecipeSlot,
    MaterialTrait,
    ToolTrait,
    Unused,
}

#[derive(Clone, Copy)]
enum Route {
    Modifiers,
    MaterialsSet,
    MaterialsStats,
    MaterialsTraits,
    ToolStats,
    Slots,
    Durability,
    Usage(UsageFilter),
    Priority,
}

pub(crate) fn register() {
    CommandRegistrationCallback::EVENT.register(|registrar| {
        let game = PERMISSION_GAME_COMMANDS;
        registrar.register_op("tconstruct modifiers <targets:word> add <modifier:word>", game, |ctx| run(ctx, Route::Modifiers));
        registrar.register_op("tconstruct modifiers <targets:word> add <modifier:word> <level:integer>", game, |ctx| run(ctx, Route::Modifiers));
        registrar.register_op("tconstruct modifiers <targets:word> remove <modifier:word>", game, |ctx| run(ctx, Route::Modifiers));
        registrar.register_op("tconstruct modifiers <targets:word> remove <modifier:word> <level:integer>", game, |ctx| run(ctx, Route::Modifiers));
        registrar.register_op("tconstruct materials set <targets:word> <index:integer> <material:word>", game, |ctx| run(ctx, Route::MaterialsSet));
        registrar.register_op("tconstruct materials stats <stat:word>", game, |ctx| run(ctx, Route::MaterialsStats));
        registrar.register_op("tconstruct materials stats <stat:word> <scale:number>", game, |ctx| run(ctx, Route::MaterialsStats));
        registrar.register_op("tconstruct materials stats <stat:word> <material:word>", game, |ctx| run(ctx, Route::MaterialsStats));
        registrar.register_op("tconstruct materials stats <stat:word> <material:word> <scale:number>", game, |ctx| run(ctx, Route::MaterialsStats));
        registrar.register_op("tconstruct materials traits <material:word>", game, |ctx| run(ctx, Route::MaterialsTraits));
        registrar.register_op("tconstruct materials traits <material:word> <stat:word>", game, |ctx| run(ctx, Route::MaterialsTraits));
        registrar.register_op("tconstruct tool_stats <targets:word> bonus add <stat:word> <value:number>", game, |ctx| run(ctx, Route::ToolStats));
        registrar.register_op("tconstruct tool_stats <targets:word> bonus set <stat:word> <value:text>", game, |ctx| run(ctx, Route::ToolStats));
        registrar.register_op("tconstruct tool_stats <targets:word> multiplier multiply <stat:word> <value:number>", game, |ctx| run(ctx, Route::ToolStats));
        registrar.register_op("tconstruct tool_stats <targets:word> multiplier set <stat:word> <value:number>", game, |ctx| run(ctx, Route::ToolStats));
        registrar.register_op("tconstruct tool_stats <targets:word> reset", game, |ctx| run(ctx, Route::ToolStats));
        registrar.register_op("tconstruct tool_stats <targets:word> reset <stat:word>", game, |ctx| run(ctx, Route::ToolStats));
        registrar.register_op("tconstruct slots <targets:word> add <slot:word>", game, |ctx| run(ctx, Route::Slots));
        registrar.register_op("tconstruct slots <targets:word> add <slot:word> <count:integer>", game, |ctx| run(ctx, Route::Slots));
        registrar.register_op("tconstruct slots <targets:word> set <slot:word> <count:integer>", game, |ctx| run(ctx, Route::Slots));
        registrar.register_op("tconstruct durability <targets:word> add <amount:integer>", game, |ctx| run(ctx, Route::Durability));
        registrar.register_op("tconstruct durability <targets:word> set <amount:integer>", game, |ctx| run(ctx, Route::Durability));
        let spawn = PERMISSION_EDIT_SPAWN;
        registrar.register_op("tconstruct report modifier_usage", spawn, |ctx| run(ctx, Route::Usage(UsageFilter::All)));
        registrar.register_op("tconstruct report modifier_usage all", spawn, |ctx| run(ctx, Route::Usage(UsageFilter::All)));
        registrar.register_op("tconstruct report modifier_usage recipe", spawn, |ctx| run(ctx, Route::Usage(UsageFilter::RecipeBare)));
        registrar.register_op("tconstruct report modifier_usage recipe <slot:word>", spawn, |ctx| run(ctx, Route::Usage(UsageFilter::RecipeSlot)));
        registrar.register_op("tconstruct report modifier_usage material_trait", spawn, |ctx| run(ctx, Route::Usage(UsageFilter::MaterialTrait)));
        registrar.register_op("tconstruct report modifier_usage tool_trait", spawn, |ctx| run(ctx, Route::Usage(UsageFilter::ToolTrait)));
        registrar.register_op("tconstruct report modifier_usage unused", spawn, |ctx| run(ctx, Route::Usage(UsageFilter::Unused)));
        registrar.register_op("tconstruct report modifier_priority", spawn, |ctx| run(ctx, Route::Priority));
        registrar.register_op("tconstruct report modifier_priority <hook:word>", spawn, |ctx| run(ctx, Route::Priority));
    });
}

fn run(ctx: &CommandContext, route: Route) -> i32 {
    let outcome = dispatch(ctx, route);
    for line in &outcome.lines {
        ctx.source.send_message(line);
    }
    outcome.value
}

fn dispatch(ctx: &CommandContext, route: Route) -> Outcome {
    let player = &ctx.source;
    let definitions = tool_definitions::register().read().unwrap_or_else(|poisoned| poisoned.into_inner());
    let materials = common::register().read().unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(modifiers_lock) = common::server_modifiers() else {
        return Outcome::fail(tr("command.tconstruct.held_modifiable.failed"));
    };
    let modifiers = modifiers_lock.read().unwrap_or_else(|poisoned| poisoned.into_inner());
    let recipes_guard = common::modifier_recipes().map(|lock| lock.read().unwrap_or_else(|poisoned| poisoned.into_inner()));
    let item_tags = common::item_snapshot();
    let tools: Vec<(Identifier, &tool_definitions::Data)> = tool_definitions::item_pairs(&definitions);
    let item_max = |id: &Identifier| tool_parts::tool_stack_size(id);
    let cx = Cx {
        materials: &materials,
        modifiers: &modifiers,
        recipes: recipes_guard.as_deref(),
        item_tags: item_tags.as_deref(),
        tools,
        item_max: &item_max,
        ready: materials.server_loaded && modifiers.definitions_loaded && !definitions.data.is_empty(),
    };
    match route {
        Route::Modifiers => modifiers_command(&cx, player, &ctx.args),
        Route::MaterialsSet => materials_set(&cx, player, &ctx.args),
        Route::MaterialsStats => materials_stats(&cx, &ctx.args),
        Route::MaterialsTraits => materials_traits(&cx, &ctx.args),
        Route::ToolStats => tool_stats_command(&cx, player, &ctx.args),
        Route::Slots => slots_command(&cx, player, &ctx.args),
        Route::Durability => durability_command(&cx, player, &ctx.args),
        Route::Usage(filter) => report_usage(&cx, filter, &ctx.args),
        Route::Priority => report_priority(&cx, ctx.args.first().map(String::as_str)),
    }
}

// ---------- shared helpers ----------

fn tr(key: &str) -> String { I18n::translate(key) }

fn tr_args(key: &str, args: &[&str]) -> String { I18n::translate_args(key, args) }

fn failed() -> Outcome { Outcome::fail(tr("command.tconstruct.held_modifiable.failed")) }

fn int_low(min: i32, found: i32) -> Outcome {
    Outcome::fail(tr_args("argument.integer.low", &[&min.to_string(), &found.to_string()]))
}

fn float_low(found: f32) -> Outcome {
    Outcome::fail(tr_args("argument.float.low", &["0.0", &java_float(found)]))
}

/// An IdParser word: a bare path takes the `tconstruct` namespace, as the original's arguments do.
fn word_id(word: &str) -> Option<Identifier> {
    if word.contains(':') { Identifier::parse(word).ok() } else { Identifier::new("tconstruct", word).ok() }
}

/// The targets subset the host can reach: the executing player, as `@s` or by name.
fn check_targets(player: &ServerPlayer, word: &str) -> Result<(), Outcome> {
    if word == "@s" || word == player.name { Ok(()) } else { Err(failed()) }
}

fn parse_int(word: Option<&String>) -> Result<i32, Outcome> {
    word.and_then(|word| word.parse::<i32>().ok()).ok_or_else(failed)
}

fn parse_float(word: Option<&String>) -> Result<f32, Outcome> {
    word.and_then(|word| word.parse::<f32>().ok()).filter(|value| value.is_finite()).ok_or_else(failed)
}

fn nbt_f32(value: Option<&Nbt>) -> Option<f32> {
    match value {
        Some(Nbt::Byte(v)) => Some(f32::from(*v)),
        Some(Nbt::Short(v)) => Some(f32::from(*v)),
        Some(Nbt::Int(v)) => Some(*v as f32),
        Some(Nbt::Long(v)) => Some(*v as f32),
        Some(Nbt::Float(v)) => Some(*v),
        Some(Nbt::Double(v)) => Some(*v as f32),
        _ => None,
    }
}

/// The validation error string the port produces is a translation key plus an argument key.
fn validation_line(error: &str) -> String {
    match error.split_once(' ') {
        Some((key, arg)) => {
            let arg = if arg.contains('.') || arg.contains(':') { tr(arg) } else { arg.to_owned() };
            tr_args(key, &[&arg])
        }
        None => tr(error),
    }
}

// ---------- number and name formatting (the original's Util formats) ----------

/// Rounds to an integer with HALF_EVEN, as DecimalFormat does.
fn round_half_even(value: f64) -> i64 {
    let floor = value.floor();
    let frac = value - floor;
    let rounded = if frac > 0.5 {
        floor + 1.0
    } else if frac < 0.5 {
        floor
    } else if (floor as i64) % 2 == 0 {
        floor
    } else {
        floor + 1.0
    };
    rounded as i64
}

fn group_digits(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut out = String::new();
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if value < 0 {
        format!("-{out}")
    } else {
        out
    }
}

/// A DecimalFormat with at most 2 fraction digits (HALF_EVEN, trailing zeros dropped).
fn decimal2(value: f64, grouping: bool) -> String {
    let scaled = round_half_even(value * 100.0);
    let magnitude = scaled.abs();
    let int_part = magnitude / 100;
    let frac_part = magnitude % 100;
    let mut text = if grouping { group_digits(int_part) } else { int_part.to_string() };
    if scaled < 0 {
        text = format!("-{text}");
    }
    if frac_part != 0 {
        let mut fraction = format!("{frac_part:02}");
        while fraction.ends_with('0') {
            fraction.pop();
        }
        text.push('.');
        text.push_str(&fraction);
    }
    text
}

/// COMMA_FORMAT `#,###,###.##`.
pub(crate) fn comma(value: f64) -> String { decimal2(value, true) }

/// COMMA_FORMAT on an integer value.
pub(crate) fn comma_int(value: i32) -> String { group_digits(i64::from(value)) }

/// BONUS_FORMAT `#.##` with a positive prefix.
fn bonus_format(value: f32) -> String {
    let body = decimal2(f64::from(value), false);
    if value >= 0.0 { format!("+{body}") } else { body }
}

/// PERCENT_BOOST_FORMAT `#%` with a positive prefix; DecimalFormat multiplies by 100.
fn percent_boost(value: f32) -> String {
    let rounded = round_half_even(f64::from(value) * 100.0);
    if value >= 0.0 { format!("+{rounded}%") } else { format!("{rounded}%") }
}

/// The shortest round-trip digits of a float, as (digits, decimal exponent of the leading digit).
fn shortest_digits(text: String) -> (String, i32) {
    // Rust's `{:e}` renders like "1.2345e5" / "5e-1" / "-2.5e0".
    let (mantissa, exponent) = text.split_once('e').expect("scientific form has an exponent");
    let exponent: i32 = exponent.parse().expect("scientific exponent");
    let digits: String = mantissa.chars().filter(|ch| ch.is_ascii_digit()).collect();
    (digits, exponent)
}

fn render_java_number(digits: &str, exponent: i32, negative: bool) -> String {
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    if (-3..7).contains(&exponent) {
        // Plain form on [1e-3, 1e7), always with a fraction part (".0" when integral).
        if exponent >= 0 {
            let int_len = (exponent + 1) as usize;
            let mut int_part: String = digits.chars().take(int_len).collect();
            while int_part.len() < int_len {
                int_part.push('0');
            }
            out.push_str(&int_part);
            out.push('.');
            let fraction: String = digits.chars().skip(int_len).collect();
            out.push_str(if fraction.is_empty() { "0" } else { &fraction });
        } else {
            out.push_str("0.");
            for _ in 0..(-exponent - 1) {
                out.push('0');
            }
            out.push_str(digits);
        }
    } else {
        // Scientific form: one digit, a fraction of at least one digit, then E<exponent>.
        let mut chars = digits.chars();
        out.push(chars.next().unwrap_or('0'));
        out.push('.');
        let rest: String = chars.collect();
        out.push_str(if rest.is_empty() { "0" } else { &rest });
        out.push('E');
        out.push_str(&exponent.to_string());
    }
    out
}

/// Float.toString: shortest round-trip, `.0` on integral values, scientific outside [1e-3, 1e7).
fn java_float(value: f32) -> String {
    if value == 0.0 {
        return if value.is_sign_negative() { "-0.0".into() } else { "0.0".into() };
    }
    if !value.is_finite() {
        return value.to_string();
    }
    let (digits, exponent) = shortest_digits(format!("{:e}", value.abs()));
    render_java_number(&digits, exponent, value < 0.0)
}

/// Double.toString, the same rules at double precision.
fn java_double(value: f64) -> String {
    if value == 0.0 {
        return if value.is_sign_negative() { "-0.0".into() } else { "0.0".into() };
    }
    if !value.is_finite() {
        return value.to_string();
    }
    let (digits, exponent) = shortest_digits(format!("{:e}", value.abs()));
    render_java_number(&digits, exponent, value < 0.0)
}

/// RomanNumeralHelper: a translated numeral when the language has one, else the literal numeral
/// (values below 1 render as the integer itself).
fn roman(value: i32) -> String {
    let key = format!("roman_numeral.value.{value}");
    if I18n::has(&key) {
        return I18n::translate(&key);
    }
    if value < 1 {
        return value.to_string();
    }
    let mut rest = value;
    let mut out = String::new();
    for (amount, symbol) in [(1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"), (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")] {
        while rest >= amount {
            out.push_str(symbol);
            rest -= amount;
        }
    }
    out
}

/// Modifier.getDisplayName(level) for the port's retained level-display kinds.
fn modifier_name(modifier: &Modifier, level: i32) -> String {
    let key = modifier.translation_key.clone().unwrap_or_else(|| format!("modifier.{}.{}", modifier.id.namespace(), modifier.id.path()));
    let base = I18n::translate(&key);
    let default = |base: &str| format!("{base} {}", roman(level));
    match modifier.level_display.path() {
        "no_levels" => base,
        "single_level" => {
            if level == 1 { base } else { default(&base) }
        }
        "pluses" => {
            if level > 1 { format!("{base}{}", "+".repeat(usize::try_from(level - 1).unwrap_or(0))) } else { base }
        }
        "unique" => match modifier.level_display_unique {
            Some((until, first)) => {
                if !first && level == 1 {
                    base
                } else if level <= until {
                    let specific = format!("{key}.{level}");
                    if I18n::has(&specific) { I18n::translate(&specific) } else { default(&base) }
                } else {
                    default(&base)
                }
            }
            None => default(&base),
        },
        _ => default(&base),
    }
}

/// MaterialTooltipCache: a variant name key wins only when it is translatable.
fn material_name(variant: &MaterialVariant) -> String {
    let text = variant.to_string();
    let (base, variant_name) = match text.split_once('#') {
        Some((base, name)) => (base, Some(name)),
        None => (text.as_str(), None),
    };
    let base_key = match base.split_once(':') {
        Some((namespace, path)) => format!("material.{namespace}.{path}"),
        None => format!("material.{base}"),
    };
    if let Some(name) = variant_name {
        let key = format!("{base_key}.{name}");
        if I18n::has(&key) {
            return I18n::translate(&key);
        }
    }
    I18n::translate(&base_key)
}

pub(crate) fn material_id_name(id: &Identifier) -> String {
    I18n::translate(&format!("material.{}.{}", id.namespace(), id.path()))
}

pub(crate) fn stat_type_name(id: &Identifier) -> String {
    I18n::translate(&format!("stat.{}.{}", id.namespace(), id.path()))
}

fn tier_name(tier: &Identifier) -> String {
    let dotted = format!("{}.{}", tier.namespace(), tier.path());
    let key = format!("stat.tconstruct.harvest_tier.{dotted}");
    if I18n::has(&key) {
        return I18n::translate(&key);
    }
    let key = format!("harvest_tier.{dotted}");
    if I18n::has(&key) {
        return I18n::translate(&key);
    }
    tier.to_string()
}

// ---------- SNBT scalars (for `tool_stats bonus set`) ----------

fn unescape_snbt(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some(next) => out.push(next),
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn quote_snbt(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        if ch == '"' || ch == '\\' {
            out.push('\\');
        }
        out.push(ch);
    }
    out.push('"');
    out
}

/// Parses one scalar SNBT value (the only shapes a tool stat can read); None for anything else.
fn parse_snbt_scalar(text: &str) -> Option<Nbt> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if text.len() >= 2 && ((text.starts_with('"') && text.ends_with('"')) || (text.starts_with('\'') && text.ends_with('\''))) {
        return Some(Nbt::String(unescape_snbt(&text[1..text.len() - 1])));
    }
    match text {
        "true" => return Some(Nbt::Byte(1)),
        "false" => return Some(Nbt::Byte(0)),
        _ => {}
    }
    let (number, suffix) = match text.chars().last().expect("non-empty") {
        'b' | 'B' => (&text[..text.len() - 1], 'b'),
        's' | 'S' => (&text[..text.len() - 1], 's'),
        'l' | 'L' => (&text[..text.len() - 1], 'l'),
        'f' | 'F' => (&text[..text.len() - 1], 'f'),
        'd' | 'D' => (&text[..text.len() - 1], 'd'),
        _ => (text, ' '),
    };
    match suffix {
        'b' => number.parse::<i8>().ok().map(Nbt::Byte),
        's' => number.parse::<i16>().ok().map(Nbt::Short),
        'l' => number.parse::<i64>().ok().map(Nbt::Long),
        'f' => number.parse::<f32>().ok().map(Nbt::Float),
        'd' => number.parse::<f64>().ok().map(Nbt::Double),
        _ => {
            if let Ok(value) = text.parse::<i64>() {
                if let Ok(value) = i32::try_from(value) { Some(Nbt::Int(value)) } else { Some(Nbt::Long(value)) }
            } else {
                text.parse::<f64>().ok().map(Nbt::Double)
            }
        }
    }
}

/// StringTag.valueOf for a scalar tag, as the bonus-set success message prints it.
fn snbt(tag: &Nbt) -> String {
    match tag {
        Nbt::Byte(value) => format!("{value}b"),
        Nbt::Short(value) => format!("{value}s"),
        Nbt::Int(value) => format!("{value}"),
        Nbt::Long(value) => format!("{value}l"),
        Nbt::Float(value) => format!("{}f", java_float(*value)),
        Nbt::Double(value) => format!("{}d", java_double(*value)),
        Nbt::String(value) => quote_snbt(value),
        _ => "{}".into(),
    }
}

// ---------- the held-item pipeline (HeldModifiableItemIterator) ----------

/// The executing player's main-hand stack and its tool definition, once the tag check passed.
struct Held<'a> {
    slot: i32,
    stack: StackData,
    item: Identifier,
    definition: Option<&'a tool_definitions::Data>,
}

fn held<'a>(cx: &Cx<'a>, player: &ServerPlayer, tag: &str, need_definition: bool) -> Result<Held<'a>, Outcome> {
    if !cx.ready {
        return Err(failed());
    }
    let Some(slot) = player.selected_slot() else { return Err(failed()) };
    let Some(stack) = player.stack(slot) else {
        return Err(Outcome::fail(tr_args("command.tconstruct.held_modifiable.failed.no_item", &[&player.name])));
    };
    let invalid = || Outcome::fail(tr_args("command.tconstruct.held_modifiable.failed.invalid_item", &[&player.name]));
    let Ok(item) = Identifier::parse(&stack.item) else { return Err(invalid()) };
    let tag_id = Identifier::parse(tag).expect("built-in tag id");
    if !tool_station::item_in_tag(cx.item_tags, &cx.tools, &item, &tag_id) {
        return Err(invalid());
    }
    // tool_station::definition_for, spelled out so the reference keeps the registries' lifetime.
    let definition = cx.tools.iter().find(|(id, _)| *id == item).map(|(_, data)| *data);
    if need_definition && definition.is_none() {
        return Err(invalid());
    }
    Ok(Held { slot, stack, item, definition })
}

/// Writes the edited data back, mirroring the stack's damage and fake max damage from the tool
/// data exactly as the stations' `sync_visuals` does.
fn write_back(player: &ServerPlayer, held: &Held, nbt: NbtCompound) -> Result<(), Outcome> {
    let mut out = held.stack.clone();
    out.custom_data = nbt;
    out.damage = u32::try_from(tool_stack::damage(&out.custom_data)).unwrap_or(0);
    out.max_damage = Some(tool_damage::fake_max_damage(&out.custom_data, true).max(1));
    if player.update_stack(held.slot, &out) { Ok(()) } else { Err(failed()) }
}

/// rebuildStats + tryValidate, the pair every editing command runs before writing.
fn rebuild_validate(nbt: &mut NbtCompound, env: &tool_stack::Environment) -> Result<(), Outcome> {
    tool_stack::rebuild_stats(nbt, env).map_err(|_| failed())?;
    if let Some(error) = tool_stack::try_validate(nbt, env) {
        return Err(Outcome::fail(validation_line(&error)));
    }
    Ok(())
}

// ---------- argument resolution (the original's seven custom argument types) ----------

fn resolve_modifier(cx: &Cx, word: &str) -> Result<Arc<Modifier>, Outcome> {
    let Some(id) = word_id(word) else { return Err(Outcome::fail(tr("argument.id.invalid"))) };
    cx.modifiers.get(&id).cloned().ok_or_else(|| Outcome::fail(tr_args("command.tconstruct.modifier.not_found", &[&id.to_string()])))
}

fn resolve_material(cx: &Cx, word: &str) -> Result<Identifier, Outcome> {
    let Some(id) = word_id(word) else { return Err(Outcome::fail(tr("argument.id.invalid"))) };
    let resolved = cx.materials.resolve_redirect(&id).clone();
    if cx.materials.materials.contains_key(&resolved) {
        Ok(resolved)
    } else {
        Err(Outcome::fail(tr_args("command.tconstruct.material.not_found", &[&id.to_string()])))
    }
}

/// MaterialVariantArgument: parse only, no existence check; bare paths take `tconstruct`.
fn parse_variant_arg(word: &str) -> Result<MaterialVariant, Outcome> {
    let (base, variant) = match word.split_once('#') {
        Some((base, variant)) => (base, Some(variant)),
        None => (word, None),
    };
    let base = if base.contains(':') { base.to_owned() } else { format!("tconstruct:{base}") };
    let text = match variant {
        Some(variant) => format!("{base}#{variant}"),
        None => base,
    };
    MaterialVariant::parse(&text).ok_or_else(|| Outcome::fail(tr("argument.id.invalid")))
}

fn resolve_stat_type(word: &str) -> Result<Identifier, Outcome> {
    let Some(id) = word_id(word) else { return Err(Outcome::fail(tr("argument.id.invalid"))) };
    if material_stats::is_registered(&id) {
        Ok(id)
    } else {
        Err(Outcome::fail(tr_args("command.tconstruct.material_stat.not_found", &[&id.to_string()])))
    }
}

#[derive(Clone, Copy)]
enum ToolStat {
    Numeric(Stat),
    Tier,
}

impl ToolStat {
    fn key(self) -> String {
        match self {
            Self::Numeric(stat) => format!("tconstruct:{}", stat.path()),
            Self::Tier => "tconstruct:harvest_tier".to_owned(),
        }
    }
    fn prefix(self) -> String {
        match self {
            Self::Numeric(stat) => tr(&format!("tool_stat.tconstruct.{}", stat.path())),
            Self::Tier => tr("tool_stat.tconstruct.harvest_tier"),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum StatKind {
    Float,
    Integer,
    Percent,
}

fn stat_kind(stat: Stat) -> StatKind {
    match stat {
        Stat::Durability | Stat::SeaLuck | Stat::Lure | Stat::Overslime | Stat::Hunger => StatKind::Integer,
        Stat::WaterInertia | Stat::EdibleCounterChance => StatKind::Percent,
        _ => StatKind::Float,
    }
}

fn resolve_tool_stat(word: &str, numeric_only: bool) -> Result<ToolStat, Outcome> {
    let Some(id) = word_id(word) else { return Err(Outcome::fail(tr("argument.id.invalid"))) };
    if id.namespace() == "tconstruct" && id.path() == "harvest_tier" {
        if numeric_only {
            return Err(Outcome::fail(tr_args("command.tconstruct.stat_type.wrong_type", &[&id.to_string(), "INumericToolStat"])));
        }
        return Ok(ToolStat::Tier);
    }
    match Stat::from_id(&id) {
        Some(stat) => Ok(ToolStat::Numeric(stat)),
        None => Err(Outcome::fail(tr_args("command.tconstruct.stat_type.not_found", &[&id.to_string()]))),
    }
}

/// SlotType.getIfPresent over the port's created types: the core four, module-created types and
/// the slot types modifier recipes spend.
fn slot_types(cx: &Cx) -> BTreeSet<String> {
    let mut set: BTreeSet<String> = CORE_SLOTS.iter().map(|name| name.to_string()).collect();
    for id in cx.modifiers.all_ids() {
        if let Some(modifier) = cx.modifiers.get(&id) {
            for module in modifier.modules() {
                match module {
                    Module::Slot { name, .. } => {
                        set.insert(name.clone());
                    }
                    Module::BonusSlot { bonus, .. } => {
                        set.insert(bonus.clone());
                    }
                    _ => {}
                }
            }
        }
    }
    if let Some(recipes) = cx.recipes {
        for recipe in recipes.all() {
            if let Some((name, _)) = &recipe.slots {
                set.insert(name.clone());
            }
            for level in &recipe.levels {
                if let Some((name, _)) = &level.slots {
                    set.insert(name.clone());
                }
            }
        }
    }
    set
}

fn resolve_slot(cx: &Cx, word: &str) -> Result<String, Outcome> {
    if slot_types(cx).contains(word) {
        Ok(word.to_owned())
    } else {
        Err(Outcome::fail(tr_args("command.tconstruct.slot_type.not_found", &[word])))
    }
}

// ---------- modifiers add/remove ----------

fn modifiers_command(cx: &Cx, player: &ServerPlayer, args: &[String]) -> Outcome {
    let (Some(targets), Some(op), Some(word)) = (args.first(), args.get(1), args.get(2)) else { return failed() };
    if let Err(outcome) = check_targets(player, targets) {
        return outcome;
    }
    let modifier = match resolve_modifier(cx, word) {
        Ok(modifier) => modifier,
        Err(outcome) => return outcome,
    };
    let held = match held(cx, player, MODIFIABLE_TAG, true) {
        Ok(held) => held,
        Err(outcome) => return outcome,
    };
    let context = cx.context();
    let env = context.environment(&held.item, held.definition.expect("definition required"));
    let mut nbt = held.stack.custom_data.clone();
    match op.as_str() {
        "add" => {
            let level = match args.get(3) {
                None => 1,
                Some(word) => match parse_int(Some(word)) {
                    Ok(level) => level,
                    Err(outcome) => return outcome,
                },
            };
            if level < 1 {
                return int_low(1, level);
            }
            let upgrades = tool_stack::upgrades(&nbt);
            let Ok(updated) = upgrades.with_modifier(modifier.id.clone(), level) else { return failed() };
            nbt.insert(tool_stack::TAG_UPGRADES.to_owned(), updated.to_nbt());
            if let Err(outcome) = rebuild_validate(&mut nbt, &env) {
                return outcome;
            }
            if let Err(outcome) = write_back(player, &held, nbt) {
                return outcome;
            }
            Outcome::ok(1, tr_args("command.tconstruct.modifiers.success.add.single", &[&modifier_name(&modifier, level), &player.name]))
        }
        "remove" => {
            let level = match args.get(3) {
                None => -1,
                Some(word) => match parse_int(Some(word)) {
                    Ok(level) => level,
                    Err(outcome) => return outcome,
                },
            };
            if level != -1 && level < 1 {
                return int_low(1, level);
            }
            let current = modifier_entries::List::from_nbt(nbt.get(tool_stack::TAG_MODIFIERS)).level(&modifier.id);
            if current == 0 {
                return Outcome::fail(tr_args("command.tconstruct.modifiers.failure.too_few_levels", &[&modifier_name(&modifier, level), &player.name]));
            }
            let remove = if level == -1 { current } else { level };
            let upgrades = tool_stack::upgrades(&nbt);
            let Ok(updated) = upgrades.without_modifier(&modifier.id, remove) else { return failed() };
            nbt.insert(tool_stack::TAG_UPGRADES.to_owned(), updated.to_nbt());
            if let Err(outcome) = rebuild_validate(&mut nbt, &env) {
                return outcome;
            }
            // The removeRawData hook has no port counterpart (its only original implementors are
            // cosmetic); ModifierRemovalHook.onRemoved is the port's on_removed.
            tool_stack::on_removed(&held.stack.custom_data, &mut nbt, &env);
            if let Err(outcome) = write_back(player, &held, nbt) {
                return outcome;
            }
            Outcome::ok(1, tr_args("command.tconstruct.modifiers.success.remove.single", &[&modifier_name(&modifier, remove), &player.name]))
        }
        _ => failed(),
    }
}

// ---------- materials set ----------

fn materials_set(cx: &Cx, player: &ServerPlayer, args: &[String]) -> Outcome {
    let (Some(targets), Some(index_word), Some(word)) = (args.first(), args.get(1), args.get(2)) else { return failed() };
    if let Err(outcome) = check_targets(player, targets) {
        return outcome;
    }
    let index = match parse_int(Some(index_word)) {
        Ok(index) => index,
        Err(outcome) => return outcome,
    };
    if index < 0 {
        return int_low(0, index);
    }
    let variant = match parse_variant_arg(word) {
        Ok(variant) => variant,
        Err(outcome) => return outcome,
    };
    let held = match held(cx, player, MODIFIABLE_TAG, true) {
        Ok(held) => held,
        Err(outcome) => return outcome,
    };
    let definition = held.definition.expect("definition required");
    let slot_count = definition.material_stat_types().map_or(0, |types| types.len());
    if index as usize >= slot_count {
        return Outcome::fail(tr_args("command.tconstruct.materials.failure.invalid_index", &[&index.to_string(), &player.name]));
    }
    let context = cx.context();
    let env = context.environment(&held.item, definition);
    let mut nbt = held.stack.custom_data.clone();
    let list = tool_stack::materials(&nbt, env.has_materials);
    let Ok(replaced) = list.replaced(index, variant.clone()) else { return failed() };
    tool_stack::set_materials_raw(&mut nbt, &replaced);
    if let Err(outcome) = rebuild_validate(&mut nbt, &env) {
        return outcome;
    }
    tool_stack::on_removed(&held.stack.custom_data, &mut nbt, &env);
    if let Err(outcome) = write_back(player, &held, nbt) {
        return outcome;
    }
    Outcome::ok(1, tr_args("command.tconstruct.materials.success.set.single", &[&index.to_string(), &material_name(&variant), &player.name]))
}

// ---------- tool_stats (the stat_override edits) ----------

fn persistent_group<'a>(nbt: &'a NbtCompound, group: &str) -> Option<&'a NbtCompound> {
    match nbt.get(tool_stack::TAG_PERSISTENT) {
        Some(Nbt::Compound(persistent)) => match persistent.get(group) {
            Some(Nbt::Compound(compound)) => Some(compound),
            _ => None,
        },
        _ => None,
    }
}

fn override_value(nbt: &NbtCompound, group: &str, key: &str, default: f32) -> f32 {
    persistent_group(nbt, group).and_then(|compound| nbt_f32(compound.get(key))).unwrap_or(default)
}

fn group_nonempty(nbt: &NbtCompound, group: &str) -> bool {
    persistent_group(nbt, group).is_some_and(|compound| compound.iter().next().is_some())
}

/// StatOverrideModifier.setStat: storing the neutral value removes the key, and an emptied
/// group compound is removed from the persistent data.
fn set_in_group(persistent: &mut NbtCompound, group: &str, key: &str, value: Option<Nbt>) {
    let mut compound = match persistent.remove(group) {
        Some(Nbt::Compound(compound)) => compound,
        Some(other) => {
            persistent.insert(group.to_owned(), other);
            if value.is_none() {
                return;
            }
            NbtCompound::new()
        }
        None => {
            if value.is_none() {
                return;
            }
            NbtCompound::new()
        }
    };
    match value {
        Some(tag) => {
            compound.insert(key.to_owned(), tag);
        }
        None => {
            compound.remove(key);
        }
    }
    if compound.iter().next().is_some() {
        persistent.insert(group.to_owned(), Nbt::Compound(compound));
    }
}

fn set_float_override(nbt: &mut NbtCompound, group: &str, key: &str, value: f32, neutral: f32) {
    let stored = if value == neutral { None } else { Some(Nbt::Float(value)) };
    tool_stack::with_persistent(nbt, |persistent| set_in_group(persistent, group, key, stored));
}

/// StatsCommand.updateEntities: the stat_override modifier is added or removed so it is present
/// exactly while either override group holds data; no onRemoved runs here in the original.
fn update_entities(nbt: &mut NbtCompound, env: &tool_stack::Environment) -> Result<(), Outcome> {
    let needs = group_nonempty(nbt, OVERRIDE_BONUS) || group_nonempty(nbt, OVERRIDE_MULTIPLIER);
    let override_id = Identifier::parse(STAT_OVERRIDE).expect("built-in modifier id");
    let upgrades = tool_stack::upgrades(nbt);
    let level = upgrades.level(&override_id);
    let changed = if needs && level == 0 {
        Some(upgrades.with_modifier(override_id.clone(), 1))
    } else if !needs && level > 0 {
        Some(upgrades.without_modifier(&override_id, level))
    } else {
        None
    };
    if let Some(changed) = changed {
        let Ok(list) = changed else { return Err(failed()) };
        nbt.insert(tool_stack::TAG_UPGRADES.to_owned(), list.to_nbt());
    }
    rebuild_validate(nbt, env)
}

/// The six vanilla tiers are the ones the port's tier registry knows.
fn tier_from_name(text: &str) -> Option<Identifier> {
    let id = if text.contains(':') { Identifier::parse(text).ok()? } else { Identifier::new("minecraft", text).ok()? };
    if id.namespace() == "minecraft" && ["wood", "gold", "stone", "iron", "diamond", "netherite"].contains(&id.path()) { Some(id) } else { None }
}

/// IToolStat.read plus the value StatOverrideModifier.set would store; None when unreadable.
fn read_stat_value(stat: ToolStat, tag: &Nbt) -> Option<(Nbt, bool)> {
    match stat {
        ToolStat::Numeric(stat) => {
            let value = nbt_f32(Some(tag))?;
            match stat_kind(stat) {
                StatKind::Integer => {
                    let value = value as i32;
                    Some((Nbt::Int(value), value != 0))
                }
                _ => Some((Nbt::Float(value), value != 0.0)),
            }
        }
        ToolStat::Tier => {
            let Nbt::String(text) = tag else { return None };
            let tier = tier_from_name(text)?;
            let store = tier != Identifier::new("minecraft", "wood").expect("built-in tier");
            Some((Nbt::String(tier.to_string()), store))
        }
    }
}

fn tool_stats_command(cx: &Cx, player: &ServerPlayer, args: &[String]) -> Outcome {
    let (Some(targets), Some(group)) = (args.first(), args.get(1)) else { return failed() };
    // Argument resolution and the pre-iteration failures happen before targets are iterated,
    // as in the original.
    enum Op {
        BonusAdd(Stat, f32),
        BonusSet(ToolStat, String),
        Multiply(Stat, f32),
        MultiplierSet(Stat, f32),
        Reset(Option<ToolStat>),
    }
    let op = match group.as_str() {
        "bonus" => match args.get(2).map(String::as_str) {
            Some("add") => {
                let stat = match args.get(3).map(|word| resolve_tool_stat(word, true)) {
                    Some(Ok(ToolStat::Numeric(stat))) => stat,
                    Some(Ok(_)) => return failed(),
                    Some(Err(outcome)) => return outcome,
                    None => return failed(),
                };
                let value = match parse_float(args.get(4)) {
                    Ok(value) => value,
                    Err(outcome) => return outcome,
                };
                if value == 0.0 {
                    return Outcome::fail(tr("command.tconstruct.stats.failure.invalid_add"));
                }
                Op::BonusAdd(stat, value)
            }
            Some("set") => {
                let stat = match args.get(3).map(|word| resolve_tool_stat(word, false)) {
                    Some(Ok(stat)) => stat,
                    Some(Err(outcome)) => return outcome,
                    None => return failed(),
                };
                let Some(text) = args.get(4) else { return failed() };
                Op::BonusSet(stat, text.clone())
            }
            _ => return failed(),
        },
        "multiplier" => match args.get(2).map(String::as_str) {
            Some("multiply") | Some("set") => {
                let multiply = args.get(2).map(String::as_str) == Some("multiply");
                let stat = match args.get(3).map(|word| resolve_tool_stat(word, true)) {
                    Some(Ok(ToolStat::Numeric(stat))) => stat,
                    Some(Ok(_)) => return failed(),
                    Some(Err(outcome)) => return outcome,
                    None => return failed(),
                };
                let value = match parse_float(args.get(4)) {
                    Ok(value) => value,
                    Err(outcome) => return outcome,
                };
                if value < 0.0 {
                    return float_low(value);
                }
                if multiply && value == 1.0 {
                    return Outcome::fail(tr("command.tconstruct.stats.failure.invalid_multiply"));
                }
                if multiply { Op::Multiply(stat, value) } else { Op::MultiplierSet(stat, value) }
            }
            _ => return failed(),
        },
        "reset" => {
            let stat = match args.get(2) {
                None => None,
                Some(word) => match resolve_tool_stat(word, false) {
                    Ok(stat) => Some(stat),
                    Err(outcome) => return outcome,
                },
            };
            Op::Reset(stat)
        }
        _ => return failed(),
    };
    if let Err(outcome) = check_targets(player, targets) {
        return outcome;
    }
    let held = match held(cx, player, MODIFIABLE_TAG, true) {
        Ok(held) => held,
        Err(outcome) => return outcome,
    };
    let context = cx.context();
    let env = context.environment(&held.item, held.definition.expect("definition required"));
    let mut nbt = held.stack.custom_data.clone();
    match op {
        Op::BonusAdd(stat, value) => {
            let key = ToolStat::Numeric(stat).key();
            let current = override_value(&nbt, OVERRIDE_BONUS, &key, 0.0);
            set_float_override(&mut nbt, OVERRIDE_BONUS, &key, current + value, 0.0);
            if let Err(outcome) = update_entities(&mut nbt, &env) {
                return outcome;
            }
            if let Err(outcome) = write_back(player, &held, nbt) {
                return outcome;
            }
            Outcome::ok(1, tr_args("command.tconstruct.stats.success.bonus.modify.single", &[&ToolStat::Numeric(stat).prefix(), &java_float(value), &player.name]))
        }
        Op::BonusSet(stat, text) => {
            let parse_fail = || {
                Outcome::fail(tr_args("command.tconstruct.stats.success.bonus.set.parse_fail", &[&stat.prefix(), &text]))
            };
            let Some(tag) = parse_snbt_scalar(&text) else { return parse_fail() };
            let Some((stored, store)) = read_stat_value(stat, &tag) else { return parse_fail() };
            tool_stack::with_persistent(&mut nbt, |persistent| {
                set_in_group(persistent, OVERRIDE_BONUS, &stat.key(), if store { Some(stored) } else { None });
            });
            if let Err(outcome) = update_entities(&mut nbt, &env) {
                return outcome;
            }
            if let Err(outcome) = write_back(player, &held, nbt) {
                return outcome;
            }
            Outcome::ok(1, tr_args("command.tconstruct.stats.success.bonus.set.single", &[&stat.prefix(), &snbt(&tag), &player.name]))
        }
        Op::Multiply(stat, value) => {
            let key = ToolStat::Numeric(stat).key();
            let current = override_value(&nbt, OVERRIDE_MULTIPLIER, &key, 1.0);
            set_float_override(&mut nbt, OVERRIDE_MULTIPLIER, &key, current * value, 1.0);
            if let Err(outcome) = update_entities(&mut nbt, &env) {
                return outcome;
            }
            if let Err(outcome) = write_back(player, &held, nbt) {
                return outcome;
            }
            Outcome::ok(1, tr_args("command.tconstruct.stats.success.multiplier.modify.single", &[&ToolStat::Numeric(stat).prefix(), &java_float(value), &player.name]))
        }
        Op::MultiplierSet(stat, value) => {
            let key = ToolStat::Numeric(stat).key();
            set_float_override(&mut nbt, OVERRIDE_MULTIPLIER, &key, value, 1.0);
            if let Err(outcome) = update_entities(&mut nbt, &env) {
                return outcome;
            }
            if let Err(outcome) = write_back(player, &held, nbt) {
                return outcome;
            }
            Outcome::ok(1, tr_args("command.tconstruct.stats.success.multiplier.set.single", &[&ToolStat::Numeric(stat).prefix(), &java_float(value), &player.name]))
        }
        Op::Reset(None) => {
            // resetAll: only a tool whose final modifier list carries stat_override changes.
            let override_id = Identifier::parse(STAT_OVERRIDE).expect("built-in modifier id");
            let final_level = modifier_entries::List::from_nbt(nbt.get(tool_stack::TAG_MODIFIERS)).level(&override_id);
            if final_level > 0 {
                let upgrades = tool_stack::upgrades(&nbt);
                let level = upgrades.level(&override_id);
                if level > 0 {
                    let Ok(updated) = upgrades.without_modifier(&override_id, level) else { return failed() };
                    nbt.insert(tool_stack::TAG_UPGRADES.to_owned(), updated.to_nbt());
                }
                if let Err(outcome) = rebuild_validate(&mut nbt, &env) {
                    return outcome;
                }
                // The original invokes stat_override's remove hook explicitly here; its whole
                // effect is clearing the two override compounds.
                tool_stack::with_persistent(&mut nbt, |persistent| {
                    persistent.remove(OVERRIDE_BONUS);
                    persistent.remove(OVERRIDE_MULTIPLIER);
                });
                if let Err(outcome) = write_back(player, &held, nbt) {
                    return outcome;
                }
            }
            Outcome::ok(1, tr_args("command.tconstruct.stats.success.reset.all.single", &[&player.name]))
        }
        Op::Reset(Some(stat)) => {
            // resetStat: a numeric stat's multiplier is reset before its bonus is removed.
            if matches!(stat, ToolStat::Numeric(_)) {
                set_float_override(&mut nbt, OVERRIDE_MULTIPLIER, &stat.key(), 1.0, 1.0);
            }
            tool_stack::with_persistent(&mut nbt, |persistent| set_in_group(persistent, OVERRIDE_BONUS, &stat.key(), None));
            if let Err(outcome) = update_entities(&mut nbt, &env) {
                return outcome;
            }
            if let Err(outcome) = write_back(player, &held, nbt) {
                return outcome;
            }
            Outcome::ok(1, tr_args("command.tconstruct.stats.success.reset.stat.single", &[&stat.prefix(), &player.name]))
        }
    }
}

// ---------- slots add/set ----------

/// ToolDataNBT.addSlots on the persistent data: ignores 0, removes the key at a 0 total.
fn add_slot_count(persistent: &mut NbtCompound, name: &str, count: i32) {
    if count == 0 {
        return;
    }
    let total = modifier_entries::nbt_int(persistent.get(name)).wrapping_add(count);
    if total == 0 {
        persistent.remove(name);
    } else {
        persistent.insert(name.to_owned(), Nbt::Int(total));
    }
}

/// ToolDataNBT.getFreeSlots: persistent plus volatile counts for the type.
fn free_slots(nbt: &NbtCompound, name: &str) -> i32 {
    let persistent = match nbt.get(tool_stack::TAG_PERSISTENT) {
        Some(Nbt::Compound(compound)) => modifier_entries::nbt_int(compound.get(name)),
        _ => 0,
    };
    let volatile = match nbt.get(tool_stack::TAG_VOLATILE) {
        Some(Nbt::Compound(compound)) => modifier_entries::nbt_int(compound.get(name)),
        _ => 0,
    };
    persistent + volatile
}

fn slots_command(cx: &Cx, player: &ServerPlayer, args: &[String]) -> Outcome {
    let (Some(targets), Some(op), Some(word)) = (args.first(), args.get(1), args.get(2)) else { return failed() };
    let slot = match resolve_slot(cx, word) {
        Ok(slot) => slot,
        Err(outcome) => return outcome,
    };
    let count = match op.as_str() {
        "add" => match args.get(3) {
            None => 1,
            Some(word) => match parse_int(Some(word)) {
                Ok(count) => count,
                Err(outcome) => return outcome,
            },
        },
        "set" => match parse_int(args.get(3)) {
            Ok(count) => count,
            Err(outcome) => return outcome,
        },
        _ => return failed(),
    };
    if op == "add" && count == 0 {
        return Outcome::fail(tr("command.tconstruct.slots.failure.invalid_count"));
    }
    if op == "set" && count < 0 {
        return int_low(0, count);
    }
    if let Err(outcome) = check_targets(player, targets) {
        return outcome;
    }
    let held = match held(cx, player, MODIFIABLE_TAG, true) {
        Ok(held) => held,
        Err(outcome) => return outcome,
    };
    let context = cx.context();
    let env = context.environment(&held.item, held.definition.expect("definition required"));
    let mut nbt = held.stack.custom_data.clone();
    match op.as_str() {
        "add" => tool_stack::with_persistent(&mut nbt, |persistent| add_slot_count(persistent, &slot, count)),
        "set" => {
            let delta = count - free_slots(&held.stack.custom_data, &slot);
            tool_stack::with_persistent(&mut nbt, |persistent| add_slot_count(persistent, &slot, delta));
        }
        _ => return failed(),
    }
    if let Err(outcome) = rebuild_validate(&mut nbt, &env) {
        return outcome;
    }
    if let Err(outcome) = write_back(player, &held, nbt) {
        return outcome;
    }
    let display = tr(&format!("slot_type.tconstruct.{slot}"));
    match op.as_str() {
        "add" => Outcome::ok(1, tr_args("command.tconstruct.slots.success.add.single", &[&count.to_string(), &display, &player.name])),
        _ => Outcome::ok(1, tr_args("command.tconstruct.slots.success.set.single", &[&display, &count.to_string(), &player.name])),
    }
}

// ---------- durability add/set ----------

fn durability_command(cx: &Cx, player: &ServerPlayer, args: &[String]) -> Outcome {
    let (Some(targets), Some(op), Some(amount_word)) = (args.first(), args.get(1), args.get(2)) else { return failed() };
    let amount = match parse_int(Some(amount_word)) {
        Ok(amount) => amount,
        Err(outcome) => return outcome,
    };
    match op.as_str() {
        "add" => {
            if amount == 0 {
                return Outcome::fail(tr("command.tconstruct.durability.failure.invalid_amount"));
            }
        }
        "set" => {
            if amount < 0 {
                return int_low(0, amount);
            }
        }
        _ => return failed(),
    }
    if let Err(outcome) = check_targets(player, targets) {
        return outcome;
    }
    let held = match held(cx, player, DURABILITY_TAG, false) {
        Ok(held) => held,
        Err(outcome) => return outcome,
    };
    let mut nbt = held.stack.custom_data.clone();
    match op.as_str() {
        "add" => {
            let damage = tool_stack::damage(&nbt);
            tool_stack::set_damage(&mut nbt, (damage - amount).max(0));
        }
        _ => {
            let durability = tool_stack::durability(&nbt);
            tool_stack::set_damage(&mut nbt, (durability - amount).max(0));
        }
    }
    if let Err(outcome) = write_back(player, &held, nbt) {
        return outcome;
    }
    Outcome::ok(1, tr_args(&format!("command.tconstruct.durability.success.{op}.single"), &[&amount.to_string(), &player.name]))
}

// ---------- materials stats / traits (chat reports) ----------

/// One getLocalizedInfo line: the stat's translated prefix plus its formatted value.
fn stat_line(stat_path: &str, formatted: String) -> String {
    format!("{}{}", tr(&format!("tool_stat.tconstruct.{stat_path}")), formatted)
}

/// The repair-value line shared by skull and repair stats: (int)(durability / 3f), unscaled.
fn repair_line(durability: i32) -> String {
    stat_line("durability.repair", comma_int((durability as f32 / 3.0) as i32))
}

/// IMaterialStats.getLocalizedInfo for the port's stat records.
pub(crate) fn localized_info(stats: &Stats, stat_type: &Identifier, scale: f32) -> Vec<String> {
    match stats {
        Stats::Head { durability, mining_speed, tier, attack } => vec![
            stat_line("durability", comma_int((*durability as f32 * scale) as i32)),
            stat_line("harvest_tier", tier_name(tier)),
            stat_line("mining_speed", comma(f64::from(mining_speed * scale))),
            stat_line("attack_damage", comma(f64::from(attack * scale))),
        ],
        // Only durability is scaled in the original's handle info; the rest print raw.
        Stats::Handle { durability, mining_speed, melee_speed, attack } => vec![
            stat_line("durability", percent_boost(durability * scale)),
            stat_line("attack_damage", percent_boost(*attack)),
            stat_line("attack_speed", percent_boost(*melee_speed)),
            stat_line("mining_speed", percent_boost(*mining_speed)),
        ],
        Stats::Limb { durability, draw_speed, velocity, accuracy } => vec![
            stat_line("durability", comma_int((*durability as f32 * scale) as i32)),
            stat_line("draw_speed", bonus_format(draw_speed * scale)),
            stat_line("velocity", bonus_format(velocity * scale)),
            stat_line("accuracy", bonus_format(accuracy * scale)),
        ],
        Stats::Grip { durability, accuracy, attack } => vec![
            stat_line("durability", percent_boost(durability * scale)),
            stat_line("accuracy", bonus_format(accuracy * scale)),
            stat_line("attack_damage", comma(f64::from(attack * scale))),
        ],
        Stats::Plating { durability, armor, toughness, knockback_resistance } => {
            let mut lines = vec![stat_line("durability", comma_int((*durability as f32 * scale) as i32))];
            if stat_type.path() != "plating_shield" {
                lines.push(stat_line("armor", comma(f64::from(armor * scale))));
            }
            lines.push(stat_line("armor_toughness", comma(f64::from(toughness * scale))));
            // Displayed at ten times the value, as vanilla scales knockback resistance.
            lines.push(stat_line("knockback_resistance", comma(f64::from(knockback_resistance * 10.0 * scale))));
            lines
        }
        Stats::Skull { durability, armor } => {
            let mut lines = vec![repair_line(*durability)];
            if *armor > 0 {
                lines.push(stat_line("armor", comma(f64::from(*armor as f32 * scale))));
            }
            lines
        }
        Stats::Slime { durability, overslime } => vec![
            stat_line("durability", comma_int((*durability as f32 * scale) as i32)),
            stat_line("overslime", comma_int((*overslime as f32 * scale) as i32)),
        ],
        Stats::Repair { durability } => vec![repair_line(*durability)],
        Stats::Statless => vec![tr("tool_stat.tconstruct.extra.no_stats")],
    }
}

fn materials_stats(cx: &Cx, args: &[String]) -> Outcome {
    let Some(stat_word) = args.first() else { return failed() };
    let stat_type = match resolve_stat_type(stat_word) {
        Ok(stat_type) => stat_type,
        Err(outcome) => return outcome,
    };
    // Brigadier tries the scale child before the material child, so a numeric second word is a
    // scale even though both usages land here.
    let (material, scale) = match (args.get(1), args.get(2)) {
        (None, _) => (None, 1.0),
        (Some(second), None) => match second.parse::<f32>() {
            Ok(scale) if scale.is_finite() => (None, scale),
            _ => match resolve_material(cx, second) {
                Ok(material) => (Some(material), 1.0),
                Err(outcome) => return outcome,
            },
        },
        (Some(second), Some(third)) => {
            let material = match resolve_material(cx, second) {
                Ok(material) => material,
                Err(outcome) => return outcome,
            };
            match parse_float(Some(third)) {
                Ok(scale) => (Some(material), scale),
                Err(outcome) => return outcome,
            }
        }
    };
    if scale < 0.0 {
        return float_low(scale);
    }
    let stat_name = stat_type_name(&stat_type);
    match material {
        None => {
            let mut text = tr_args("command.tconstruct.materials.success.stats.default", &[&stat_name, &java_float(scale)]);
            if let Some(stats) = material_stats::default_stats(&stat_type) {
                for line in localized_info(&stats, &stat_type, scale) {
                    text.push_str("\n* ");
                    text.push_str(&line);
                }
            }
            Outcome::ok(1, text)
        }
        Some(material) => {
            let Some(stats) = cx.materials.get_stats(&material, &stat_type) else {
                return Outcome::fail(tr_args("command.tconstruct.materials.failure.invalid_stats", &[&stat_type.to_string(), &material_id_name(&material)]));
            };
            let mut text = tr_args("command.tconstruct.materials.success.stats.material", &[&stat_name, &material_id_name(&material), &java_float(scale)]);
            for line in localized_info(stats, &stat_type, scale) {
                text.push_str("\n* ");
                text.push_str(&line);
            }
            Outcome::ok(1, text)
        }
    }
}

fn materials_traits(cx: &Cx, args: &[String]) -> Outcome {
    let Some(word) = args.first() else { return failed() };
    let material = match resolve_material(cx, word) {
        Ok(material) => material,
        Err(outcome) => return outcome,
    };
    let name = material_id_name(&material);
    let (header, entries) = match args.get(1) {
        None => {
            let entries: &[Option<crate::material_traits::ModifierEntry>] = match cx.materials.traits.get(&material) {
                Some(traits) => traits.default.as_slice(),
                None => &[],
            };
            (tr_args("command.tconstruct.materials.success.traits.default", &[&name]), entries)
        }
        Some(word) => {
            let stat_type = match resolve_stat_type(word) {
                Ok(stat_type) => stat_type,
                Err(outcome) => return outcome,
            };
            let header = tr_args("command.tconstruct.materials.success.traits.stat", &[&stat_type_name(&stat_type), &name]);
            (header, cx.materials.get_traits(&material, &stat_type))
        }
    };
    let mut text = header;
    let mut count = 0;
    for entry in entries.iter().flatten() {
        count += 1;
        let display = match cx.modifiers.get(&entry.id) {
            Some(modifier) => modifier_name(modifier, entry.level),
            None => entry.id.to_string(),
        };
        text.push_str(&format!("\n* {display} ({})", entry.id));
    }
    if count == 0 {
        text.push_str(&format!("\n* {}", tr("command.tconstruct.materials.failure.no_traits")));
    }
    Outcome { value: count, lines: vec![text] }
}

// ---------- reports ----------

/// A plain padded table: header row, then one row per record, columns joined with " | ".
fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut widths: Vec<usize> = headers.iter().map(|header| header.chars().count()).collect();
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(cell.chars().count());
        }
    }
    let render = |cells: &[String]| -> String {
        cells
            .iter()
            .enumerate()
            .map(|(index, cell)| format!("{cell:<width$}", width = widths[index]))
            .collect::<Vec<_>>()
            .join(" | ")
            .trim_end()
            .to_owned()
    };
    let mut out = render(&headers.iter().map(|header| header.to_string()).collect::<Vec<_>>());
    for row in rows {
        out.push('\n');
        out.push_str(&render(row));
    }
    out
}

/// The hook ids a module binds, from the port's module bindings (the decode guards' functional
/// sets), under the original's ModifierHooks names; the stats hook is `modifier_stats` there,
/// where the port's data spelling is `tool_stats`.
fn module_hooks(module: &Module, out: &mut BTreeSet<&'static str>) {
    match module {
        Module::Flag { .. } | Module::Int { .. } | Module::Slot { .. } | Module::Rarity { .. } | Module::Sweep { .. } => {
            out.insert("tconstruct:volatile_data");
        }
        Module::CreativeSlots { .. } => {
            out.insert("tconstruct:volatile_data");
            out.insert("tconstruct:remove");
        }
        Module::Trait { .. } => {
            out.insert("tconstruct:modifier_traits");
        }
        Module::SwappableTraits { .. } => {
            out.insert("tconstruct:modifier_traits");
            out.insert("tconstruct:remove");
        }
        Module::Boost { .. } | Module::SetTier { .. } | Module::StatCopy { .. } => {
            out.insert("tconstruct:modifier_stats");
        }
        Module::StatOverride => {
            out.insert("tconstruct:modifier_stats");
            out.insert("tconstruct:remove");
        }
        Module::Repair { .. } => {
            out.insert("tconstruct:repair_factor");
        }
        Module::Requirements { .. } => {
            out.insert("tconstruct:validate_upgrade");
            out.insert("tconstruct:requirements");
        }
        Module::SwappableSlot { .. } => {
            out.insert("tconstruct:volatile_data");
            out.insert("tconstruct:remove");
        }
        Module::BonusSlot { .. } => {
            out.insert("tconstruct:volatile_data");
        }
        Module::ReduceDamage { .. } | Module::DamageRange { .. } | Module::DurabilityShield { .. } => {
            out.insert("tconstruct:tool_damage");
        }
        Module::MiningSpeed { .. } => {
            out.insert("tconstruct:break_speed");
        }
        Module::ConditionalStat { .. } => {
            out.insert("tconstruct:conditional_stat");
        }
        Module::ConditionalPower { .. } => {
            out.insert("tconstruct:projectile_launch");
            out.insert("tconstruct:projectile_shot");
            out.insert("tconstruct:projectile_hit");
            out.insert("tconstruct:sling_force");
        }
        Module::MeleeDamage { .. } => {
            out.insert("tconstruct:melee_damage");
            out.insert("tconstruct:monster_melee_damage");
        }
        Module::Knockback { .. } => {
            out.insert("tconstruct:melee_hit");
        }
        Module::WeaponEffect { .. } => {
            out.insert("tconstruct:melee_hit");
            out.insert("tconstruct:monster_melee_hit");
            out.insert("tconstruct:projectile_hit");
        }
        Module::UsageEffect { .. } => {
            out.insert("tconstruct:block_break");
            out.insert("tconstruct:melee_hit");
            out.insert("tconstruct:monster_melee_hit");
            out.insert("tconstruct:projectile_launch");
            out.insert("tconstruct:projectile_shot");
            out.insert("tconstruct:projectile_thrown");
            out.insert("tconstruct:projectile_hit");
            out.insert("tconstruct:plant_harvest");
            out.insert("tconstruct:shear_entity");
            out.insert("tconstruct:sling_launch");
        }
        Module::BlockDamage { .. } => {
            out.insert("tconstruct:damage_block");
        }
        Module::EffectImmunity { .. } => {
            out.insert("tconstruct:equipment_change");
        }
        Module::CapacityBar { .. } => {
            out.insert("tconstruct:capacity_bar");
        }
        Module::CapacityMelee { .. } => {
            out.insert("tconstruct:melee_hit");
        }
        Module::CapacityDamage { .. } => {
            out.insert("tconstruct:modify_damage");
        }
        Module::CapacityMining { .. } => {
            out.insert("tconstruct:block_harvest");
        }
        Module::CapacityLoot { .. } => {
            out.insert("tconstruct:process_loot");
        }
        Module::CapacityLaunch { .. } => {
            out.insert("tconstruct:projectile_launch");
            out.insert("tconstruct:projectile_shot");
            out.insert("tconstruct:projectile_thrown");
        }
        Module::Lifesteal { .. } => {
            out.insert("tconstruct:melee_hit");
            out.insert("tconstruct:monster_melee_hit");
            out.insert("tconstruct:projectile_launch");
            out.insert("tconstruct:projectile_hit");
        }
        Module::Inventory { .. } => {
            out.insert("tconstruct:volatile_data");
            out.insert("tconstruct:validate");
            out.insert("tconstruct:remove");
            out.insert("tconstruct:slot_stack");
        }
        Module::InventoryMenu(_) => {
            out.insert("tconstruct:armor_interact");
            out.insert("tconstruct:general_interact");
        }
        Module::InventorySlotMenu => {
            out.insert("tconstruct:slot_stack");
        }
        Module::MaxArmorAttribute { .. } | Module::GoldenAttribute { .. } | Module::ArmorLevel { .. } | Module::ShowOffhand { .. } | Module::MobDisguise { .. }
        | Module::ClearEffectOnUnequip { .. } | Module::DamageOnUnequip { .. } | Module::ReduceEffectOnUnequip { .. } => {
            out.insert("tconstruct:equipment_change");
        }
        Module::LightspeedAttribute { .. } => {
            out.insert("tconstruct:boot_walk");
            out.insert("tconstruct:equipment_change");
        }
        Module::Edible => {
            out.insert("tconstruct:general_interact");
            out.insert("tconstruct:tool_using");
            out.insert("tconstruct:on_attacked");
        }
        Module::Severing => {
            out.insert("tconstruct:process_loot");
        }
        Module::EdibleWear { .. } | Module::EdibleCure { .. } | Module::EdibleRemove { .. } => {
            out.insert("tconstruct:edible_effect");
        }
        Module::Thorns { .. } => {
            out.insert("tconstruct:on_attacked");
        }
        Module::AttackAction { .. } => {
            out.insert("tconstruct:melee_hit");
            out.insert("tconstruct:monster_melee_hit");
            out.insert("tconstruct:projectile_launch");
            out.insert("tconstruct:projectile_shot");
            out.insert("tconstruct:projectile_thrown");
            out.insert("tconstruct:projectile_hit");
        }
        Module::CounterAction { .. } | Module::CounterEffect { .. } => {
            out.insert("tconstruct:on_attacked");
        }
        Module::ArmorAttackEffect { .. } => {
            out.insert("tconstruct:damage_dealt");
        }
        Module::LegacyEffect { .. } => {
            out.insert("tconstruct:on_attacked");
            out.insert("tconstruct:melee_hit");
            out.insert("tconstruct:monster_melee_hit");
            out.insert("tconstruct:projectile_hit");
        }
        Module::TheOneProbe => {
            out.insert("tconstruct:raw_data");
        }
        Module::Display(display) => match display {
            Display::InteractionSource | Display::MaterialVariantColor { .. } | Display::VariantColor | Display::VariantName { .. } | Display::BannerPatterns | Display::Dyed | Display::Trim => {
                out.insert("tconstruct:display_name");
            }
            Display::Embellished => {
                out.insert("tconstruct:display_name");
                out.insert("tconstruct:remove");
                out.insert("tconstruct:raw_data");
            }
            Display::DurabilityColor { .. } => {
                out.insert("tconstruct:durability_display");
            }
            Display::StatTooltip { .. } => {
                out.insert("tconstruct:tooltip");
            }
        },
        Module::ToolActions { .. } => {
            out.insert("tconstruct:tool_action");
        }
        Module::ToolAction { .. } => {
            out.insert("tconstruct:block_interact");
            out.insert("tconstruct:tool_action");
            out.insert("tconstruct:aoe_highlight");
        }
        Module::Enchantment { ignoring, .. } => {
            out.insert("tconstruct:enchantments");
            if *ignoring {
                out.insert("tconstruct:protection");
            }
        }
        Module::ArmorLooting { .. } => {
            out.insert("tconstruct:armor_looting");
        }
        Module::Protection { .. } | Module::DepthProtection { .. } | Module::Overshield { .. } | Module::ProtectionCap { .. } | Module::FlameBarrier { .. } => {
            out.insert("tconstruct:protection");
        }
        Module::AdjustDamage { .. } | Module::RestoreLostHealth { .. } | Module::TeleportDodge { .. } => {
            out.insert("tconstruct:modify_damage");
        }
        Module::Attribute { .. } => {
            out.insert("tconstruct:attributes");
            out.insert("tconstruct:equipment_change");
        }
        Module::CustomAttribute { .. } => {
            out.insert("tconstruct:attributes");
            out.insert("tconstruct:equipment_change");
        }
        Module::Float { .. } => {
            out.insert("tconstruct:volatile_data");
            out.insert("tconstruct:projectile_launch");
        }
        Module::CraftCount { .. } => {
            out.insert("tconstruct:craft_count");
        }
        Module::MaterialRepair { .. } => {
            out.insert("tconstruct:material_repair");
        }
        Module::QuiverInventory { .. } => {
            out.insert("tconstruct:volatile_data");
            out.insert("tconstruct:validate");
            out.insert("tconstruct:remove");
            out.insert("tconstruct:slot_stack");
        }
        Module::UpdateHealth { .. } => {
            out.insert("tconstruct:equipment_change");
        }
        Module::Punch { .. } => {
            out.insert("tconstruct:projectile_launch");
            out.insert("tconstruct:projectile_shot");
        }
        Module::ReversePunch { .. } => {
            out.insert("tconstruct:projectile_hit");
            out.insert("tconstruct:projectile_hit_client");
        }
        Module::MeleeInstrument { .. } => {
            out.insert("tconstruct:melee_hit");
            out.insert("tconstruct:monster_melee_hit");
            out.insert("tconstruct:damage_dealt");
        }
        Module::Overgrowth { .. } => {
            out.insert("tconstruct:inventory_tick");
        }
        Module::Kinetic => {
            out.insert("tconstruct:on_attacked");
            out.insert("tconstruct:tooltip");
        }
        Module::Conducting => {
            out.insert("tconstruct:melee_hit");
            out.insert("tconstruct:monster_melee_hit");
            out.insert("tconstruct:conditional_stat");
        }
        Module::Insatiable => {
            out.insert("tconstruct:melee_hit");
            out.insert("tconstruct:conditional_stat");
        }
        Module::Lacerating => {
            out.insert("tconstruct:melee_hit");
            out.insert("tconstruct:on_attacked");
        }
        Module::Momentum => {
            out.insert("tconstruct:break_speed");
            out.insert("tconstruct:conditional_stat");
        }
        Module::Enderclearance { .. } => {
            out.insert("tconstruct:melee_hit");
            out.insert("tconstruct:on_attacked");
        }
        Module::Channeling { .. } => {
            out.insert("tconstruct:melee_hit");
        }
        Module::BulkQuiver { .. } => {
            out.insert("tconstruct:bow_ammo");
        }
        Module::TrickQuiver => {
            out.insert("tconstruct:bow_ammo");
            out.insert("tconstruct:general_interact");
            out.insert("tconstruct:remove");
        }
        Module::ShieldStrap { .. } | Module::ToolBelt { .. } => {
            out.insert("tconstruct:armor_interact");
        }
        Module::Minimap => {
            out.insert("tconstruct:equipment_change");
            out.insert("tconstruct:armor_interact");
            out.insert("tconstruct:inventory_tick");
            out.insert("tconstruct:remove");
        }
        Module::ShareDurability { .. } => {
            out.insert("tconstruct:tool_damage");
        }
        Module::Shears { .. } => {
            out.insert("tconstruct:entity_interact");
            out.insert("tconstruct:tool_action");
        }
        Module::Fishing => {
            out.insert("tconstruct:general_interact");
            out.insert("tconstruct:tool_action");
            out.insert("tconstruct:equipment_change");
        }
        Module::RecurrentProtection { .. } => {
            out.insert("tconstruct:modify_hurt");
        }
        Module::DamageOnShoot { .. } => {
            out.insert("tconstruct:projectile_launch");
            out.insert("tconstruct:projectile_shot");
            out.insert("tconstruct:projectile_thrown");
            out.insert("tconstruct:sling_launch");
        }
        Module::SlingForce { .. } => {
            out.insert("tconstruct:sling_force");
        }
        Module::Sling { .. } => {
            out.insert("tconstruct:general_interact");
            out.insert("tconstruct:tool_using");
            out.insert("tconstruct:armor_interact");
            out.insert("tconstruct:melee_hit");
            out.insert("tconstruct:melee_damage");
        }
        Module::Fireball { .. } => {
            out.insert("tconstruct:general_interact");
            out.insert("tconstruct:armor_interact");
        }
        Module::Tipped => {
            out.insert("tconstruct:projectile_launch");
            out.insert("tconstruct:projectile_shot");
            out.insert("tconstruct:projectile_thrown");
            out.insert("tconstruct:projectile_hit");
            out.insert("tconstruct:display_name");
            out.insert("tconstruct:tooltip");
            out.insert("tconstruct:remove");
        }
        Module::Infinity { .. } => {
            out.insert("tconstruct:bow_ammo");
            out.insert("tconstruct:projectile_launch");
            out.insert("tconstruct:projectile_shot");
            out.insert("tconstruct:remove");
        }
        Module::CampfireExtinguish { .. } => {
            out.insert("tconstruct:block_interact");
            out.insert("tconstruct:aoe_highlight");
        }
        Module::Blocking => {
            out.insert("tconstruct:general_interact");
            out.insert("tconstruct:tool_action");
        }
        Module::WalkTransform { .. } => {
            out.insert("tconstruct:boot_walk");
            out.insert("tconstruct:tool_action");
        }
        Module::ArrowPierce { .. } => {
            out.insert("tconstruct:projectile_shot");
        }
        Module::ProjectileBounce { .. } => {
            out.insert("tconstruct:projectile_hit");
        }
        Module::ProjectileGravity { .. } | Module::ProjectileFuse { .. } => {
            out.insert("tconstruct:projectile_shot");
            out.insert("tconstruct:schedule_projectile_task");
        }
        Module::ProjectileExplosion { .. } | Module::ProjectileAttractMobs { .. } => {
            out.insert("tconstruct:projectile_hit");
            out.insert("tconstruct:projectile_fuse");
        }
        Module::ProjectilePlaceGlow { .. } => {
            out.insert("tconstruct:projectile_hit");
            out.insert("tconstruct:projectile_launch");
        }
        Module::RestrictAngle { .. } => {
            out.insert("tconstruct:projectile_shot");
            out.insert("tconstruct:sling_launch");
        }
        Module::Throwing => {
            out.insert("tconstruct:general_interaction");
        }
        Module::Fov { .. } => {
            out.insert("tconstruct:equipment_change");
        }
        Module::Zoom { style } => {
            out.insert("tconstruct:general_interact");
            out.insert("tconstruct:tool_using");
            out.insert("tconstruct:equipment_change");
            if matches!(style, ZoomStyle::Spyglass) {
                out.insert("tconstruct:armor_interact");
            }
        }
        Module::Tank => {
            out.insert("tconstruct:volatile_data");
            out.insert("tconstruct:validate");
            out.insert("tconstruct:remove");
            out.insert("tconstruct:slot_stack");
            out.insert("tconstruct:display_name");
        }
        Module::Bucket { .. } => {
            out.insert("tconstruct:block_interact");
            out.insert("tconstruct:general_interact");
        }
        Module::TankInteraction { .. } => {
            out.insert("tconstruct:block_interact");
        }
        Module::Melting { .. } => {
            out.insert("tconstruct:melee_hit");
            out.insert("tconstruct:monster_melee_hit");
            out.insert("tconstruct:launcher_hit");
            out.insert("tconstruct:process_loot");
        }
        Module::Slurping { .. } => {
            out.insert("tconstruct:general_interact");
            out.insert("tconstruct:tool_using");
            out.insert("tconstruct:armor_interact");
            out.insert("tconstruct:inventory_tick");
            out.insert("tconstruct:equipment_change");
        }
        Module::Spilling { .. } => {
            out.insert("tconstruct:melee_hit");
            out.insert("tconstruct:monster_melee_hit");
            out.insert("tconstruct:launcher_hit");
        }
        Module::Spitting { .. } => {
            out.insert("tconstruct:general_interact");
            out.insert("tconstruct:armor_interact");
            out.insert("tconstruct:inventory_tick");
        }
        Module::Splashing { .. } => {
            out.insert("tconstruct:entity_interact");
            out.insert("tconstruct:block_interact");
            out.insert("tconstruct:aoe_highlight");
        }
        Module::Overburn => {
            out.insert("tconstruct:inventory_tick");
            out.insert("tconstruct:remove");
        }
        Module::Smashing => {
            out.insert("tconstruct:projectile_launch");
            out.insert("tconstruct:projectile_shot");
            out.insert("tconstruct:projectile_hit");
            out.insert("tconstruct:projectile_fuse");
            out.insert("tconstruct:volatile_data");
            out.insert("tconstruct:validate");
            out.insert("tconstruct:remove");
            out.insert("tconstruct:display_name");
            out.insert("tconstruct:tooltip");
        }
        Module::EdibleRepresentative { .. } => {
            out.insert("tconstruct:tool_using");
            out.insert("tconstruct:edible_effect");
        }
        Module::FireWalker { .. } | Module::GlowWalker { .. } | Module::CoverGround { .. } => {
            out.insert("tconstruct:boot_walk");
        }
        Module::PlaceGlow { .. } => {
            out.insert("tconstruct:block_interact");
            out.insert("tconstruct:remove_block");
        }
        Module::BlockItemProvider { .. } => {
            out.insert("tconstruct:block_item_provider");
        }
        Module::ArmorHarvestEnchantment { .. } => {
            out.insert("tconstruct:harvest_enchantments");
        }
        Module::Autosmelt { .. } => {
            out.insert("tconstruct:process_loot");
        }
        Module::Smelting { .. } => {
            out.insert("tconstruct:melee_hit");
            out.insert("tconstruct:monster_melee_hit");
            out.insert("tconstruct:launcher_hit");
            out.insert("tconstruct:block_harvest");
            out.insert("tconstruct:projectile_launch");
            out.insert("tconstruct:on_attacked");
            out.insert("tconstruct:plant_harvest");
            out.insert("tconstruct:shear_entity");
            out.insert("tconstruct:sling_launch");
        }
        Module::Dwarven => {
            out.insert("tconstruct:break_speed");
            out.insert("tconstruct:conditional_stat");
        }
        Module::Enderporting => {
            out.insert("tconstruct:melee_hit");
            out.insert("tconstruct:block_harvest");
            out.insert("tconstruct:plant_harvest");
            out.insert("tconstruct:projectile_hit");
        }
        Module::Harvest => {
            out.insert("tconstruct:block_interact");
            out.insert("tconstruct:aoe_highlight");
        }
        Module::SelfDestructive => {
            out.insert("tconstruct:armor_interact");
            out.insert("tconstruct:equipment_change");
        }
        Module::StrongBones => {
            out.insert("tconstruct:equipment_change");
        }
    }
}

fn modifier_hooks(modifier: &Modifier) -> BTreeSet<&'static str> {
    let mut out = BTreeSet::new();
    for module in modifier.modules() {
        module_hooks(module, &mut out);
    }
    out
}

/// Every hook id in the original's ModifierHooks registry, for the hook argument; the port's
/// data spelling `tool_stats` is accepted for the stats hook and normalized.
fn known_hook(word: &str) -> Option<String> {
    let id = word_id(word)?;
    let path = if id.path() == "tool_stats" { "modifier_stats" } else { id.path() };
    const HOOKS: &[&str] = &[
        "conditional_stat", "tool_action", "equipment_change", "repair_factor", "material_repair", "tool_damage", "durability_changed",
        "inventory_tick", "block_item_provider", "capacity_bar", "display_name", "tooltip", "durability_display", "requirements",
        "craft_count", "modifier_stats", "attributes", "enchantments", "volatile_data", "raw_data", "validate", "validate_upgrade",
        "remove", "modifier_traits", "melee_damage", "monster_melee_damage", "melee_hit", "monster_melee_hit", "protection",
        "damage_block", "on_attacked", "modify_hurt", "modify_damage", "damage_dealt", "process_loot", "weapon_looting",
        "armor_looting", "harvest_enchantments", "break_speed", "remove_block", "block_break", "block_harvest", "projectile_launch",
        "projectile_shot", "projectile_thrown", "projectile_hit", "projectile_hit_client", "launcher_hit", "projectile_fuse",
        "bow_ammo", "schedule_projectile_task", "sling_force", "sling_angle", "sling_launch", "elytra_flight", "boot_walk",
        "general_interact", "tool_using", "block_interact", "entity_interact", "armor_interact", "aoe_highlight", "slot_stack",
        "plant_harvest", "shear_entity", "block_transform", "edible_effect",
    ];
    if id.namespace() == "tconstruct" && HOOKS.contains(&path) { Some(format!("tconstruct:{path}")) } else { None }
}

fn report_priority(cx: &Cx, hook_word: Option<&str>) -> Outcome {
    let filter = match hook_word {
        None => None,
        Some(word) => match known_hook(word) {
            Some(hook) => Some(hook),
            None => {
                let shown = word_id(word).map_or_else(|| word.to_owned(), |id| id.to_string());
                return Outcome::fail(tr_args("command.tconstruct.modifier_hook.not_found", &[&shown]));
            }
        },
    };
    let mut rows: Vec<(Identifier, i32, BTreeSet<&'static str>)> = Vec::new();
    for id in cx.modifiers.all_ids() {
        let Some(modifier) = cx.modifiers.get(&id) else { continue };
        let hooks = modifier_hooks(modifier);
        if let Some(filter) = &filter {
            if !hooks.contains(filter.as_str()) {
                continue;
            }
        }
        rows.push((id, modifier.priority, hooks));
    }
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let title = match &filter {
        Some(filter) => format!("Modifier Priorities for {filter}:"),
        None => "Modifier Priorities:".to_owned(),
    };
    let cells: Vec<Vec<String>> = rows
        .iter()
        .map(|(id, priority, hooks)| {
            let mut row = vec![id.to_string(), priority.to_string()];
            if filter.is_none() {
                row.push(hooks.iter().copied().collect::<Vec<_>>().join(", "));
            }
            row
        })
        .collect();
    let headers: &[&str] = if filter.is_none() { &["ID", "Priority", "Hooks"] } else { &["ID", "Priority"] };
    eprintln!("{title}\n{}", table(headers, &cells));
    Outcome::ok(rows.len() as i32, tr("command.tconstruct.modifier_priority"))
}

fn report_usage(cx: &Cx, filter: UsageFilter, args: &[String]) -> Outcome {
    // Recipe usage per modifier, from ModifierRecipeLookup's data.
    let mut recipe_slots: BTreeMap<Identifier, BTreeSet<String>> = BTreeMap::new();
    let mut recipe_slotless: BTreeSet<Identifier> = BTreeSet::new();
    if let Some(recipes) = cx.recipes {
        for recipe in recipes.all() {
            let mut names = BTreeSet::new();
            if let Some((name, _)) = &recipe.slots {
                names.insert(name.clone());
            }
            for level in &recipe.levels {
                if let Some((name, _)) = &level.slots {
                    names.insert(name.clone());
                }
            }
            if names.is_empty() {
                recipe_slotless.insert(recipe.result.clone());
            } else {
                recipe_slots.entry(recipe.result.clone()).or_default().extend(names);
            }
        }
    }
    let recipe_modifiers: BTreeSet<Identifier> = recipe_slots.keys().cloned().chain(recipe_slotless.iter().cloned()).collect();
    // Material traits: default traits plus the traits for each stat type the material has stats
    // for, where it defines unique traits for that type.
    let mut material_traits: BTreeSet<Identifier> = BTreeSet::new();
    for (material_id, traits) in &cx.materials.traits {
        for entry in traits.default.iter().flatten() {
            material_traits.insert(entry.id.clone());
        }
        if let Some(by_stat) = cx.materials.stats.get(material_id) {
            for stat in by_stat.keys() {
                if traits.has_unique(stat) {
                    for entry in traits.get(stat).iter().flatten() {
                        material_traits.insert(entry.id.clone());
                    }
                }
            }
        }
    }
    // Tool traits: the tool_traits hook of every tool definition, with empty materials.
    let mut tool_traits: BTreeSet<Identifier> = BTreeSet::new();
    let hook = Identifier::parse(TOOL_TRAITS_HOOK).expect("built-in hook id");
    for (_, data) in &cx.tools {
        if let Ok(entries) = data.hook_traits(&hook, &MaterialList::from_variants(Vec::new()), cx.materials) {
            for entry in entries {
                tool_traits.insert(entry.id);
            }
        }
    }
    // The filter selects the row set; a slot word rides along for the recipe filter.
    enum Selection {
        All,
        Recipe(Option<Option<String>>),
        MaterialTrait,
        ToolTrait,
        Unused,
    }
    let selection = match filter {
        UsageFilter::All => Selection::All,
        UsageFilter::RecipeBare => Selection::Recipe(None),
        UsageFilter::RecipeSlot => {
            let Some(word) = args.first() else { return failed() };
            if word == "slotless" {
                Selection::Recipe(Some(None))
            } else {
                match resolve_slot(cx, word) {
                    Ok(slot) => Selection::Recipe(Some(Some(slot))),
                    Err(outcome) => return outcome,
                }
            }
        }
        UsageFilter::MaterialTrait => Selection::MaterialTrait,
        UsageFilter::ToolTrait => Selection::ToolTrait,
        UsageFilter::Unused => Selection::Unused,
    };
    let all = cx.modifiers.all_ids();
    let keep: BTreeSet<Identifier> = match &selection {
        Selection::All => all.iter().cloned().collect(),
        Selection::Recipe(None) => recipe_modifiers.clone(),
        Selection::Recipe(Some(None)) => recipe_slotless.clone(),
        Selection::Recipe(Some(Some(name))) => recipe_slots
            .iter()
            .filter(|(_, slots)| slots.contains(name))
            .map(|(id, _)| id.clone())
            .collect(),
        Selection::MaterialTrait => material_traits.clone(),
        Selection::ToolTrait => tool_traits.clone(),
        Selection::Unused => all
            .iter()
            .filter(|id| !recipe_modifiers.contains(id) && !material_traits.contains(id) && !tool_traits.contains(id))
            .cloned()
            .collect(),
    };
    let slot_filter = matches!(selection, Selection::Recipe(Some(_)));
    let show_recipe = !matches!(selection, Selection::Unused) && !(matches!(selection, Selection::Recipe(_)) && slot_filter);
    let show_material = !matches!(selection, Selection::Unused | Selection::MaterialTrait);
    let show_tool = !matches!(selection, Selection::Unused | Selection::ToolTrait);
    let title = match &selection {
        Selection::All => "All modifiers:".to_owned(),
        Selection::Recipe(None) => "Recipe modifiers:".to_owned(),
        Selection::Recipe(Some(None)) => "Recipe modifiers: (slotless)".to_owned(),
        Selection::Recipe(Some(Some(name))) => format!("Recipe modifiers: ({name})"),
        Selection::MaterialTrait => "Material trait modifiers:".to_owned(),
        Selection::ToolTrait => "Tool trait modifiers:".to_owned(),
        Selection::Unused => "Unused modifiers:".to_owned(),
    };
    // Recipe cells list slot types in registration order: the core four first, then the rest.
    let recipe_cell = |id: &Identifier| -> String {
        let Some(slots) = recipe_slots.get(id) else {
            return if recipe_slotless.contains(id) { "slotless".to_owned() } else { String::new() };
        };
        let mut ordered: Vec<&str> = CORE_SLOTS.iter().copied().filter(|name| slots.contains(*name)).collect();
        ordered.extend(slots.iter().map(String::as_str).filter(|name| !CORE_SLOTS.contains(name)));
        ordered.join(", ")
    };
    let mut headers: Vec<&str> = vec!["ID"];
    if show_recipe {
        headers.push("Recipe");
    }
    if show_material {
        headers.push("material Trait");
    }
    if show_tool {
        headers.push("tool Trait");
    }
    let mut ids: Vec<Identifier> = keep.into_iter().collect();
    ids.sort();
    let cells: Vec<Vec<String>> = ids
        .iter()
        .map(|id| {
            let mut row = vec![id.to_string()];
            if show_recipe {
                row.push(recipe_cell(id));
            }
            if show_material {
                row.push(if material_traits.contains(id) { "Material trait".to_owned() } else { String::new() });
            }
            if show_tool {
                row.push(if tool_traits.contains(id) { "Tool trait".to_owned() } else { String::new() });
            }
            row
        })
        .collect();
    eprintln!("{title}\n{}", table(&headers, &cells));
    Outcome::ok(cells.len() as i32, tr("command.tconstruct.modifier_usage"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{MemoryPlayerOps, Translations, with_ops};
    use serde_json::json;

    #[test]
    fn java_float_rendering_matches_float_to_string() {
        assert_eq!(java_float(1.0), "1.0");
        assert_eq!(java_float(0.0), "0.0");
        assert_eq!(java_float(-0.0), "-0.0");
        assert_eq!(java_float(0.5), "0.5");
        assert_eq!(java_float(-3.25), "-3.25");
        assert_eq!(java_float(0.001), "0.001");
        assert_eq!(java_float(0.0001), "1.0E-4");
        assert_eq!(java_float(1_234_567.0), "1234567.0");
        assert_eq!(java_float(10_000_000.0), "1.0E7");
        assert_eq!(java_float(12_345_678.0), "1.2345678E7");
        assert_eq!(java_double(2.5), "2.5");
    }

    #[test]
    fn comma_bonus_and_percent_formats() {
        assert_eq!(comma(1_234_567.891), "1,234,567.89");
        assert_eq!(comma(1000.0), "1,000");
        assert_eq!(comma(0.125), "0.12");
        assert_eq!(comma(0.625), "0.62");
        assert_eq!(comma(-42.5), "-42.5");
        assert_eq!(comma_int(-1_234_567), "-1,234,567");
        assert_eq!(bonus_format(1.5), "+1.5");
        assert_eq!(bonus_format(0.0), "+0");
        assert_eq!(bonus_format(-0.25), "-0.25");
        assert_eq!(percent_boost(0.1), "+10%");
        assert_eq!(percent_boost(0.0), "+0%");
        assert_eq!(percent_boost(-0.05), "-5%");
        assert_eq!(percent_boost(1.0), "+100%");
    }

    #[test]
    fn roman_numerals_fall_back_to_literals() {
        assert_eq!(roman(1), "I");
        assert_eq!(roman(4), "IV");
        assert_eq!(roman(9), "IX");
        assert_eq!(roman(14), "XIV");
        assert_eq!(roman(2024), "MMXXIV");
        assert_eq!(roman(0), "0");
        assert_eq!(roman(-1), "-1");
    }

    #[test]
    fn snbt_scalars_parse_and_render() {
        assert_eq!(parse_snbt_scalar("5"), Some(Nbt::Int(5)));
        assert_eq!(parse_snbt_scalar("5b"), Some(Nbt::Byte(5)));
        assert_eq!(parse_snbt_scalar("-3s"), Some(Nbt::Short(-3)));
        assert_eq!(parse_snbt_scalar("5l"), Some(Nbt::Long(5)));
        assert_eq!(parse_snbt_scalar("2.5f"), Some(Nbt::Float(2.5)));
        assert_eq!(parse_snbt_scalar("2.5"), Some(Nbt::Double(2.5)));
        assert_eq!(parse_snbt_scalar("true"), Some(Nbt::Byte(1)));
        assert_eq!(parse_snbt_scalar("\"a b\""), Some(Nbt::String("a b".into())));
        assert_eq!(parse_snbt_scalar("'x'"), Some(Nbt::String("x".into())));
        assert_eq!(parse_snbt_scalar("{}"), None);
        assert_eq!(parse_snbt_scalar("[1,2]"), None);
        assert_eq!(parse_snbt_scalar("5x"), None);
        assert_eq!(snbt(&Nbt::Float(2.5)), "2.5f");
        assert_eq!(snbt(&Nbt::Int(5)), "5");
        assert_eq!(snbt(&Nbt::Long(5)), "5l");
        assert_eq!(snbt(&Nbt::String("a\"b".into())), "\"a\\\"b\"");
    }

    #[test]
    fn override_groups_store_neutral_removes_and_empty_groups_vanish() {
        let mut nbt = NbtCompound::new();
        set_float_override(&mut nbt, OVERRIDE_BONUS, "tconstruct:attack_damage", 2.5, 0.0);
        assert_eq!(override_value(&nbt, OVERRIDE_BONUS, "tconstruct:attack_damage", 0.0), 2.5);
        assert!(group_nonempty(&nbt, OVERRIDE_BONUS));
        set_float_override(&mut nbt, OVERRIDE_BONUS, "tconstruct:attack_damage", 0.0, 0.0);
        assert!(!group_nonempty(&nbt, OVERRIDE_BONUS));
        assert!(persistent_group(&nbt, OVERRIDE_BONUS).is_none());
        // Storing into one group leaves the other alone.
        set_float_override(&mut nbt, OVERRIDE_MULTIPLIER, "tconstruct:durability", 1.5, 1.0);
        assert_eq!(override_value(&nbt, OVERRIDE_MULTIPLIER, "tconstruct:durability", 1.0), 1.5);
        assert!(persistent_group(&nbt, OVERRIDE_BONUS).is_none());
    }

    #[test]
    fn read_stat_value_follows_each_stat_kind() {
        let attack = resolve_tool_stat("attack_damage", true).expect("float stat");
        assert_eq!(read_stat_value(attack, &Nbt::Double(2.5)), Some((Nbt::Float(2.5), true)));
        assert_eq!(read_stat_value(attack, &Nbt::Float(0.0)), Some((Nbt::Float(0.0), false)));
        assert_eq!(read_stat_value(attack, &Nbt::String("x".into())), None);
        let durability = resolve_tool_stat("durability", true).expect("integer stat");
        assert_eq!(read_stat_value(durability, &Nbt::Double(2.7)), Some((Nbt::Int(2), true)));
        let tier = resolve_tool_stat("harvest_tier", false).expect("tier stat");
        assert_eq!(
            read_stat_value(tier, &Nbt::String("iron".into())),
            Some((Nbt::String("minecraft:iron".into()), true))
        );
        assert_eq!(
            read_stat_value(tier, &Nbt::String("minecraft:wood".into())),
            Some((Nbt::String("minecraft:wood".into()), false))
        );
        assert_eq!(read_stat_value(tier, &Nbt::String("obsidian".into())), None);
        // The tier stat is not numeric, and unknown stats do not resolve.
        assert!(resolve_tool_stat("harvest_tier", true).is_err());
        assert!(resolve_tool_stat("tank_capacity", false).is_ok());
        assert!(resolve_tool_stat("max_energy", false).is_err());
        assert!(resolve_tool_stat("nonsense", false).is_err());
    }

    #[test]
    fn slot_counts_follow_tool_data_rules() {
        let mut persistent = NbtCompound::new();
        add_slot_count(&mut persistent, "upgrades", 2);
        add_slot_count(&mut persistent, "upgrades", -1);
        assert_eq!(modifier_entries::nbt_int(persistent.get("upgrades")), 1);
        add_slot_count(&mut persistent, "upgrades", -1);
        assert!(persistent.get("upgrades").is_none());
        add_slot_count(&mut persistent, "upgrades", 0);
        assert!(persistent.get("upgrades").is_none());
        // Free slots are persistent plus volatile.
        let mut nbt = NbtCompound::new();
        let mut persistent = NbtCompound::new();
        persistent.insert("defense".to_owned(), Nbt::Int(1));
        nbt.insert(tool_stack::TAG_PERSISTENT.to_owned(), Nbt::Compound(persistent));
        let mut volatile = NbtCompound::new();
        volatile.insert("defense".to_owned(), Nbt::Int(2));
        nbt.insert(tool_stack::TAG_VOLATILE.to_owned(), Nbt::Compound(volatile));
        assert_eq!(free_slots(&nbt, "defense"), 3);
        assert_eq!(free_slots(&nbt, "souls"), 0);
    }

    #[test]
    fn level_display_names() {
        let decode = |value: serde_json::Value| Modifier::decode(Identifier::parse("test:cmd_mod").unwrap(), &value).expect("modifier decodes");
        let plain = decode(json!({"modules": []}));
        assert_eq!(modifier_name(&plain, 1), "modifier.test.cmd_mod I");
        assert_eq!(modifier_name(&plain, 3), "modifier.test.cmd_mod III");
        let no_levels = decode(json!({"level_display": "tconstruct:no_levels", "modules": []}));
        assert_eq!(modifier_name(&no_levels, 5), "modifier.test.cmd_mod");
        let pluses = decode(json!({"level_display": "tconstruct:pluses", "modules": []}));
        assert_eq!(modifier_name(&pluses, 1), "modifier.test.cmd_mod");
        assert_eq!(modifier_name(&pluses, 3), "modifier.test.cmd_mod++");
        let unique = decode(json!({"level_display": {"type": "tconstruct:unique", "unique_until": 3, "first_unique": false}, "modules": []}));
        assert_eq!(modifier_name(&unique, 1), "modifier.test.cmd_mod");
        assert_eq!(modifier_name(&unique, 2), "modifier.test.cmd_mod II");
        assert_eq!(modifier_name(&unique, 4), "modifier.test.cmd_mod IV");
        Translations::register("en_us", [("modifier.test.cmd_mod.2".to_owned(), "Two".to_owned())]);
        assert_eq!(modifier_name(&unique, 2), "Two");
    }

    #[test]
    fn tables_pad_columns() {
        let text = table(&["ID", "Priority"], &[vec!["a".into(), "1".into()], vec!["long".into(), "22".into()]]);
        assert_eq!(text, "ID   | Priority\na    | 1\nlong | 22");
    }

    #[test]
    fn a_command_against_an_unreachable_target_fails_with_one_line() {
        let mut ops = MemoryPlayerOps::default();
        ops.selected_slot = 0;
        let player = ServerPlayer { uuid: 7, name: "Steve".into() };
        ops.stacks.insert(7, vec![Some(StackData {
            item: "tconstruct:pickaxe".into(),
            count: 1,
            damage: 0,
            max_damage: Some(60),
            custom_data: NbtCompound::new(),
            custom_model_data: None,
            enchantments: Default::default(),
        })]);
        // The selected-slot op the held-item commands rely on.
        assert_eq!(with_ops(&mut ops, || player.selected_slot()), Some(0));
        ops.selected_slot = 4;
        assert_eq!(with_ops(&mut ops, || player.selected_slot()), Some(4));
        let ctx = CommandContext { source: player, args: vec!["@a".into(), "add".into(), "tconstruct:haste".into()] };
        let value = with_ops(&mut ops, || run(&ctx, Route::Modifiers));
        assert_eq!(value, 0);
        assert_eq!(ops.told.len(), 1);
        assert_eq!(ops.told[0].0, 7);
        assert!(ops.told[0].1.contains("held_modifiable.failed"), "unexpected line: {}", ops.told[0].1);
    }
}
