// Owned registered template pieces. No native world or random borrow is kept.
use super::*;
use super::legacy::{LegacyPiece,PieceContext,TemplateHandler,TemplatePiece};
use super::processor::ProcessorList;
use crate::level::pluto_structure_access::{self,RegisteredTemplate};
use crate::nbt::{Compound,Tag};
use serde_json::Value;
use std::sync::{Mutex,Weak,atomic::{AtomicI64,Ordering}};

pub const KIND:&str="pluto_lib:registered_template_piece";
const MAX_BYTES:usize=1_048_576;
static NEXT_ID:AtomicI64=AtomicI64::new(1);
static RECORDS:RwLock<BTreeMap<i64,Weak<RegisteredPiece>>>=RwLock::new(BTreeMap::new());

pub struct RegisteredPiece {
    pub native:TemplatePiece,
    pub resources:Arc<StructureResources>,
    pub definition:RegisteredTemplate,
    pub state:Mutex<Value>,
    kind:String,
    configuration:Value,
    runtime_id:i64,
}
impl std::fmt::Debug for RegisteredPiece {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result {
        f.debug_struct("RegisteredPiece").field("kind",&self.kind).field("template",&self.native.template_name).finish()
    }
}
#[derive(Debug)]
struct Piece(Arc<RegisteredPiece>);

impl LegacyPiece for Piece {
    fn bounding_box(&self)->BoundingBox {self.0.native.bounding_box}
    fn type_name(&self)->&'static str {KIND}
    fn save(&self)->Option<Compound> {
        let record=&self.0;
        let state=record.state.lock().ok()?;
        pluto_structure_access::validate_value(&state).ok()?;
        record.definition.callbacks.validate_state(&state).ok()?;
        let config=serde_json::to_string(&record.configuration).ok()?;
        let state=serde_json::to_string(&*state).ok()?;
        if config.len()>MAX_BYTES || state.len()>MAX_BYTES {return None}
        let mut tag=record.native.saved();
        tag.insert("Mi".into(),Tag::String(record.native.mirror.saved_name().into()));
        tag.insert("PlutoVersion".into(),Tag::Int(1));
        tag.insert("PlutoKind".into(),Tag::String(record.kind.as_str().into()));
        tag.insert("PlutoConfiguration".into(),Tag::String(config.as_str().into()));
        tag.insert("PlutoState".into(),Tag::String(state.as_str().into()));
        // An ephemeral weak lookup token, never trusted on restore. Restoring
        // always allocates a fresh record and token from the decoded config.
        tag.insert("PlutoRuntimeId".into(),Tag::Long(record.runtime_id));
        Some(tag)
    }
    fn post_process(&self,_:&mut PieceContext<'_>)->legacy::Result<()> {
        // The legacy object-safe PieceWorld erases feature/finalization access.
        // Never run a callback through an incomplete or invented world adapter.
        Err(crate::worldgen::placement::PlacementQueryError::Failed("registered piece requires the typed placement adapter".into()))
    }
}

fn safe_id(value:&str)->bool {
    let Some((namespace,path))=value.split_once(':') else {return false};
    !namespace.is_empty() && !path.is_empty()
        && namespace.bytes().all(|b|b.is_ascii_lowercase()||b.is_ascii_digit()||b"_.-".contains(&b))
        && path.bytes().all(|b|b.is_ascii_lowercase()||b.is_ascii_digit()||b"_./-".contains(&b))
        && path.split('/').all(|s|!s.is_empty()&&s!="."&&s!="..")
}

