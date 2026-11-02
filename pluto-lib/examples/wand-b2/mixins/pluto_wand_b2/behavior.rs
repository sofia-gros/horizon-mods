use pluto_api::item::{BlockTarget, ItemBehavior, ItemData, UseContext, UseOutcome};
struct Wand;
impl ItemBehavior for Wand {
    fn on_use_with_context(&self, context: &UseContext, data: &mut ItemData) -> UseOutcome {
        let outcome = self.on_use(data);
        if std::env::var_os("PLUTO_WAND_TRACE").is_some() {
            eprintln!(
                "wand-demo item actor={} hand={:?} outcome={:?}",
                context.actor.is_some(),
                context.hand,
                outcome
            );
        }
        outcome
    }
    fn on_use_on_block(
        &self,
        context: &UseContext,
        target: BlockTarget,
        _data: &mut ItemData,
    ) -> UseOutcome {
        if std::env::var_os("PLUTO_WAND_TRACE").is_some() {
            eprintln!(
                "wand-demo block actor={} hand={:?} target={:?} face={:?} outcome=Pass",
                context.actor.is_some(),
                context.hand,
                target.position,
                target.face
            );
        }
        UseOutcome::Pass
    }
    fn on_use(&self, data: &mut ItemData) -> UseOutcome {
        let charge = data.get("demo:charge").unwrap_or(0);
        // A negative test fixture exercises Pass without changing the stack.
        if charge < 0 {
            return UseOutcome::Pass;
        }
        if charge >= 20 {
            return UseOutcome::Cancel;
        }
        data.set("demo:charge", charge + 1);
        UseOutcome::Handled
    }
    fn tooltip(&self, data: &ItemData) -> Vec<String> {
        vec![format!(
            "Charge: {} / 20",
            data.get("demo:charge").unwrap_or(0)
        )]
    }
}
pub fn register() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        pluto_api::item::register("pluto_wand:wand", Wand).expect("unique demo behavior")
    });
}
