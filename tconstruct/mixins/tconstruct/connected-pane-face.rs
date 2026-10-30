// The prepared render table marks TConstruct panes as (SameBlock, bars_tag).
// This pair is otherwise absent from the vanilla table; it lets this hook
// recognize them without global state or changing the visibility struct.
fn tconstruct_connected_pane_face(a: State, b: State, direction: Direction) -> std::ops::ControlFlow<bool> {
    use std::ops::ControlFlow::{Break, Continue};
    let marked = |state: State| state.definition.skip == SkipRendering::SameBlock
        && state.definition.bars_tag && state.connection_properties == 0b111100;
    if marked(a) {
        if a.block != b.block { return Break(false); }
        if matches!(direction, Direction::Up | Direction::Down) {
            // BetterPaneBlock: skip only when every arm is also in the neighbor.
            return Break(a.connections & !b.connections == 0);
        }
        let bit = 1 << direction as u8;
        let opposite = 1 << direction.opposite() as u8;
        return Break(a.connections & bit != 0 && b.connections & opposite != 0);
    }
    // bars_tag is an internal marker here, not a real iron-bars tag. Keep
    // ordinary vanilla bars from treating a marked glass pane as iron bars.
    if a.definition.skip == SkipRendering::Bars && marked(b) { return Break(false); }
    Continue(())
}
