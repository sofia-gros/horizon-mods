// Excluded root-key proposal, not installed. The exact native caller supplies
// the owned out path and complete 64-hex native stamp after stamp() succeeds.
fn separate_root(mut out: std::path::PathBuf, stamp: &String) -> std::path::PathBuf {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for part in [
        "plutolib language root policy 1",
        "plutolib language lower-only strict strings 1",
        stamp.as_str(),
    ] {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part.as_bytes());
    }
    let separated = format!("{:x}", hasher.finalize());
    out.set_file_name(&separated[..ROOT_NAME]);
    out
}
