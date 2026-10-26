//! Strict loader for the explicitly supported material/tool subset.
use crate::{
    stats::{Accumulator, Stats},
    tool::{Evaluation, ModifierLevel, ToolState},
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

type Result<T> = std::result::Result<T, String>;
const DATA: &str = "embedded TConstruct snapshot";
const PICKAXE: &str = include_str!(
    "../third_party/tconstruct/src/generated/resources/data/tconstruct/tinkering/tool_definitions/pickaxe.json"
);

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialDefinition {
    pub craftable: bool,
    pub hidden: bool,
    #[serde(rename = "sortOrder")]
    pub sort_order: u32,
    pub tier: u32,
}

#[derive(Clone, Debug)]
pub struct Material {
    pub definition: MaterialDefinition,
    stats: BTreeMap<String, Value>,
    traits: Traits,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Traits {
    default: Vec<Trait>,
    #[serde(default, rename = "perStat")]
    per_stat: BTreeMap<String, Vec<Trait>>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Trait {
    name: String,
    level: u32,
}

#[derive(Debug)]
pub struct Catalog {
    materials: BTreeMap<String, Material>,
    tool_modules: Vec<Value>,
    modifiers: BTreeMap<String, Vec<Value>>,
}

fn parse(text: &str, path: &str) -> Result<Value> {
    serde_json::from_str(text).map_err(|e| format!("{path}: {e}"))
}

pub(crate) fn fields(value: &Value, allowed: &[&str], path: &str) -> Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{path}: expected object"))?;
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(format!("{path}: unsupported field {key}"));
    }
    Ok(())
}

pub(crate) fn number(value: &Value, path: &str) -> Result<f32> {
    let x = value
        .as_f64()
        .ok_or_else(|| format!("{path}: expected number"))? as f32;
    if !x.is_finite() {
        return Err(format!("{path}: non-finite number"));
    }
    Ok(x)
}

pub(crate) fn text<'a>(value: &'a Value, path: &str) -> Result<&'a str> {
    value
        .as_str()
        .ok_or_else(|| format!("{path}: expected string"))
}

fn array<'a>(value: &'a Value, path: &str) -> Result<&'a Vec<Value>> {
    value
        .as_array()
        .ok_or_else(|| format!("{path}: expected array"))
}

impl Catalog {
    pub fn embedded() -> Result<Self> {
        macro_rules! snapshot {
            ($path:literal) => {
                include_str!(concat!(
                    "../third_party/tconstruct/src/generated/resources/data/tconstruct/",
                    $path
                ))
            };
        }
        Self::from_documents(
            &[
                (
                    "tconstruct:wood",
                    snapshot!("tinkering/materials/definition/wood.json"),
                    snapshot!("tinkering/materials/stats/wood.json"),
                    snapshot!("tinkering/materials/traits/wood.json"),
                ),
                (
                    "tconstruct:flint",
                    snapshot!("tinkering/materials/definition/flint.json"),
                    snapshot!("tinkering/materials/stats/flint.json"),
                    snapshot!("tinkering/materials/traits/flint.json"),
                ),
            ],
            snapshot!("tinkering/tool_definitions/pickaxe.json"),
            &[
                (
                    "tconstruct:cultivated",
                    snapshot!("tinkering/modifiers/cultivated.json"),
                ),
                (
                    "tconstruct:jagged",
                    snapshot!("tinkering/modifiers/jagged.json"),
                ),
                (
                    "tconstruct:pierce",
                    snapshot!("tinkering/modifiers/pierce.json"),
                ),
            ],
        )
    }

