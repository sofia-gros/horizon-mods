// Native packet projection only. The installed RETURN hook establishes phase.
use horizon_protocol::packets::play::Clientbound as Packet;
use pluto_inventory_api::client_payload as api;

pub fn observe(packet: &Packet) {
    if !api::enabled() {
        return;
    }
    if let Packet::CustomPayload(payload) = packet {
        api::observe(api::Payload {
            channel: &payload.channel,
            bytes: &payload.data,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    static POINTER: AtomicUsize = AtomicUsize::new(0);
    fn record(payload: api::Payload<'_>) {
        assert_eq!(payload.channel, "fixture:payload");
        assert_eq!(payload.bytes, &[0, 255, 128]);
        assert_eq!(
            payload.bytes.as_ptr() as usize,
            POINTER.load(Ordering::SeqCst)
        );
        CALLS.fetch_add(1, Ordering::SeqCst);
    }
    #[test]
    fn actual_packet_projection_borrows_without_modifying_or_observing_other_variants() {
        use horizon_protocol::packets::configuration::CustomPayload;
        api::register("fixture:packet_projection", "fixture:payload", 3, record).unwrap();
        CALLS.store(0, Ordering::SeqCst);
        observe(&Packet::Unknown(-1, vec![0, 255, 128]));
        observe(&Packet::CustomPayload(CustomPayload {
            channel: "fixture:other".into(),
            data: vec![0, 255, 128],
        }));
        let packet = Packet::CustomPayload(CustomPayload {
            channel: "fixture:payload".into(),
            data: vec![0, 255, 128],
        });
        let before = packet.clone();
        if let Packet::CustomPayload(payload) = &packet {
            POINTER.store(payload.data.as_ptr() as usize, Ordering::SeqCst);
        }
        observe(&packet);
        assert_eq!(packet, before);
        assert_eq!(CALLS.load(Ordering::SeqCst), 1);
        api::remove("fixture:packet_projection").unwrap();
        observe(&packet);
        assert_eq!(CALLS.load(Ordering::SeqCst), 1);
    }
}
