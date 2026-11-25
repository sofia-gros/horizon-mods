//! Material render info (SlimeKnights, MIT; see LICENSE): the client-side display data of
//! each material variant — display color, luminosity, sprite-generator stat coverage,
//! texture override, parent variant and sprite fallbacks — loaded from the pack's assets
//! tinkering/materials tree. Consumers read colors through `color` (armor tint, trim and
//! book displays); the sprite palettes themselves stay in the shipped files, as the port
//! bakes part sprites at import time.
//! Java revision: f4e77d43f39ba1047bf93ae816b7e1beaab514c2.
use crate::tool_materials::MaterialVariant;
use pluto_lib::{Identifier, ResourceManagerHelper, ResourceType};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::{OnceLock, RwLock};

pub(crate) const FOLDER: &str = "tinkering/materials";

/// One variant's render info (MaterialRenderInfo): only the fields a consumer reads;
/// a missing field is the original's default (no color override, luminosity 0).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RenderInfo {
    /// ARGB display color.
    pub color: Option<u32>,
    pub luminosity: i32,
    /// Variant whose render info this one extends.
    pub parent: Option<MaterialVariant>,
    /// Materials whose sprites serve when this material has none for a part.
    pub fallbacks: Vec<Identifier>,
    pub texture: Option<Identifier>,
    /// The generator's supported stat types.
    pub supported_stats: Vec<Identifier>,
}

impl RenderInfo {
    fn decode(value: &Value) -> Result<Self, String> {
        let fields = value.as_object().ok_or("render info must be an object")?;
        let color = match fields.get("color") {
            None => None,
            Some(Value::String(text)) => {
                let text = text.strip_prefix("0x").unwrap_or(text);
                let parsed = u32::from_str_radix(text, 16).map_err(|_| format!("invalid color {text}"))?;
                // A six-digit color is opaque; the shipped data writes all eight digits.
                Some(if text.len() <= 6 { parsed | 0xFF00_0000 } else { parsed })
            }
            Some(_) => return Err("color must be a hex string".into()),
        };
        let id = |value: &Value| -> Result<Identifier, String> {
            let text = value.as_str().ok_or("expected an identifier string")?;
            // Bare names live in the tconstruct namespace.
            if text.contains(':') { Identifier::parse(text).map_err(|e| e.to_string()) } else { Identifier::new("tconstruct", text).map_err(|e| e.to_string()) }
        };
        let fallbacks = match fields.get("fallbacks") {
            None => Vec::new(),
            Some(Value::Array(list)) => list.iter().map(&id).collect::<Result<_, _>>()?,
            Some(_) => return Err("fallbacks must be an array".into()),
        };
        let supported_stats = match fields.get("generator").and_then(|g| g.get("supported_stats")) {
            None => Vec::new(),
            Some(Value::Array(list)) => list.iter().map(&id).collect::<Result<_, _>>()?,
            Some(_) => return Err("generator supported_stats must be an array".into()),
        };
        Ok(Self {
            color,
            luminosity: match fields.get("luminosity") {
                None => 0,
                Some(Value::Number(n)) => crate::materials::number_int(&n.to_string()),
                Some(_) => return Err("luminosity must be a number".into()),
            },
            parent: match fields.get("parent") {
                None => None,
                Some(Value::String(text)) => Some(MaterialVariant::parse(text).ok_or("invalid parent variant")?),
                Some(_) => return Err("parent must be a material variant string".into()),
            },
            fallbacks,
            texture: match fields.get("texture") {
                None => None,
                Some(value) => Some(id(value)?),
            },
            supported_stats,
        })
    }
}