pub fn decode(resources:&Arc<StructureResources>,kind:&str,configuration:&Value,saved_state:Option<Value>)->Result<start::Piece,String> {
    pluto_structure_access::validate_value(configuration)?;
    if serde_json::to_vec(configuration).map_err(|e|e.to_string())?.len()>MAX_BYTES {return Err("piece configuration exceeds save budget".into())}
    let definition=pluto_structure_access::decode_piece(kind,configuration)?.ok_or("unsupported structure piece type")?;
    if !safe_id(&definition.template) {return Err("unsafe registered template identifier".into())}
    pluto_structure_access::validate_value(&definition.initial_state)?;
    let state=saved_state.unwrap_or_else(||definition.initial_state.clone());
    pluto_structure_access::validate_value(&state)?;
    definition.callbacks.validate_state(&state)?;
    if serde_json::to_vec(&state).map_err(|e|e.to_string())?.len()>MAX_BYTES {return Err("piece state exceeds save budget".into())}
    let rotation=match definition.rotation {0=>Rotation::None,1=>Rotation::Clockwise90,2=>Rotation::Clockwise180,3=>Rotation::Counterclockwise90,_=>return Err("invalid registered piece rotation".into())};
    let mirror=match definition.mirror {0=>Mirror::None,1=>Mirror::LeftRight,2=>Mirror::FrontBack,_=>return Err("invalid registered piece mirror".into())};
    let template=resources.template(&definition.template).ok_or("registered template could not be decoded")?;
    let reach=template.size.iter().copied().max().ok_or("empty registered template")?;
    if template.size.iter().any(|v|*v<=0) {return Err("missing or empty registered template".into())}
    if definition.origin.iter().any(|v|v.checked_add(reach).is_none()||v.checked_sub(reach).is_none()) {return Err("registered template bounds overflow".into())}
    for palette in template.palettes() {
        for block in palette.blocks() {
            if resources.registry.state(block.state).is_none_or(|s|s.name=="minecraft:jigsaw") {
                return Err("registered template jigsaws/unknown states are unsupported".into());
            }
        }
    }
    let native=TemplatePiece::new(KIND,&definition.template,template,definition.origin,rotation,mirror,[0;3],
        Arc::new(ProcessorList(Vec::new())),false,vec!["minecraft:structure_block".into()],TemplateHandler::Plain);
    let runtime_id=NEXT_ID.fetch_update(Ordering::Relaxed,Ordering::Relaxed,|id|id.checked_add(1)).map_err(|_|"piece runtime id exhausted")?;
    let record=Arc::new(RegisteredPiece {native,resources:resources.clone(),definition,state:Mutex::new(state),kind:kind.to_owned(),configuration:configuration.clone(),runtime_id});
    let mut records=RECORDS.write().map_err(|_|"piece record registry poisoned")?;
    records.retain(|_,record|record.strong_count()>0);
    records.insert(runtime_id,Arc::downgrade(&record));
    drop(records);
    Ok(start::Piece::Legacy(Box::new(Piece(record))))
}

/// Only owned record state crosses this lookup; no borrowed context is stored.
pub fn record(piece:&dyn LegacyPiece)->Result<Option<Arc<RegisteredPiece>>,String> {
    if piece.type_name()!=KIND {return Ok(None)}
    let saved=piece.save().ok_or("registered piece could not save its state")?;
    let Some(Tag::Long(id))=saved.get(&crate::nbt::JavaString::from("PlutoRuntimeId")) else {return Err("missing piece runtime token".into())};
    let weak=RECORDS.read().map_err(|_|"piece record registry poisoned")?.get(id).cloned();
    weak.and_then(|record|record.upgrade()).map(Some).ok_or_else(||"expired piece runtime record".into())
}

pub fn restore(resources:&Arc<StructureResources>,saved:&Value)->Result<start::Piece,String> {
    if saved.get("id").and_then(Value::as_str)!=Some(KIND)||saved.get("PlutoVersion").and_then(Value::as_i64)!=Some(1) {return Err("unsupported registered piece/version".into())}
    let kind=saved.get("PlutoKind").and_then(Value::as_str).ok_or("missing registered piece type")?;
    let parse=|name:&str|->Result<Value,String> {
        let text=saved.get(name).and_then(Value::as_str).filter(|s|s.len()<=MAX_BYTES).ok_or_else(||format!("missing/oversized {name}"))?;
        let value=serde_json::from_str(text).map_err(|e|format!("invalid {name}: {e}"))?;
        pluto_structure_access::validate_value(&value)?;
        Ok(value)
    };
    let configuration=parse("PlutoConfiguration")?;
    decode(resources,kind,&configuration,Some(parse("PlutoState")?))
}
