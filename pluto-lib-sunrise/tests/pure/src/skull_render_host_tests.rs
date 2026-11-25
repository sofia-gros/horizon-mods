//! Actual decoder/parts/worn/bounds/submit helpers with trace host types.
//! Native JSON codec/types, model baking, transformations and pixels are NOT
//! exercised by these facades and require separate native validation.
use std::{collections::HashMap,path::Path,sync::atomic::{AtomicBool,Ordering}};
use serde_json::{Value,Map,json};
mod sunrise_assets {pub(crate) mod item_model {pub(crate) use crate::skull_model_access_tests as pluto_skull_model_access;}}
pub(crate) mod parts {
    use super::sunrise_assets;
    include!("../../../mod/src/pluto-skull-item-models.rs");
}
#[derive(Clone,Debug,PartialEq)]
pub(crate) struct ModelPart {id:f32}
impl ModelPart {fn parse(value:&Value)->Self {Self {id:value["id"].as_f64().unwrap() as f32}}}
#[derive(Clone,Debug,PartialEq)]
enum SpecialKind {Head {kind:String,texture:Option<String>,animation:f32},Other}
#[derive(Clone,Debug,PartialEq)]
struct SpecialModel {kind:SpecialKind}
type Result<T>=std::result::Result<T,String>;
fn err(message:&str)->String {message.into()}
fn object(value:&Value)->Result<&Map<String,Value>> {value.as_object().ok_or("expected object".into())}
fn get<'a>(map:&'a Map<String,Value>,key:&str)->Option<&'a Value> {map.get(key)}
fn ident(value:&Value)->Result<String> {
    let s=value.as_str().ok_or("expected identifier string")?;
    let s=if s.contains(':') {s.to_owned()} else {format!("minecraft:{s}")};
    crate::Identifier::parse(&s).map(|id|id.to_string()).map_err(|e|e.to_string())
}
fn float(value:&Value)->Result<f32> {value.as_f64().map(|v|v as f32).ok_or("expected number".into())}
#[derive(Clone,Copy,Debug,PartialEq)]
struct Mat4(u32);
impl Mat4 {const IDENTITY:Self=Self(0);}
type Vec3=[f32;3];
#[derive(Clone,Copy,Debug,PartialEq)]
struct Rect(u32);
#[derive(Debug,PartialEq)]
enum Foil {None}
struct ItemRenderer {parts:HashMap<String,ModelPart>}
impl ItemRenderer {fn part(&self,key:&str)->Option<&ModelPart> {self.parts.get(key)}}
// The native pose algorithm is not duplicated: this marker records only which
// inherited family and animation the real dispatch helpers pass to it.
fn skull_pose(style:&str,animation:f32)->impl Fn(&str,&mut [f32;9])+'_ {
    move|_,pose|{pose[0]=match style {"piglin"=>1.0,"dragon"=>2.0,_=>0.0};pose[1]=animation;}
}
fn extents_of(part:&ModelPart,_:&str,pose:Mat4,adjust:&dyn Fn(&str,&mut [f32;9]),out:&mut Vec<Vec3>) {
    assert_eq!(pose,Mat4::IDENTITY);let mut p=[0.0;9];adjust("head",&mut p);out.push([part.id,p[0],p[1]]);
}
#[derive(Debug,PartialEq)]
struct Draw {part:f32,pose:Mat4,rect:Rect,color:u32,style:f32,animation:f32}
trait ItemSink {fn draw(&mut self,draw:Draw);}
impl ItemSink for Vec<Draw> {fn draw(&mut self,draw:Draw) {self.push(draw);}}
fn argb(color:i32)->u32 {color as u32}
#[allow(clippy::too_many_arguments)]
fn draw(part:&ModelPart,_:&str,pose:Mat4,adjust:&dyn Fn(&str,&mut [f32;9]),rect:Rect,color:u32,foil:Foil,sink:&mut dyn ItemSink) {
    assert_eq!(foil,Foil::None);let mut p=[0.0;9];adjust("head",&mut p);
    sink.draw(Draw {part:part.id,pose,rect,color,style:p[0],animation:p[1]});
}
include!("../../../mod/src/pluto-skull-special-decode.rs");
include!("../../../mod/src/pluto-skull-parts-return.rs");
struct EntityModelDocument(Value);
impl EntityModelDocument {fn value(&self)->&Value {&self.0}}
include!("../../../mod/src/pluto-skull-document-parts-return.rs");
include!("../../../mod/src/pluto-skull-worn-kind.rs");
include!("../../../mod/src/pluto-skull-extents.rs");
include!("../../../mod/src/pluto-skull-submit.rs");
#[test]
fn actual_skull_chain_decodes_owns_geometry_and_forwards_kind_texture_pose_animation_without_native_fallback() {
    use std::ops::ControlFlow;
    use crate::{Identifier,SkullModelAnimation,SkullModelRegistry};
    static BAD:AtomicBool=AtomicBool::new(false);
    static DOCUMENT_FALLBACK:AtomicBool=AtomicBool::new(false);
    SkullModelRegistry::register(Identifier::parse("skull_trace:native").unwrap(),
        "minecraft:skeleton#main",Identifier::parse("skull_trace:entity/native").unwrap(),SkullModelAnimation::None).unwrap();
    for (name,style) in [("plain",SkullModelAnimation::None),("piglin",SkullModelAnimation::Piglin)] {
        SkullModelRegistry::register(Identifier::parse(&format!("skull_trace:{name}")).unwrap(),
            &format!("skull_trace:{name}#main"),Identifier::parse(&format!("skull_trace:entity/{name}")).unwrap(),style).unwrap();
    }
    crate::skull_model_access_tests::set_provider(||Ok(crate::pure::skull_model_registry::definitions()?.into_iter().map(|d|
        crate::skull_model_access_tests::Definition {item:d.item.to_string(),layer:d.layer,texture:d.texture.to_string(),animation:d.animation as u8}).collect())).unwrap();
    parts::set_provider(||Ok(if BAD.load(Ordering::SeqCst) {vec![("skull_trace:piglin".into(),ModelPart {id:2.0})]} else {
        let mut models=vec![("skull_trace:piglin".into(),ModelPart {id:2.0}),("skull_trace:plain".into(),ModelPart {id:1.0})];
        if !DOCUMENT_FALLBACK.load(Ordering::SeqCst) {models.push(("skull_trace:native".into(),ModelPart {id:3.0}));}
        models
    })).unwrap();
    let base=HashMap::from([("skeleton".into(),ModelPart {id:99.0})]);
    BAD.store(true,Ordering::SeqCst);let mut failed=base.clone();
    assert!(parts::extend(&mut failed,Path::new("unused-custom-only-root")).is_err());assert_eq!(failed,base,"no prefix geometry published");
    BAD.store(false,Ordering::SeqCst);
    let loaded=pluto_skull_parts_return(base.clone(),Path::new("unused-custom-only-root"));
    assert_eq!(loaded.get("skeleton"),base.get("skeleton"));assert_eq!(loaded.len(),4);
    let renderer=ItemRenderer {parts:loaded};
    assert_eq!(pluto_skull_worn_kind("minecraft:piglin_head"),ControlFlow::Continue(()));
    assert_eq!(pluto_skull_worn_kind("other:missing"),ControlFlow::Continue(()));
    assert_eq!(pluto_skull_worn_kind("skull_trace:piglin"),ControlFlow::Break(Some("skull_trace:piglin")));
    for (kind,part,style) in [("plain",1.0,0.0),("piglin",2.0,1.0)] {
        let value=json!({"type":"minecraft:head","kind":format!("skull_trace:{kind}"),"animation":4.25});
        let ControlFlow::Break(Ok(model))=pluto_skull_special_decode(&value) else {panic!("registered custom head not decoded")};
        assert_eq!(pluto_skull_extents(&renderer,&model),ControlFlow::Break(vec![[part,style,4.25]]));
        let sprite=format!("skull_trace:entity/{kind}");let sprites=HashMap::from([(sprite,Rect(17))]);let mut sink=vec![];
        assert_eq!(pluto_skull_submit(&renderer,&model,Mat4(42),&sprites,&mut sink),ControlFlow::Break(()));
        assert_eq!(sink,vec![Draw {part,pose:Mat4(42),rect:Rect(17),color:u32::MAX,style,animation:4.25}]);
        sink.clear();assert_eq!(pluto_skull_submit(&renderer,&model,Mat4(43),&HashMap::new(),&mut sink),ControlFlow::Break(()));
        assert!(sink.is_empty(),"missing custom textures must not draw a vanilla head");
    }
    let value=json!({"type":"minecraft:head","kind":"skull_trace:plain","texture":"pack:replacement"});
    let ControlFlow::Break(Ok(model))=pluto_skull_special_decode(&value) else {panic!("override not decoded")};
    let mut sink=vec![];
    assert_eq!(pluto_skull_submit(&renderer,&model,Mat4(7),&HashMap::from([("pack:entity/replacement".into(),Rect(23))]),&mut sink),ControlFlow::Break(()));
    assert_eq!(sink[0].rect,Rect(23));
    assert!(matches!(pluto_skull_special_decode(&json!({"type":"minecraft:head","kind":"skull_trace:plain","animation":"bad"})),ControlFlow::Break(Err(_))));
    assert!(matches!(pluto_skull_special_decode(&json!({"type":"minecraft:head","kind":"skull_trace:plain","texture":17})),ControlFlow::Break(Err(_))));
    for value in [json!({"type":"minecraft:head","kind":"skeleton"}),json!({"type":"minecraft:head","kind":"other:missing"}),json!({"type":"other:renderer","kind":"skull_trace:plain"})] {
        assert_eq!(pluto_skull_special_decode(&value),ControlFlow::Continue(()));
    }
    let native=SpecialModel {kind:SpecialKind::Head {kind:"skeleton".into(),texture:None,animation:0.0}};
    assert_eq!(pluto_skull_extents(&renderer,&native),ControlFlow::Continue(()));
    assert_eq!(pluto_skull_submit(&renderer,&native,Mat4(1),&HashMap::new(),&mut vec![]),ControlFlow::Continue(()));
    assert_eq!(pluto_skull_extents(&renderer,&SpecialModel {kind:SpecialKind::Other}),ControlFlow::Continue(()));
    // Shared startup data supplies native fallback layers without reopening a file.
    DOCUMENT_FALLBACK.store(true,Ordering::SeqCst);
    let document=EntityModelDocument(json!({"minecraft:skeleton#main":{"id":17}}));
    let from_document=pluto_skull_document_parts_return(base.clone(),&document);
    assert_eq!(from_document["skull_trace:native"].id,17.0);
    assert_eq!(from_document["skull_trace:plain"].id,1.0);
    assert_eq!(from_document["skeleton"],base["skeleton"]);
    let reloaded=pluto_skull_document_parts_return(base.clone(),&EntityModelDocument(json!({"minecraft:skeleton#main":{"id":23}})));
    assert_eq!(reloaded["skull_trace:native"].id,23.0);
    assert_eq!(from_document["skull_trace:native"].id,17.0,"renderer trees remain owned");
    assert_eq!(pluto_skull_document_parts_return(base.clone(),&EntityModelDocument(json!({}))),base,
        "missing native layer publishes no partial custom models");
}