/// The variant a render-info file names: `iron` in the folder root is the material's base
/// variant, `copper/oxidized` below it is that material's `oxidized` flavor.
fn variant_of(id: &Identifier) -> Option<MaterialVariant> {
    let relative = id.path().strip_prefix("tinkering/materials/")?;
    match relative.split_once('/') {
        None => MaterialVariant::parse(&format!("{}:{}", id.namespace(), relative)),
        Some((material, flavor)) => MaterialVariant::parse(&format!("{}:{}#{}", id.namespace(), material, flavor)),
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct RenderInfos {
    /// Keyed by the variant's string form (MaterialVariant carries no ordering).
    infos: BTreeMap<String, RenderInfo>,
    pub problems: Vec<String>,
    pub loaded: bool,
}

impl RenderInfos {
    pub fn load(files: &[(Identifier, Value)]) -> Self {
        let mut result = Self { loaded: true, ..Self::default() };
        let mut sorted: Vec<&(Identifier, Value)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (id, value) in sorted {
            let outcome = (|| -> Result<(MaterialVariant, RenderInfo), String> {
                let variant = variant_of(id).ok_or("file is not a material variant")?;
                Ok((variant, RenderInfo::decode(value)?))
            })();
            match outcome {
                Ok((variant, info)) => { result.infos.insert(variant.to_string(), info); }
                Err(error) => result.problems.push(format!("{id}: {error}")),
            }
        }
        result
    }
    pub fn len(&self) -> usize { self.infos.len() }
    /// The variant's own render info, exactly as its file declares it.
    pub fn get(&self, variant: &MaterialVariant) -> Option<&RenderInfo> { self.infos.get(&variant.to_string()) }
    /// The display color for a variant: its own color, else its parent chain's, else the
    /// base material's, else the first fallback material that names one.
    pub fn color(&self, variant: &MaterialVariant) -> Option<u32> {
        // The variant's own info, then its parent chain.
        let mut current = variant.clone();
        for _ in 0..8 {
            let Some(info) = self.infos.get(&current.to_string()) else { break };
            if let Some(color) = info.color { return Some(color); }
            let Some(parent) = info.parent.clone() else { break };
            current = parent;
        }
        // The base material (flavor stripped), then its declared fallbacks.
        let base = variant.material_id().and_then(|id| MaterialVariant::parse(&id.to_string()))?;
        if base != *variant {
            if let Some(color) = self.infos.get(&base.to_string()).and_then(|info| info.color) { return Some(color); }
        }
        self.infos.get(&base.to_string()).and_then(|info| {
            info.fallbacks.iter().find_map(|fallback| self.infos.get(&fallback.to_string()).and_then(|info| info.color))
        })
    }
}

fn registry() -> &'static RwLock<RenderInfos> {
    static REGISTRY: OnceLock<RwLock<RenderInfos>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(RenderInfos::default()))
}

/// The loaded render infos (empty until the client resources land).
pub(crate) fn infos() -> &'static RwLock<RenderInfos> { registry() }

/// Installs the client-resources reload listener for the render info tree.
pub(crate) fn register() {
    let output = registry();
    ResourceManagerHelper::get(ResourceType::ClientResources)
        .register_json_reload_listener(
            Identifier::new("tconstruct", "material_render").unwrap(), FOLDER, move |files| {
                let mut state = output.write().map_err(|_| "material render infos poisoned")?;
                *state = RenderInfos::load(files);
                for problem in &state.problems { eprintln!("Tinkers: {problem}"); }
                Ok(())
            },
        )
        .expect("Tinkers material render listener registration failed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn id(text: &str) -> Identifier { Identifier::parse(text).unwrap() }
    fn variant(text: &str) -> MaterialVariant { MaterialVariant::parse(text).unwrap() }

    #[test]
    fn render_infos_load_colors_parents_and_fallbacks() {
        let files = vec![
            (id("tconstruct:tinkering/materials/iron"), json!({"color":"FFD8D8D8","fallbacks":["metal"],"generator":{"supported_stats":["tconstruct:head"],"transformer":{"type":"tconstruct:recolor_sprite"}}})),
            (id("tconstruct:tinkering/materials/metal"), json!({"color":"FF888888"})),
            (id("tconstruct:tinkering/materials/copper/oxidized"), json!({"color":"FF4FAB90","parent":"tconstruct:copper"})),
            (id("tconstruct:tinkering/materials/copper"), json!({"fallbacks":["metal"]})),
            (id("tconstruct:tinkering/materials/glass"), json!({"luminosity":4})),
            (id("tconstruct:tinkering/materials/broken"), json!({"color":"not-a-color"})),
        ];
        let infos = RenderInfos::load(&files);
        assert_eq!(infos.len(), 5, "{:?}", infos.problems);
        assert_eq!(infos.problems.len(), 1, "the bad color is a problem");
        let iron = infos.get(&variant("tconstruct:iron")).unwrap();
        assert_eq!(iron.color, Some(0xFFD8_D8D8));
        assert_eq!(iron.fallbacks, vec![id("tconstruct:metal")]);
        assert_eq!(iron.supported_stats, vec![id("tconstruct:head")]);
        // A subfolder file is the material's flavored variant.
        let oxidized = infos.get(&variant("tconstruct:copper#oxidized")).unwrap();
        assert_eq!(oxidized.color, Some(0xFF4F_AB90));
        assert_eq!(oxidized.parent, Some(variant("tconstruct:copper")));
        // Color resolution: own color wins; a colorless variant falls to its
        // base material's fallbacks; a variant with no file has no color.
        assert_eq!(infos.color(&variant("tconstruct:copper#oxidized")), Some(0xFF4F_AB90));
        assert_eq!(infos.color(&variant("tconstruct:copper")), Some(0xFF88_8888), "copper names no color; its fallback metal does");
        assert_eq!(infos.color(&variant("tconstruct:glass")), None, "glass declares luminosity only");
        assert_eq!(infos.color(&variant("tconstruct:unobtanium")), None);
    }
}
