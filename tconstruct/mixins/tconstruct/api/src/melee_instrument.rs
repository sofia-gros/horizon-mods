//! MeleeInstrumentModule, from SlimeKnights (MIT; see LICENSE): a successful melee hit plays an
//! instrument. The instrument is the one the tool's horn material names (its variant, with the first dot
//! turned into a colon, is the instrument's ID); when no material of the tool matches, a random instrument
//! of the module's tag is played instead. The sound is chosen here exactly as the original chooses it;
//! the hooks turn it into an `EntityAction::PlaySound` on the attacker, which the game plays where the
//! attacker stands when the action lands (a player's weapon hit, and a monster's melee hit).
use crate::{instrument_tags, melee_damage::TargetCondition, tool_materials::MaterialVariant};
use pluto_lib::Identifier;
use serde_json::{Map, Value};

/// An instrument of the pinned vanilla registry: its sound event and its range in blocks, from the pinned
/// vanilla instrument definitions (every goat horn has a range of 256). The original plays the sound at a
/// volume of the range over 16 and a pitch of 1.
fn instrument(id: &Identifier) -> Option<(&'static str, f32)> {
    Some(match id.to_string().as_str() {
        "minecraft:ponder_goat_horn" => ("minecraft:item.goat_horn.sound.0", 256.0),
        "minecraft:sing_goat_horn" => ("minecraft:item.goat_horn.sound.1", 256.0),
        "minecraft:seek_goat_horn" => ("minecraft:item.goat_horn.sound.2", 256.0),
        "minecraft:feel_goat_horn" => ("minecraft:item.goat_horn.sound.3", 256.0),
        "minecraft:admire_goat_horn" => ("minecraft:item.goat_horn.sound.4", 256.0),
        "minecraft:call_goat_horn" => ("minecraft:item.goat_horn.sound.5", 256.0),
        "minecraft:yearn_goat_horn" => ("minecraft:item.goat_horn.sound.6", 256.0),
        "minecraft:dream_goat_horn" => ("minecraft:item.goat_horn.sound.7", 256.0),
        _ => return None,
    })
}

/// The sound a hit plays: the sound event, and the volume to play it at.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Sound { pub event: String, pub volume: f32 }

/// MeleeInstrumentModule: plays an instrument on a melee hit its attacker and target predicates accept.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MeleeInstrument {
    material: Option<Identifier>,
    tag: Identifier,
    attacker: TargetCondition,
    target: TargetCondition,
}

impl MeleeInstrument {
    /// `None` for an attacker or target predicate this port cannot evaluate.
    pub fn decode(fields: &Map<String, Value>) -> Result<Option<Self>, String> {
        let material = match fields.get("instrument_material").filter(|value| !value.is_null()) {
            None => None,
            Some(value) => Some(Identifier::parse(value.as_str().ok_or("instrument_material must be an ID string")?).map_err(|e| e.to_string())?),
        };
        let tag = Identifier::parse(fields.get("instrument_tag").and_then(Value::as_str).ok_or("instrument_tag missing")?).map_err(|e| e.to_string())?;
        let Some(attacker) = TargetCondition::decode_for("attacker", fields.get("attacker"))? else { return Ok(None); };
        let Some(target) = TargetCondition::decode_for("target", fields.get("target"))? else { return Ok(None); };
        Ok(Some(Self { material, tag, attacker, target }))
    }

