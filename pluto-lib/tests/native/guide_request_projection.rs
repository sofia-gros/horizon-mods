// Actual official packet type and existing production projection, not live IO.
// IDs are explicitly carried in-process by this fixture, not decoded from a new
// wire format. Native observer phase and screen ownership are not exercised.
use crate::{client_payload, pluto_inventory};
use pluto_inventory::{
    client_payload as api,
    guide::requests::{RequestId, RequestTracker},
};
use std::sync::Mutex;

struct Case {
    tracker: RequestTracker,
    response: RequestId,
    calls: usize,
    consumed: usize,
    pointer: usize,
}
static CASE: Mutex<Option<Case>> = Mutex::new(None);
fn observe(payload: api::Payload<'_>) {
    let mut lock = CASE.lock().unwrap();
    let case = lock.as_mut().unwrap();
    case.calls += 1;
    if let Ok(original) = case
        .tracker
        .consume(case.response, "fixture:request_owner", payload)
    {
        assert_eq!(original.bytes, [0, 255, 128]);
        assert_eq!(original.bytes.as_ptr() as usize, case.pointer);
        case.consumed += 1;
    }
}
#[test]
fn actual_protocol_projection_preserves_borrow_with_explicit_correlation() {
    use horizon_protocol::packets::{configuration::CustomPayload, play::Clientbound as Packet};
    let mut tracker =
        RequestTracker::new("fixture:request_owner", "fixture:request_reply").unwrap();
    tracker.restart().unwrap();
    let old = tracker.issue().unwrap();
    let newest = tracker.issue().unwrap();
    let packet = Packet::CustomPayload(CustomPayload {
        channel: "fixture:request_reply".into(),
        data: vec![0, 255, 128],
    });
    let before = packet.clone();
    let pointer = match &packet {
        Packet::CustomPayload(p) => p.data.as_ptr() as usize,
        _ => unreachable!(),
    };
    *CASE.lock().unwrap() = Some(Case {
        tracker,
        response: old,
        calls: 0,
        consumed: 0,
        pointer,
    });
    api::register(
        "fixture:request_projection",
        "fixture:request_reply",
        3,
        observe,
    )
    .unwrap();
    client_payload::observe(&Packet::Unknown(-1, vec![0, 255, 128]));
    client_payload::observe(&packet);
    {
        let mut lock = CASE.lock().unwrap();
        let case = lock.as_mut().unwrap();
        assert_eq!((case.calls, case.consumed), (1, 0));
        case.response = newest;
    }
    client_payload::observe(&packet);
    client_payload::observe(&packet);
    assert_eq!(packet, before);
    {
        let lock = CASE.lock().unwrap();
        let case = lock.as_ref().unwrap();
        assert_eq!((case.calls, case.consumed), (3, 1));
        assert!(!case.tracker.has_pending());
    }
    api::remove("fixture:request_projection").unwrap();
    *CASE.lock().unwrap() = None;
}