    /// Only the pickaxe's head/handle/binding stats are interpreted. Other stat
    /// entries stay in the immutable source snapshot and are not advertised.
    pub fn from_documents(
        materials: &[(&str, &str, &str, &str)],
        tool: &str,
        modifiers: &[(&str, &str)],
    ) -> Result<Self> {
        let mut catalog = Self {
            materials: BTreeMap::new(),
            tool_modules: Vec::new(),
            modifiers: BTreeMap::new(),
        };
        for (id, definition, stats, traits) in materials {
            if catalog.materials.contains_key(*id) {
                return Err(format!("duplicate material {id}"));
            }
            let definition =
                serde_json::from_str(definition).map_err(|e| format!("{id}/definition: {e}"))?;
            let raw = parse(stats, id)?;
            fields(&raw, &["stats"], id)?;
            let stats: BTreeMap<String, Value> = serde_json::from_value(raw["stats"].clone())
                .map_err(|e| format!("{id}/stats: {e}"))?;
            for kind in ["head", "handle", "binding"] {
                let key = format!("tconstruct:{kind}");
                let entry = stats
                    .get(&key)
                    .ok_or_else(|| format!("{id}: missing {key}"))?;
                let allowed: &[&str] = match kind {
                    "head" => &["durability", "melee_attack", "mining_speed", "mining_tier"],
                    "handle" => &["durability", "melee_damage", "melee_speed", "mining_speed"],
                    _ => &[],
                };
                fields(entry, allowed, &format!("{id}/{kind}"))?;
                for key in allowed {
                    if *key == "mining_tier" {
                        tier_index(text(&entry[*key], id)?)?;
                    } else {
                        number(&entry[*key], &format!("{id}/{kind}/{key}"))?;
                    }
                }
            }
            let traits: Traits =
                serde_json::from_str(traits).map_err(|e| format!("{id}/traits: {e}"))?;
            catalog.materials.insert(
                (*id).into(),
                Material {
                    definition,
                    stats,
                    traits,
                },
            );
        }
        let raw = parse(tool, "pickaxe")?;
        fields(&raw, &["modules"], "pickaxe")?;
        catalog.tool_modules = array(&raw["modules"], "pickaxe/modules")?.clone();
        let mut seen = std::collections::BTreeSet::new();
        for module in &catalog.tool_modules {
            let kind = text(&module["type"], "pickaxe/module/type")?;
            if !seen.insert(kind.to_owned()) {
                return Err(format!("pickaxe: duplicate module {kind}"));
            }
            let keys: &[&str] = match kind {
                "tconstruct:part_stats" => &["type", "parts", "primary_part"],
                "tconstruct:default_materials" => &["type", "materials"],
                "tconstruct:base_stats" => &["type", "stats"],
                "tconstruct:modifier_slots" => &["type", "slots"],
                "tconstruct:traits" => &["type", "traits"],
                "tconstruct:tool_actions" => &["type", "tool_actions"],
                "tconstruct:is_effective" => &["type", "predicate_type", "tag"],
                "tconstruct:box_aoe" => &["type", "expansion_direction", "expansions"],
                "tconstruct:fixed_material_name" => &["type", "index"],
                _ => return Err(format!("pickaxe: unsupported module {kind}")),
            };
            fields(module, keys, kind)?;
            // Non-calculation modules are preserved for later host integration.
            // Accept only the pinned configuration, never silently ignore edits.
            if ![
                "tconstruct:base_stats",
                "tconstruct:traits",
                "tconstruct:part_stats",
            ]
            .contains(&kind)
            {
                let pinned = parse(PICKAXE, "pinned pickaxe")?;
                let same = array(&pinned["modules"], DATA)?.iter().any(|v| v == module);
                if !same {
                    return Err(format!("unsupported configuration for {kind}"));
                }
            }
        }
        if seen.len() != 9 {
            return Err("pickaxe requires all nine pinned modules".into());
        }
        let expected_parts = serde_json::json!([
            "tconstruct:pick_head",
            "tconstruct:tool_handle",
            "tconstruct:tool_binding"
        ]);
        let parts = catalog.module("tconstruct:part_stats")?;
        if parts["parts"] != expected_parts || parts["primary_part"] != 0 {
            return Err("unsupported pickaxe part configuration".into());
        }
        for (id, source) in modifiers {
            if catalog.modifiers.contains_key(*id) {
                return Err(format!("duplicate modifier {id}"));
            }
            let raw = parse(source, id)?;
            fields(&raw, &["level_display", "modules"], id)?;
            let modules = array(&raw["modules"], id)?.clone();
            for module in &modules {
                crate::tool::validate_modifier_module(module)?;
            }
            catalog.modifiers.insert((*id).into(), modules);
        }
        // Close dependencies only for the supported stat types, not ammo traits.
        for material in catalog.materials.values() {
            for stat in ["tconstruct:head", "tconstruct:handle", "tconstruct:binding"] {
                for entry in material
                    .traits
                    .per_stat
                    .get(stat)
                    .unwrap_or(&material.traits.default)
                {
                    catalog.require_modifier(&entry.name, entry.level)?;
                }
            }
        }
        for entry in array(
            &catalog.module("tconstruct:traits")?["traits"],
            "tool traits",
        )? {
            let entry: Trait = serde_json::from_value(entry.clone()).map_err(|e| e.to_string())?;
            catalog.require_modifier(&entry.name, entry.level)?;
        }
        Ok(catalog)
    }

    fn module(&self, name: &str) -> Result<&Value> {
        self.tool_modules
            .iter()
            .find(|v| v["type"] == name)
            .ok_or_else(|| format!("missing module {name}"))
    }

    fn require_modifier(&self, id: &str, level: u32) -> Result<()> {
        if level == 0 || level > 255 {
            return Err(format!("invalid modifier level {level}"));
        }
        if !self.modifiers.contains_key(id) {
            return Err(format!("unsupported modifier {id}"));
        }
        Ok(())
    }

