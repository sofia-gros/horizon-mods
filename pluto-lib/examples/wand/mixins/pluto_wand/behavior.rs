use pluto_item_api::{ItemBehavior, ItemData, UseOutcome};
struct Wand;
impl ItemBehavior for Wand {
    fn on_use(&self, data: &mut ItemData) -> UseOutcome {
        let charge = data.get("demo:charge").unwrap_or(0);
        // A negative test fixture exercises Pass without changing the stack.
        if charge < 0 { return UseOutcome::Pass; }
        if charge >= 20 { return UseOutcome::Cancel; }
        data.set("demo:charge", charge + 1);
        UseOutcome::Handled
    }
    fn tooltip(&self, data: &ItemData) -> Vec<String> {
        vec![format!("Charge: {} / 20", data.get("demo:charge").unwrap_or(0))]
    }
}
pub fn register() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| pluto_item_api::register("pluto_wand:wand", Wand).expect("unique demo behavior"));
}
