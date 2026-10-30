// Station layouts loaded from the active data packs, in their UI order.
use serde_json::Value;

// TinkerStationScreen uses six 18-pixel buttons with four-pixel gaps.
const BUTTON_COLUMNS: usize = 6;
const BUTTON_ROWS: usize = 8;
const BUTTON_SIZE: i32 = 18;
const BUTTON_STEP: i32 = 22;
const BUTTON_LEFT: i32 = -130;
const BUTTON_TOP: i32 = 9;

pub(super) fn button_position(index: usize) -> Option<(i32, i32)> {
    if index >= BUTTON_COLUMNS * BUTTON_ROWS { return None; }
    Some((BUTTON_LEFT + (index % BUTTON_COLUMNS) as i32 * BUTTON_STEP,
          BUTTON_TOP + (index / BUTTON_COLUMNS) as i32 * BUTTON_STEP))
}

pub(super) fn button_at(count: usize, x: i32, y: i32) -> Option<usize> {
    let (dx, dy) = (x - BUTTON_LEFT, y - BUTTON_TOP);
    if dx < 0 || dy < 0 || dx % BUTTON_STEP >= BUTTON_SIZE || dy % BUTTON_STEP >= BUTTON_SIZE { return None; }
    let (column, row) = ((dx / BUTTON_STEP) as usize, (dy / BUTTON_STEP) as usize);
    if column >= BUTTON_COLUMNS || row >= BUTTON_ROWS { return None; }
    let index = row * BUTTON_COLUMNS + column;
    (index < count).then_some(index)
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct LayoutSlot {
    pub x: i32,
    pub y: i32,
    pub icon: Option<String>,
    pub filter: Option<Value>,
    pub translation: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct StationLayout {
    pub id: String,
    pub icon: Value,
    pub sort_index: i32,
    pub translation: String,
    pub tool_slot: Option<LayoutSlot>,
    pub input_slots: Vec<LayoutSlot>,
}

impl StationLayout {
    pub(super) fn parse(id: String, bytes: &[u8]) -> Option<Self> {
        let value: Value = serde_json::from_slice(bytes).ok()?;
        let slot = |value: &Value| -> Option<LayoutSlot> {
            Some(LayoutSlot {
                x: i32::try_from(value.get("x")?.as_i64()?).ok()?,
                y: i32::try_from(value.get("y")?.as_i64()?).ok()?,
                icon: value.get("icon").and_then(Value::as_str).map(str::to_owned),
                filter: value.get("filter").cloned(),
                translation: value.get("translation_key").and_then(Value::as_str).map(str::to_owned),
            })
        };
        let tool_slot = match value.get("tool_slot") { Some(value) => Some(slot(value)?), None => None };
        let input_slots = value.get("input_slots")?.as_array()?.iter().map(slot).collect::<Option<Vec<_>>>()?;
        Some(Self {
            id,
            icon: value.get("icon")?.clone(),
            sort_index: value.get("sortIndex").and_then(Value::as_i64).and_then(|v| i32::try_from(v).ok()).unwrap_or(0),
            translation: value.get("translation_key")?.as_str()?.to_owned(),
            tool_slot,
            input_slots,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_layout_keeps_slot_coordinates_and_filters() {
        let bytes = br#"{"icon":{"item":"tconstruct:pickaxe"},"sortIndex":6,"translation_key":"item.tconstruct.pickaxe","input_slots":[{"filter":{"item":"tconstruct:pick_head"},"x":53,"y":22}]}"#;
        let layout = StationLayout::parse("tconstruct:pickaxe".into(), bytes).unwrap();
        assert_eq!(layout.sort_index, 6);
        assert_eq!((layout.input_slots[0].x, layout.input_slots[0].y), (53, 22));
        assert_eq!(layout.input_slots[0].filter.as_ref().unwrap()["item"], "tconstruct:pick_head");
    }

    #[test]
    fn button_hit_test_matches_source_grid_and_gaps() {
        assert_eq!(button_position(0), Some((-130, 9)));
        assert_eq!(button_position(5), Some((-20, 9)));
        assert_eq!(button_position(6), Some((-130, 31)));
        assert_eq!(button_at(7, -130, 9), Some(0));
        assert_eq!(button_at(7, -113, 26), Some(0));
        assert_eq!(button_at(7, -112, 9), None);
        assert_eq!(button_at(7, -130, 31), Some(6));
        assert_eq!(button_at(6, -130, 31), None);
    }
}
