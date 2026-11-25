//! Runs the native demo initializer as a standalone Rust program for inspection.
#[path = "native-creative/mixins/pluto_creative_demo/demo.rs"]
mod demo;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    demo::prepare()
}
