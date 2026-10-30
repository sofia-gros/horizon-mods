// Source item-model layers and material indices for dynamic tool rendering.
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ToolLayer {
    pub texture: String,
    pub material_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ToolModel {
    pub parent: String,
    pub layers: Vec<ToolLayer>,
    pub overrides: Vec<Value>,
    pub modifier_maps: Vec<String>,
}

impl ToolModel {
    pub(super) fn parse(bytes: &[u8]) -> Option<Self> {
        let value: Value = serde_json::from_slice(bytes).ok()?;
        if value.get("loader")?.as_str()? != "tconstruct:tool" { return None; }
        let textures = value.get("textures")?.as_object()?;
        let layers = value.get("parts")?.as_array()?.iter().map(|part| {
            let name = part.get("name")?.as_str()?;
            Some(ToolLayer {
                texture: textures.get(name)?.as_str()?.to_owned(),
                material_index: usize::try_from(part.get("index")?.as_u64()?).ok()?,
            })
        }).collect::<Option<Vec<_>>>()?;
        let modifier_maps = match value.get("modifier_maps") {
            Some(value) => value.as_array()?.iter().map(|v| v.as_str().map(str::to_owned)).collect::<Option<Vec<_>>>()?,
            None => Vec::new(),
        };
        Some(Self {
            parent: value.get("parent")?.as_str()?.to_owned(),
            layers,
            overrides: value.get("overrides").and_then(Value::as_array).cloned().unwrap_or_default(),
            modifier_maps,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_source_layer_order_and_material_indices() {
        let value = br#"{"loader":"tconstruct:tool","parent":"forge:item/default-tool","textures":{"head":"tconstruct:item/tool/pickaxe/head","handle":"tconstruct:item/tool/pickaxe/handle"},"parts":[{"name":"handle","index":1},{"name":"head","index":0}]}"#;
        let model = ToolModel::parse(value).unwrap();
        assert_eq!(model.layers.iter().map(|v| v.material_index).collect::<Vec<_>>(), [1, 0]);
        assert_eq!(model.layers[1].texture, "tconstruct:item/tool/pickaxe/head");
    }
}
