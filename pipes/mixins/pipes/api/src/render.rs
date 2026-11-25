//! Drawing what rides the pipes: the stacks in transit inside an item pipe, and the fluid a
//! fluid pipe holds. Both are block entity renderers over PlutoLib's renderer surface
//! (`BlockEntityRendererRegistry`); a renderer only reads the data the server sent
//! (`BlockEntityType::sync_data`, which the fluid pipe types already had and the item pipe
//! types now have too) and draws under the block's own pose, so nothing here can change
//! transport: same stacks, same timing, same arrivals — now visible.
//!
//! A stack in transit already carries everything the drawing needs (`item_pipe::Transit`):
//! the side it came in (`from`), the side it leaves by (`to`) and the ticks it still has
//! (`ticks`, counting down from `TRAVEL_TICKS`). Its place between the two faces is the
//! elapsed share of the ride, with the frame's partial tick smoothing the steps, along the
//! path every such item takes: in through the entry face to the centre, then out to the
//! exit face. A stack whose delivery was refused starts a new leg in the core's own
//! bookkeeping (its `ticks` reset, its exit possibly changed), so it is drawn starting a
//! new leg too — the picture follows the state, never the other way round.
use crate::common::{EXTRACTION_PIPE, FLUID_EXTRACTION_PIPE, FLUID_PIPE, ITEM_PIPE, NAMESPACE};
use crate::item_pipe::{PipeState, Transit, TRAVEL_TICKS};
use pluto_lib::{
    BlockEntityCanvas, BlockEntityRender, BlockEntityRendererRegistry, Direction, FluidStack, Identifier,
    ItemDisplayContext, NbtCompound, NbtCompoundExt,
};

/// The scale a riding stack is drawn at: the pipe's channel is 6/16 of a block wide (the
/// core model's 5..11), so a stack drawn at 0.30 of its fixed-frame size rides inside it.
const ITEM_SCALE: f32 = 0.30;
/// The scale of the fluid drawn in a fluid pipe: a little inside the same 6/16 channel.
const FLUID_SCALE: f32 = 0.34;

/// How far a stack in transit has ridden, 0 at its entry face to 1 at its exit face. The
/// core delivers on the tick after `ticks` reaches 0, so the whole ride spans
/// `TRAVEL_TICKS + 1` tick steps; `partial_tick` is the frame's place inside the next one.
pub fn transit_progress(transit: &Transit, partial_tick: f32) -> f32 {
    let elapsed = (TRAVEL_TICKS - transit.ticks).max(0) as f32;
    ((elapsed + partial_tick) / (TRAVEL_TICKS + 1) as f32).clamp(0.0, 1.0)
}

/// Where a stack in transit is drawn, in blocks from the pipe's lower north-west corner:
/// from the middle of its entry face to the centre for the first half of the ride, then
/// from the centre to the middle of its exit face.
pub fn transit_position(transit: &Transit, partial_tick: f32) -> [f32; 3] {
    let centre = [0.5, 0.5, 0.5];
    let face_point = |side: Direction| {
        let offset = side.offset();
        [0.5 + offset[0] as f32 * 0.5, 0.5 + offset[1] as f32 * 0.5, 0.5 + offset[2] as f32 * 0.5]
    };
    let progress = transit_progress(transit, partial_tick);
    let (from, to, step) = if progress < 0.5 {
        (face_point(transit.from), centre, progress * 2.0)
    } else {
        (centre, face_point(transit.to), (progress - 0.5) * 2.0)
    };
    [from[0] + (to[0] - from[0]) * step, from[1] + (to[1] - from[1]) * step, from[2] + (to[2] - from[2]) * step]
}

/// Item pipes and extraction pipes: every stack in transit, drawn at its place on its ride.
pub(crate) fn render_item_pipe(render: &mut BlockEntityRender<'_>) {
    let state = PipeState::load(render.data);
    if state.transit.is_empty() {
        return;
    }
    let partial = render.canvas.partial_tick().unwrap_or(0.0);
    for transit in &state.transit {
        render.canvas.push();
        render.canvas.translate(transit_position(transit, partial));
        render.canvas.scale([ITEM_SCALE; 3]);
        render.canvas.item_with_context(&transit.stack, ItemDisplayContext::None);
        render.canvas.pop();
    }
}

/// The fluid in tank 0 of a block entity's sent data (the host's saved tank encoding: a
/// `fluids` list of `Fluid` / `Amount` / optional `Tag` compounds).
pub fn tank_fluid(data: &NbtCompound) -> Option<FluidStack> {
    let tank = data.get("fluids")?.as_list()?.first()?.as_compound()?;
    let name = tank.get("Fluid")?.as_str()?;
    let amount = tank.get("Amount")?.as_i64()?;
    if name == "minecraft:empty" || amount <= 0 {
        return None;
    }
    Some(FluidStack {
        fluid: Identifier::parse(name).ok()?,
        amount: amount as u32,
        tag: tank.get("Tag").and_then(|tag| tag.as_compound().cloned()),
    })
}

/// The block a fluid is drawn as: the base game's own fluids are blocks already; a mod
/// fluid is drawn as its world block, `<path>_fluid` in its own namespace (the block a
/// fluid definition declares for the world). A fluid with no such block draws nothing —
/// the canvas answers false and the pipe simply shows empty.
pub fn fluid_block(fluid: &Identifier) -> String {
    match fluid.to_string().as_str() {
        "minecraft:water" => "minecraft:water".to_owned(),
        "minecraft:lava" => "minecraft:lava".to_owned(),
        _ => format!("{}:{}_fluid", fluid.namespace(), fluid.path()),
    }
}

fn draw_fluid(canvas: &mut dyn BlockEntityCanvas, block: &str, at: [f32; 3]) {
    canvas.push();
    canvas.translate(at);
    canvas.scale([FLUID_SCALE; 3]);
    canvas.translate([-0.5; 3]);
    canvas.block(block, &[("level", "0")]);
    canvas.pop();
}

/// Fluid pipes and fluid extraction pipes: the held fluid, drawn in the centre and along
/// every arm the block state connects (the state's own face properties say which).
pub(crate) fn render_fluid_pipe(render: &mut BlockEntityRender<'_>) {
    let Some(fluid) = tank_fluid(render.data) else { return };
    let block = fluid_block(&fluid.fluid);
    let canvas = &mut *render.canvas;
    draw_fluid(canvas, &block, [0.5, 0.5, 0.5]);
    for side in Direction::ALL {
        if canvas.state_property(side.name()).as_deref() == Some("true") {
            let offset = side.offset();
            draw_fluid(
                canvas,
                &block,
                [0.5 + offset[0] as f32 * 0.33, 0.5 + offset[1] as f32 * 0.33, 0.5 + offset[2] as f32 * 0.33],
            );
        }
    }
}

pub fn register() {
    for name in [ITEM_PIPE, EXTRACTION_PIPE] {
        BlockEntityRendererRegistry::register(
            Identifier::new(NAMESPACE, name).expect("valid pipe id"),
            render_item_pipe,
        );
    }
    for name in [FLUID_PIPE, FLUID_EXTRACTION_PIPE] {
        BlockEntityRendererRegistry::register(
            Identifier::new(NAMESPACE, name).expect("valid pipe id"),
            render_fluid_pipe,
        );
    }
}
