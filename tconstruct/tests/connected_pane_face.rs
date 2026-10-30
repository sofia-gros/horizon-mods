use horizon_core::geometry::Direction;
#[derive(Clone, Copy, PartialEq, Eq)] enum SkipRendering { Never, SameBlock, Bars }
#[derive(Clone, Copy)] struct Definition {skip: SkipRendering, bars_tag: bool}
#[derive(Clone, Copy)] struct State {block:u32, connections:u8, connection_properties:u8, definition:Definition}
include!("../mixins/tconstruct/connected-pane-face.rs");
#[test] fn source_panes_preserve_only_exposed_vertical_arms() {
 let pane=|block,connections| State{block,connections,connection_properties:0b111100,definition:Definition{skip:SkipRendering::SameBlock,bars_tag:true}};
 let full=pane(100,0b001100); let thin=pane(100,0b000100);
 assert_eq!(tconstruct_connected_pane_face(full,thin,Direction::Up),std::ops::ControlFlow::Break(false));
 assert_eq!(tconstruct_connected_pane_face(thin,full,Direction::Down),std::ops::ControlFlow::Break(true));
 let bar=State{block:1,connections:0,connection_properties:0b111100,definition:Definition{skip:SkipRendering::Bars,bars_tag:true}};
 assert_eq!(tconstruct_connected_pane_face(bar,thin,Direction::Up),std::ops::ControlFlow::Break(false));
 let vanilla=State{block:2,connections:0,connection_properties:0b111100,definition:Definition{skip:SkipRendering::Bars,bars_tag:false}};
 assert_eq!(tconstruct_connected_pane_face(vanilla,vanilla,Direction::Up),std::ops::ControlFlow::Continue(()));
}
