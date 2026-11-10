// Generic, marker-free template pieces and their persisted codec data.
// Custom piece/marker codecs are deliberately rejected until their dispatch
// exists. No Tinkers identifiers or algorithms belong in this host adapter.
use super::*;
use super::legacy::{LegacyPiece,PieceContext,TemplateHandler,TemplatePiece};
use super::processor::ProcessorList;
use crate::nbt::{Compound,Tag};
use serde_json::Value;

pub const TEMPLATE_KIND:&str="pluto_lib:template_piece";
const MAX_SAVED_CONFIG:usize=1_048_576;

/// Iterative guard before any recursive serde operation or template lookup.
pub fn validate_value(value:&Value)->Result<(),String> {
    let mut pending=vec![(value,0usize)];
    let (mut nodes,mut bytes)=(0usize,0usize);
    while let Some((value,depth))=pending.pop() {
        nodes+=1;
        if depth>128 || nodes>65_536 {return Err("structure descriptor depth/node budget exceeded".into())}
        match value {
            Value::String(text)=>bytes=bytes.saturating_add(text.len()),
            Value::Array(values)=>pending.extend(values.iter().map(|v|(v,depth+1))),
            Value::Object(values)=>for (key,value) in values {bytes=bytes.saturating_add(key.len()); pending.push((value,depth+1));},
            _=>{},
        }
        if bytes>MAX_SAVED_CONFIG {return Err("structure descriptor string budget exceeded".into())}
    }
    Ok(())
}

fn identifier(value:&str)->bool {
    let Some((namespace,path))=value.split_once(':') else {return false};
    !namespace.is_empty() && !path.is_empty()
        && namespace.bytes().all(|b|b.is_ascii_lowercase()||b.is_ascii_digit()||b"_.-".contains(&b))
        && path.bytes().all(|b|b.is_ascii_lowercase()||b.is_ascii_digit()||b"_./-".contains(&b))
        && path.split('/').all(|segment|!segment.is_empty()&&segment!="."&&segment!="..")
}
fn position(value:&Value)->Result<[i32;3],String> {
    let values=value.as_array().filter(|v|v.len()==3).ok_or("template origin must have three coordinates")?;
    let mut pos=[0;3];
    for (axis,value) in values.iter().enumerate() {
        pos[axis]=value.as_i64().and_then(|v|i32::try_from(v).ok()).ok_or("template coordinate out of range")?;
    }
    Ok(pos)
}
fn rotation(value:&str)->Result<Rotation,String> {
    match value {"NONE"=>Ok(Rotation::None),"CLOCKWISE_90"=>Ok(Rotation::Clockwise90),
        "CLOCKWISE_180"=>Ok(Rotation::Clockwise180),"COUNTERCLOCKWISE_90"=>Ok(Rotation::Counterclockwise90),
        _=>Err("unknown template rotation".into())}
}
fn mirror(value:&str)->Result<Mirror,String> {
    match value {"NONE"=>Ok(Mirror::None),"LEFT_RIGHT"=>Ok(Mirror::LeftRight),"FRONT_BACK"=>Ok(Mirror::FrontBack),
        _=>Err("unknown template mirror".into())}
}