    pub(crate) fn modifier_modules(&self, id: &str) -> Result<&[Value]> {
        self.modifiers
            .get(id)
            .map(Vec::as_slice)
            .ok_or_else(|| format!("unsupported modifier {id}"))
    }

    pub fn material(&self, id: &str) -> Result<&Material> {
        let id = if id.contains(':') {
            id.to_owned()
        } else {
            format!("tconstruct:{id}")
        };
        self.materials
            .get(&id)
            .ok_or_else(|| format!("unsupported material {id}"))
    }

    pub fn evaluate(&self, state: &ToolState) -> Result<Evaluation> {
        state.validate()?;
        let mut durability = Accumulator::new(1.0);
        let mut mining = Accumulator::new(1.0);
        let mut attack = Accumulator::new(0.0);
        let mut speed = Accumulator::new(1.0);
        let mut tier = "minecraft:wood".to_owned();
        let mut modifiers = Vec::<ModifierLevel>::new();
        let mut add_trait = |id: &str, level: u32| -> Result<()> {
            self.require_modifier(id, level)?;
            if let Some(entry) = modifiers.iter_mut().find(|m| m.id == id) {
                entry.level = entry
                    .level
                    .checked_add(level)
                    .filter(|v| *v <= 255)
                    .ok_or("modifier level overflow")?;
            } else {
                modifiers.push(ModifierLevel {
                    id: id.into(),
                    level,
                });
            }
            Ok(())
        };
        // The initial core has no upgrades. Tool traits precede material traits.
        for entry in array(&self.module("tconstruct:traits")?["traits"], DATA)? {
            let entry: Trait = serde_json::from_value(entry.clone()).map_err(|e| e.to_string())?;
            add_trait(&entry.name, entry.level)?;
        }
        for (index, kind) in ["head", "handle", "binding"].into_iter().enumerate() {
            let material = self.material(&state.materials[index])?;
            let key = format!("tconstruct:{kind}");
            let stats = &material.stats[&key];
            if kind == "head" {
                durability.update(number(&stats["durability"], DATA)?);
                attack.update(number(&stats["melee_attack"], DATA)?);
                mining.update(number(&stats["mining_speed"], DATA)?);
                tier = text(&stats["mining_tier"], DATA)?.into();
            } else if kind == "handle" {
                for (acc, key) in [
                    (&mut durability, "durability"),
                    (&mut attack, "melee_damage"),
                    (&mut speed, "melee_speed"),
                    (&mut mining, "mining_speed"),
                ] {
                    acc.percent(f64::from(number(&stats[key], DATA)?));
                }
            }
            for entry in material
                .traits
                .per_stat
                .get(&key)
                .unwrap_or(&material.traits.default)
            {
                add_trait(&entry.name, entry.level)?;
            }
        }
        let base = &self.module("tconstruct:base_stats")?["stats"];
        fields(
            base,
            &["tconstruct:attack_damage", "tconstruct:attack_speed"],
            "base stats",
        )?;
        // SetStatsModule uses update, which adds after cancelling the default base.
        attack.update(number(&base["tconstruct:attack_damage"], DATA)?);
        speed.update(number(&base["tconstruct:attack_speed"], DATA)?);
        // jagged is priority 75 (conditional percent); other selected traits use 100.
        modifiers.sort_by_key(|entry| {
            if entry.id == "tconstruct:jagged" {
                1
            } else {
                0
            }
        });
        for entry in &modifiers {
            for module in self.modifier_modules(&entry.id)? {
                if module["type"] == "tconstruct:stat_boost" {
                    let amount =
                        f64::from(number(&module["each_level"], DATA)?) * f64::from(entry.level);
                    match text(&module["stat"], DATA)? {
                        "tconstruct:attack_damage" => attack.add(amount),
                        other => return Err(format!("unsupported stat boost {other}")),
                    }
                }
            }
        }
        let stats = Stats {
            durability: durability.build().clamp(1.0, i32::MAX as f32),
            mining_speed: mining.build().clamp(0.1, 2048.0),
            attack_damage: attack.build().clamp(0.0, 2048.0),
            attack_speed: speed.build().clamp(0.0, 1024.0),
            harvest_tier: tier,
        };
        stats.validate()?;
        if state.damage > stats.durability()?
            || state.broken != (state.damage == stats.durability()?)
        {
            return Err(
                "tool damage/broken state does not match current definition; migrate explicitly"
                    .into(),
            );
        }
        Ok(Evaluation { stats, modifiers })
    }
}

fn tier_index(id: &str) -> Result<usize> {
    [
        "minecraft:wood",
        "minecraft:stone",
        "minecraft:iron",
        "minecraft:diamond",
        "minecraft:netherite",
    ]
    .iter()
    .position(|v| *v == id)
    .ok_or_else(|| format!("unsupported harvest tier {id}"))
}
