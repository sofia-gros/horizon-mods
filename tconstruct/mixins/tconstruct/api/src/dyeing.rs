//! Dyeing travelers gear at the tinker station, the original's
//! `ArmorDyeingRecipe` (SlimeKnights, MIT; see LICENSE): a tool in the
//! dyeable tag plus only dyes on the table takes the vanilla averaged dye
//! color — the tool's current color counts as one more dye — stored as an
//! int in the persistent data under the `dyed` modifier's id, and gains
//! the `dyed` modifier if it lacks it. The recipe file itself
//! (`tools/modifiers/slotless/dyeing.json`) carries no fields; its mere
//! presence enables the recipe. Dye colors are vanilla's `DyeColor`
//! colors (the post-1.12 values), one color per input slot however tall
//! the stack, and one item is spent per slot.
use crate::{
    materials::Conditions,
    modifier_entries::List,
    tool_slot,
    tool_stack::{rebuild_stats, try_validate, Environment, TAG_MODIFIERS, TAG_PERSISTENT, TAG_UPGRADES},
};
use pluto_lib::{
    nbt::{Nbt, NbtCompound},
    Identifier, ItemSlot, ResourceManagerHelper, ResourceType, ServerLifecycleEvents,
};
use serde_json::Value;
use std::sync::{Arc, RwLock};

/// `TinkerTags.Items.DYEABLE`.
const DYEABLE_TAG: &str = "tconstruct:modifiable/dyeable";
/// `TinkerModifiers.dyed`.
const DYED: &str = "tconstruct:dyed";

