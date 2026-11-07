//! Counterpart of Fabric's `fabric-item-api-v1` (so far: item tooltips).
//!
//! Fabric's `ItemTooltipCallback` lets a callback read and edit the tooltip's `List<Text>`. PlutoLib can only add lines for now: it cannot
//! show a callback the lines already there (see the support matrix).

use std::cell::RefCell;

use super::base::Event;

/// One tooltip line to add.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TooltipLine {
    pub text: String,
    /// The line colour as `0xRRGGBBAA`.
    pub color: u32,
}

impl TooltipLine {
    /// A grey line, like most tooltip text.
    pub fn grey(text: impl Into<String>) -> Self {
        Self { text: text.into(), color: 0xAAAA_AAFF }
    }
}

/// The tooltip being built for an item stack.
pub struct ItemTooltipContext {
    /// The item id, such as `tconstruct:flint_pickaxe`.
    pub item: String,
    pub count: u32,
    /// Lines callbacks want added after the game's own.
    pub added: RefCell<Vec<TooltipLine>>,
}

static ITEM_TOOLTIP: Event<ItemTooltipContext, ()> = Event::void();

/// Fabric: `ItemTooltipCallback`.
pub struct ItemTooltipCallback;

impl ItemTooltipCallback {
    /// Fabric: `ItemTooltipCallback.EVENT`.
    pub const EVENT: &'static Event<ItemTooltipContext, ()> = &ITEM_TOOLTIP;
}

/// Runs the tooltip callbacks and returns the lines they added.
pub(crate) fn item_tooltip(item: &str, count: u32) -> Vec<TooltipLine> {
    let context = ItemTooltipContext { item: item.to_owned(), count, added: RefCell::new(Vec::new()) };
    ITEM_TOOLTIP.invoke(&context);
    context.added.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callbacks_add_lines_in_registration_order_for_the_item_they_choose() {
        ItemTooltipCallback::EVENT.register(|c| {
            if c.item == "tconstruct:flint_pickaxe" {
                c.added.borrow_mut().push(TooltipLine::grey("Tinkers tool"));
            }
        });
        ItemTooltipCallback::EVENT.register(|c| c.added.borrow_mut().push(TooltipLine::grey(format!("x{}", c.count))));
        let lines = item_tooltip("tconstruct:flint_pickaxe", 1);
        assert_eq!(lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(), ["Tinkers tool", "x1"]);
        assert_eq!(item_tooltip("minecraft:stone", 3).len(), 1);
    }
}
