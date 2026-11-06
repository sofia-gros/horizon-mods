fn after_native_packet(value: (), game: &mut Game<'_>, packet: &ClientboundPlayPacket) {
    let selected = super::pluto_client_payload_updates::snapshot(packet);
    if game.options.execution_role.is_client() {
        horizon_client::pluto_client_payload::observe(packet);
    }
    match selected {
        Ok(Some(snapshot)) => snapshot.dispatch(game),
        Ok(None) => {},
        Err(error) => {
            // No UI effect on arbitrary custom channels. The one-time log is
            // emitted after native/read-only observation, without retrying.
            static REPORTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            if !REPORTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                log::error!("PlutoLib: retained-view payload dispatch unavailable: {error:?}");
            }
        }
    }
    value
}
