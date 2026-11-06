// Source proposal only: calls the exact public official 5a64 Index/token/hash APIs.
// The outer existing diagnostic controller must pin both clean source trees and
// locks, use its guarded PGID/timeout, and recheck full source seals afterward.
use horizon_mixins_core::{index::{Index, tokens}, source};
use serde_json::json;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let root = PathBuf::from(args.next().ok_or("expected one pristine official workspace")?);
    if args.next().is_some() { return Err("unexpected extra argument".into()); }
    let index = Index::load(&root)?;
    let targets = [
        ("horizon_client::game::native_hooks::Game<'_>::native_client_packet", Some("6439828d2b9197605942c9bbfff9c2a32b93c95d934c8554fb52cc72ef85827f")),
        ("horizon_client::game::remote::Game<'_>::handle_packet", Some("9f16b281b32524d06842ac401582ce22a2c022e4b8dc45a43bfc82bd98eb9724")),
        ("horizon_client::game::remote::Game<'_>::poll_remote", None),
    ];
    let mut rows = Vec::new();
    for (target, known) in targets {
        let f = index.resolve(target)?;
        // Exact expression used by official plan.rs, not reimplemented tokens.
        let fingerprint = source::fingerprint(format!("{}{}", tokens(&f.signature), tokens(&f.block)));
        if let Some(known) = known {
            if fingerprint != known { return Err("known complete-plan control fingerprint differs".into()); }
        }
        rows.push(json!({"target":target,"resolved_id":f.id,"file":f.file,
                         "node":f.node,"function_fingerprint":fingerprint}));
    }
    println!("{}", serde_json::to_string(&json!({"schema":1,"targets":rows}))?);
    Ok(())
}
