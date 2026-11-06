fn write_if_eligible(overlay: &Overlay<'_>, rel: &str, bytes: &[u8]) -> std::ops::ControlFlow<Result<()>> {
    let std::borrow::Cow::Owned(merged) = pluto_language_overlay::prepare(overlay.base, rel, bytes) else {
        return std::ops::ControlFlow::Continue(());
    };
    let result = (|| -> Result<()> {
        let rel = Path::new(rel);
        if let Some(parent) = rel.parent() {
            overlay.dir(parent)?;
        }
        let path = overlay.out.join(rel);
        if fs::symlink_metadata(&path).is_ok() {
            fs::remove_file(&path)?;
        }
        fs::write(path, &merged)?;
        Ok(())
    })();
    std::ops::ControlFlow::Break(result)
}