#[derive(Debug)]
struct Piece {native:TemplatePiece,configuration:Value}
impl Piece {
    fn decode(resources:&StructureResources,configuration:&Value)->Result<Self,String> {
        let object=configuration.as_object().ok_or("template piece configuration must be an object")?;
        if object.keys().any(|key|!matches!(key.as_str(),"template"|"origin"|"rotation"|"mirror")) {
            return Err("unsupported template piece field".into());
        }
        let name=configuration.get("template").and_then(Value::as_str).filter(|v|identifier(v)).ok_or("invalid template identifier")?;
        let pos=position(configuration.get("origin").ok_or("missing template origin")?)?;
        let rot=rotation(configuration.get("rotation").and_then(Value::as_str).ok_or("missing template rotation")?)?;
        let mir=mirror(configuration.get("mirror").and_then(Value::as_str).ok_or("missing template mirror")?)?;
        let template=resources.template(name).ok_or("template could not be decoded")?;
        if template.size.iter().any(|v|*v<=0) {return Err("missing or empty structure template".into())}
        let reach=template.size.iter().copied().max().ok_or("empty template size")?;
        if pos.iter().any(|v|v.checked_add(reach).is_none()||v.checked_sub(reach).is_none()) {
            return Err("template transformed bounds overflow".into());
        }
        // Native plain pieces can spawn template entities and process jigsaws.
        // This first codec does neither: reject them before publishing a start,
        // not after part of its template was written.
        if !template.entities.is_empty() {return Err("template entities are not supported by this piece codec".into())}
        for palette in template.palettes() {
            for block in palette.blocks() {
                let state=resources.registry.state(block.state).ok_or("unknown template block state")?;
                if matches!(state.name.as_str(),"minecraft:structure_block"|"minecraft:jigsaw") {
                    return Err("template marker/jigsaw dispatch is not connected".into());
                }
            }
        }
        let native=TemplatePiece::new(TEMPLATE_KIND,name,template,pos,rot,mir,[0;3],
            Arc::new(ProcessorList(Vec::new())),false,
            vec!["minecraft:structure_void".into()],TemplateHandler::Plain);
        Ok(Self {native,configuration:configuration.clone()})
    }
}
impl LegacyPiece for Piece {
    fn bounding_box(&self)->BoundingBox {self.native.bounding_box}
    fn type_name(&self)->&'static str {TEMPLATE_KIND}
    fn save(&self)->Option<Compound> {
        let configuration=serde_json::to_string(&self.configuration).ok()?;
        if configuration.len()>MAX_SAVED_CONFIG {return None}
        let mut saved=self.native.saved();
        saved.insert("Mi".into(),Tag::String(self.native.mirror.saved_name().into()));
        saved.insert("PlutoVersion".into(),Tag::Int(1));
        saved.insert("PlutoConfiguration".into(),Tag::String(configuration.as_str().into()));
        Some(saved)
    }
    fn post_process(&self,context:&mut PieceContext<'_>)->legacy::Result<()> {
        self.native.place_at(context,self.native.position,&[])?;
        Ok(())
    }
}

/// Decode every descriptor before a start can be published; no world writes.
pub fn decode(resources:&StructureResources,descriptor:&Value)->Result<start::Piece,String> {
    if descriptor.get("kind").and_then(Value::as_str)!=Some(TEMPLATE_KIND) {
        return Err("unsupported structure piece type".into());
    }
    let configuration=descriptor.get("configuration").ok_or("missing piece configuration")?;
    validate_value(configuration)?;
    if serde_json::to_vec(configuration).map_err(|e|e.to_string())?.len()>MAX_SAVED_CONFIG {
        return Err("structure piece configuration exceeds save budget".into());
    }
    Ok(start::Piece::Legacy(Box::new(Piece::decode(resources,configuration)?)))
}

/// Restore from the host's NBT-as-JSON view, rebuilding bounds from the current
/// template rather than trusting a potentially inconsistent saved bounding box.
pub fn restore(resources:&StructureResources,saved:&Value)->Result<start::Piece,String> {
    if saved.get("id").and_then(Value::as_str)!=Some(TEMPLATE_KIND)
        || saved.get("PlutoVersion").and_then(Value::as_i64)!=Some(1) {
        return Err("unsupported saved structure piece/version".into());
    }
    let text=saved.get("PlutoConfiguration").and_then(Value::as_str).filter(|v|v.len()<=MAX_SAVED_CONFIG)
        .ok_or("missing or oversized saved structure piece configuration")?;
    let configuration:Value=serde_json::from_str(text).map_err(|e|format!("invalid saved structure configuration: {e}"))?;
    decode(resources,&serde_json::json!({"kind":TEMPLATE_KIND,"configuration":configuration}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn descriptor_boundaries_reject_paths_invalid_transforms_and_coordinate_overflow() {
        assert!(identifier("example:buildings/test-house"));
        for id in ["example:../house","example:/house","example:a//b","example:a/./b","example:House","a:b:c"] {assert!(!identifier(id),"{id}")}
        assert_eq!(position(&serde_json::json!([-8,70,24])).unwrap(),[-8,70,24]);
        assert!(position(&serde_json::json!([0,2147483648i64,0])).is_err());
        assert!(position(&serde_json::json!([0,1])).is_err());
        assert!(rotation("CLOCKWISE_90").is_ok()); assert!(rotation("90").is_err());
        assert!(mirror("FRONT_BACK").is_ok()); assert!(mirror("front").is_err());
    }
}