/// The diffuse color of a vanilla dye item, as `(r, g, b)` channels.
fn dye_color(item: &Identifier) -> Option<(i32, i32, i32)> {
    if item.namespace() != "minecraft" {
        return None;
    }
    // Vanilla DyeColor colors (post-1.12), by dye item name.
    let packed = match item.path() {
        "white_dye" => 0xF9FFFE,
        "orange_dye" => 0xF9801D,
        "magenta_dye" => 0xC74EBD,
        "light_blue_dye" => 0x3AB3DA,
        "yellow_dye" => 0xFED83D,
        "lime_dye" => 0x80C71F,
        "pink_dye" => 0xF38BAA,
        "gray_dye" => 0x474F52,
        "light_gray_dye" => 0x9D9D97,
        "cyan_dye" => 0x169C9C,
        "purple_dye" => 0x8932B8,
        "blue_dye" => 0x3C44AA,
        "brown_dye" => 0x835432,
        "green_dye" => 0x5E7C16,
        "red_dye" => 0xB02E26,
        "black_dye" => 0x1D1D21,
        _ => return None,
    };
    Some(((packed >> 16) & 255, (packed >> 8) & 255, packed & 255))
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Recipes {
    /// A `tconstruct:armor_dyeing_modifier` file was loaded.
    pub present: bool,
    pub problems: Vec<String>,
    pub loaded: bool,
}

impl Recipes {
    pub fn load<C: Conditions>(files: &[(Identifier, Value)], conditions: &C) -> Self {
        let mut result = Self { loaded: true, ..Self::default() };
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            let outcome = (|| -> Result<bool, String> {
                let fields = value.as_object().ok_or("recipe must be an object")?;
                if fields.get("type").and_then(Value::as_str) != Some("tconstruct:armor_dyeing_modifier") {
                    return Ok(false);
                }
                if let Some(list) = fields.get("conditions") {
                    let parsed = list
                        .as_array()
                        .ok_or("conditions must be an array")?
                        .iter()
                        .map(|c| conditions.decode(c))
                        .collect::<Result<Vec<_>, _>>()?;
                    for condition in &parsed {
                        if !conditions.test(condition)? {
                            return Ok(false);
                        }
                    }
                }
                Ok(true)
            })();
            match outcome {
                Ok(true) => result.present = true,
                Ok(false) => {}
                Err(error) => result.problems.push(format!("{id}: {error}")),
            }
        }
        result
    }

    /// matches + getValidatedResult in one evaluation, when the recipe is
    /// present at all. None is no match.
    pub fn attempt(
        &self,
        tool: &ItemSlot,
        env: &Environment,
        raw: &[Option<(Identifier, u32)>],
        in_tag: &dyn Fn(&Identifier, &Identifier) -> bool,
    ) -> Option<Result<(ItemSlot, Vec<u32>), String>> {
        if !self.present {
            return None;
        }
        let dyeable = Identifier::parse(DYEABLE_TAG).ok()?;
        if !in_tag(&tool.item, &dyeable) {
            return None;
        }
        // Slots must be only dyes, and have at least 1 dye.
        let mut colors: Vec<(i32, i32, i32)> = Vec::new();
        let mut consumed: Vec<u32> = Vec::with_capacity(raw.len());
        for slot in raw {
            match slot {
                None => consumed.push(0),
                Some((item, _)) => {
                    colors.push(dye_color(item)?);
                    consumed.push(1);
                }
            }
        }
        if colors.is_empty() {
            return None;
        }
        let mut result = tool.clone();
        result.count = 1;
        let mut persistent = match result.custom_data.get(TAG_PERSISTENT) {
            Some(Nbt::Compound(data)) => data.clone(),
            _ => NbtCompound::new(),
        };
        let (mut nr, mut ng, mut nb) = (0i32, 0i32, 0i32);
        let mut brightness = 0i32;
        let mut count = 0i32;
        // The existing color counts as one more dye.
        if let Some(Nbt::Int(color)) = persistent.get(DYED) {
            let (r, g, b) = ((color >> 16) & 255, (color >> 8) & 255, color & 255);
            brightness = r.max(g).max(b);
            nr = r;
            ng = g;
            nb = b;
            count += 1;
        }
        for (r, g, b) in colors {
            brightness += r.max(g).max(b);
            nr += r;
            ng += g;
            nb += b;
            count += 1;
        }
        if count == 0 {
            return None;
        }
        nr /= count;
        ng /= count;
        nb /= count;
        let scaled = brightness as f32 / count as f32 / nr.max(ng).max(nb) as f32;
        nr = (nr as f32 * scaled) as i32;
        ng = (ng as f32 * scaled) as i32;
        nb = (nb as f32 * scaled) as i32;
        persistent.insert(DYED.into(), Nbt::Int((nr << 16) | (ng << 8) | nb));
        result.custom_data.insert(TAG_PERSISTENT.into(), Nbt::Compound(persistent));
        // Add the dyed modifier if missing, and rebuild like any modifier gain.
        let dyed = Identifier::parse(DYED).ok()?;
        let level = List::from_nbt(result.custom_data.get(TAG_UPGRADES)).level(&dyed)
            + List::from_nbt(result.custom_data.get(TAG_MODIFIERS)).level(&dyed);
        if level == 0 {
            let outcome = (|| -> Result<(), String> {
                let current = List::from_nbt(result.custom_data.get(TAG_UPGRADES));
                let upgrades = current.with_modifier(dyed.clone(), 1)?;
                result.custom_data.insert(TAG_UPGRADES.into(), upgrades.to_nbt());
                if !rebuild_stats(&mut result.custom_data, env)? {
                    return Err("the tool could not be rebuilt".into());
                }
                if let Some(error) = try_validate(&result.custom_data, env) {
                    return Err(error);
                }
                tool_slot::sync_visuals(&mut result, env)?;
                Ok(())
            })();
            if let Err(error) = outcome {
                return Some(Err(error));
            }
        }
        Some(Ok((result, consumed)))
    }
}

/// Reload listener, shaped like the other recipe listeners: it scans the
/// modifier recipe folder for the fieldless dyeing recipe type.
pub(crate) fn register<C: Conditions + 'static>(
    context: impl Fn() -> Result<C, String> + Send + Sync + 'static,
) -> Result<Arc<RwLock<Recipes>>, String> {
    let state = Arc::new(RwLock::new(Recipes::default()));
    let output = state.clone();
    ResourceManagerHelper::get(ResourceType::ServerData).register_json_reload_listener(
        Identifier::new("tconstruct", "dyeing_recipes").unwrap(),
        crate::modifier_recipes::FOLDER,
        move |files| {
            let context = context();
            let mut state = output.write().map_err(|_| "dyeing recipes poisoned")?;
            match context {
                Ok(context) => {
                    *state = Recipes::load(files, &context);
                    for problem in &state.problems {
                        eprintln!("Tinkers: {problem}");
                    }
                    Ok(())
                }
                Err(error) => {
                    *state = Recipes { problems: vec![error.clone()], ..Recipes::default() };
                    Err(error)
                }
            }
        },
    )?;
    let stopped = state.clone();
    ServerLifecycleEvents::SERVER_STOPPED.register(move |_| {
        *stopped.write().expect("dyeing recipes poisoned") = Recipes::default()
    });
    Ok(state)
}
