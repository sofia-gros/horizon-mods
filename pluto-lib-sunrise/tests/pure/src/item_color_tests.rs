//! Executes the actual native bridge/layers helper against structural fixtures.
//! This checks tint transport, not Horizon type/site attachment or model baking.
use std::{borrow::Cow,cell::Cell,collections::BTreeMap,sync::{Arc,Mutex}};
use serde_json::{Value,json};
struct StackView { item:String,count:i32 }
struct Models { effective:BTreeMap<String,Value>,reads:Cell<usize> }
impl Models {
    fn components(&self,_:&StackView)->Cow<'_,BTreeMap<String,Value>> {
        self.reads.set(self.reads.get()+1); Cow::Borrowed(&self.effective)
    }
}
struct ItemRenderer { models:Models }
#[derive(Clone,Debug,PartialEq)]
struct Quad { tint_index:i32 }
#[derive(Clone,Debug,PartialEq)]
struct Baked { quads:Vec<Quad> }
#[derive(Clone,Debug,PartialEq)]
enum LayerGeometry { Quads(Arc<Baked>), Special { argument:Value } }
#[derive(Clone,Debug,PartialEq)]
struct LayerRender { geometry:LayerGeometry,tints:Vec<i32>,unchanged:i32 }
#[derive(Clone,Debug,PartialEq)]
struct ItemRenderState {
    layers:Vec<LayerRender>, display:u8, oversized_in_gui:bool, animated:bool,
    cit_glint:Option<Arc<String>>, hdr_tier:u8, hdr_light:u8,
}
#[path="../../../mod/pluto-item-colors.rs"]
mod pluto_item_colors;
// The layers helper also runs the retexture pass, which needs the host's
// item environment and the retexture module; neither exists in this
// fixture, and the tint transport this test checks does not need them:
// the stub environment carries nothing, and the stub retexture passes the
// layers through unchanged, as the real one does with no provider installed.
trait ItemEnv {}
struct NoEnv;
impl ItemEnv for NoEnv {}
mod pluto_retexture_item {
    use super::{ItemEnv, ItemRenderer, LayerRender, StackView};
    pub fn retexture(_renderer: &ItemRenderer, _stack: &StackView, _env: &dyn ItemEnv, layers: Vec<LayerRender>) -> Vec<LayerRender> {
        layers
    }
}
include!("../../../mod/pluto-item-color-layers.rs");

#[test]
fn actual_layer_helper_keeps_native_fallback_specials_and_shared_geometry_and_adds_opaque_rgb() {
    static CALLS:Mutex<Vec<(String,i32,BTreeMap<String,Value>,i32)>>=Mutex::new(Vec::new());
    pluto_item_colors::set_provider(|item|item=="example:painted",|item,count,components,index| {
        CALLS.lock().unwrap().push((item.into(),count,components.clone(),index));
        match index {0=>Some(0x12010203),3=>Some(0xFEDCBA),_=>None}
    }).unwrap();
    assert!(pluto_item_colors::set_provider(|_|true,|_,_,_,_|None).is_err());
    let renderer=ItemRenderer { models:Models {effective:[("minecraft:damage".into(),json!(5)),
        ("minecraft:custom_data".into(),json!({"colour":7}))].into_iter().collect(),reads:Cell::new(0)} };
    let geometry=Arc::new(Baked {quads:[-1,0,0,1,2,3,4096,i32::MAX].into_iter().map(|tint_index|Quad{tint_index}).collect()});
    let layers=vec![LayerRender {geometry:LayerGeometry::Quads(geometry.clone()),tints:vec![0x22,0x334455],unchanged:41},
        LayerRender {geometry:LayerGeometry::Special {argument:json!({"head":"kept"})},tints:vec![7],unchanged:99}];
    let state=ItemRenderState {layers:layers.clone(),display:3,oversized_in_gui:true,animated:true,
        cit_glint:Some(Arc::new("kept CIT glint".into())),hdr_tier:2,hdr_light:15};
    let env=NoEnv;
    let unknown=StackView {item:"example:unknown".into(),count:2};
    assert_eq!(pluto_item_color_layers(state.clone(),&renderer,&unknown,&env),state);
    assert_eq!(renderer.models.reads.get(),0,"unknown items do not decode components");
    assert!(CALLS.lock().unwrap().is_empty());
    let stack=StackView {item:"example:painted".into(),count:2};
    let result_state=pluto_item_color_layers(state.clone(),&renderer,&stack,&env);
    let mut expected_state=state;
    expected_state.layers=result_state.layers.clone();
    assert_eq!(result_state,expected_state,"tinting preserves every non-layer render-state field");
    assert!(Arc::ptr_eq(result_state.cit_glint.as_ref().unwrap(),expected_state.cit_glint.as_ref().unwrap()));
    let result=result_state.layers;
    assert_eq!(result[0].tints,vec![0xFF010203u32 as i32,0x334455,-1,0xFFFEDCBAu32 as i32]);
    assert_eq!(result[0].unchanged,41);
    assert_eq!(result[1],layers[1],"special renderers retain their own tinting");
    assert_eq!(layers[0].tints,vec![0x22,0x334455],"the input snapshot stays independent");
    let LayerGeometry::Quads(after)=&result[0].geometry else {panic!("geometry replaced")};
    assert!(Arc::ptr_eq(after,&geometry),"shared baked models are never rewritten");
    assert_eq!(renderer.models.reads.get(),1);
    let calls=CALLS.lock().unwrap();
    assert_eq!(calls.iter().map(|call|call.3).collect::<Vec<_>>(),vec![0,1,2,3]);
    for (item,count,components,_) in calls.iter() {
        assert_eq!(item,"example:painted"); assert_eq!(*count,2);
        assert_eq!(components,&renderer.models.effective,"the resolver supplies its already merged view");
    }
}
