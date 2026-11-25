//! Compile the pipes mod's production modules against the actual host-independent PlutoLib API.
#[path = "../../../pipes/mixins/pipes/api/src/common.rs"]
mod common;
#[path = "../../../pipes/mixins/pipes/api/src/item_pipe.rs"]
mod item_pipe;
#[path = "../../../pipes/mixins/pipes/api/src/fluid_pipe.rs"]
mod fluid_pipe;
#[path = "../../../pipes/mixins/pipes/api/src/connections.rs"]
mod connections;
#[path = "../../../pipes/mixins/pipes/api/src/render.rs"]
mod render;

mod pipe_tests;