    /// The sound for one melee hit, or None when the hit plays nothing: the module's condition failed, a
    /// predicate rejected the attacker or the target, or no instrument could be found. `materials` are the
    /// tool's materials; `attacker_in_tag` and `target_in_tag` answer the entity type tags and the pseudo
    /// state tags of the hit for each side; `roll` picks the tag's instrument when no material names one,
    /// uniform in [0, 1).
    pub fn sound(&self, condition: bool, materials: &[MaterialVariant], attacker_in_tag: &dyn Fn(&Identifier) -> bool, target_in_tag: &dyn Fn(&Identifier) -> bool, roll: f32) -> Option<Sound> {
        if !condition || !self.attacker.matches(attacker_in_tag) || !self.target.matches(target_in_tag) { return None; }
        // The first material of the named kind names the instrument through its variant; a variant that
        // names no instrument of the registry leaves the choice to the tag, as a missing material does.
        let mut chosen = None;
        if let Some(material) = &self.material {
            for variant in materials {
                if variant.material_id().as_ref() != Some(material) { continue; }
                let text = variant.to_string();
                let raw = text.split_once('#').map(|(_, variant)| variant).unwrap_or("");
                let converted = match raw.find('.') {
                    Some(index) => { let mut converted = raw.to_owned(); converted.replace_range(index..=index, ":"); converted }
                    None => raw.to_owned(),
                };
                if let Ok(id) = Identifier::parse(&converted) { if instrument(&id).is_some() { chosen = Some(id); } }
                break;
            }
        }
        let id = match chosen {
            Some(id) => id,
            None => {
                let members = instrument_tags::TAGS.iter().find(|(tag, _)| *tag == self.tag.to_string()).map(|(_, members)| *members)?;
                if members.is_empty() { return None; }
                let index = ((roll * members.len() as f32) as usize).min(members.len() - 1);
                Identifier::parse(members[index]).ok()?
            }
        };
        let (event, range) = instrument(&id)?;
        Some(Sound { event: event.to_owned(), volume: range / 16.0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The shipped ram attack module.
    fn module() -> MeleeInstrument {
        MeleeInstrument::decode(json!({"attacker": "mantle:sprinting", "instrument_material": "tconstruct:horn", "instrument_tag": "minecraft:regular_goat_horns"}).as_object().unwrap()).unwrap().unwrap()
    }
    fn sprinting(tag: &Identifier) -> bool { tag.to_string() == "tconstruct_state:attacker/sprinting" }
    fn nothing(_: &Identifier) -> bool { false }
    fn variant(text: &str) -> MaterialVariant { MaterialVariant::parse(text).unwrap() }

    #[test]
    fn the_horn_materials_variant_names_the_instrument() {
        let materials = [variant("tconstruct:wood"), variant("tconstruct:horn#minecraft.sing_goat_horn")];
        let sound = module().sound(true, &materials, &sprinting, &nothing, 0.0).unwrap();
        assert_eq!(sound, Sound { event: "minecraft:item.goat_horn.sound.1".into(), volume: 16.0 });
        // A horn without an instrument variant falls back to the tag.
        let plain = [variant("tconstruct:horn")];
        assert_eq!(module().sound(true, &plain, &sprinting, &nothing, 0.0).unwrap().event, "minecraft:item.goat_horn.sound.0");
    }

    #[test]
    fn without_a_matching_material_a_random_instrument_of_the_tag_plays() {
        let materials = [variant("tconstruct:wood")];
        // regular_goat_horns flattens to ponder, sing, seek, feel in the pinned vanilla tag.
        assert_eq!(module().sound(true, &materials, &sprinting, &nothing, 0.0).unwrap().event, "minecraft:item.goat_horn.sound.0");
        assert_eq!(module().sound(true, &materials, &sprinting, &nothing, 0.99).unwrap().event, "minecraft:item.goat_horn.sound.3");
    }

    #[test]
    fn a_hit_the_attacker_predicate_rejects_plays_nothing() {
        let materials = [variant("tconstruct:horn#minecraft.sing_goat_horn")];
        // The shipped module asks for a sprinting attacker; a standing one is silent, as is a failed condition.
        assert_eq!(module().sound(true, &materials, &nothing, &nothing, 0.0), None);
        assert_eq!(module().sound(false, &materials, &sprinting, &nothing, 0.0), None);
    }

    #[test]
    fn a_predicate_this_port_cannot_evaluate_leaves_the_module_unbound() {
        let fields = json!({"attacker": {"type": "mantle:entity_type", "entity_type": "minecraft:player"}, "instrument_tag": "minecraft:regular_goat_horns"});
        assert!(MeleeInstrument::decode(fields.as_object().unwrap()).unwrap().is_none());
    }
}
