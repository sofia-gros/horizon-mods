# Synchronous advancement event adapter, 2026-11-05

This extends the same advancement API recovery group with an explicit EventSink.
PlutoLib owns actor/authority checks and native player-condition/event dispatch.
The consumer owns trigger IDs, payload, successful-action boundary and matching
policy. There is no extra registry, automatic hook, actor cache or delayed queue.

## Focused checks

- 18 tests pass in the real shared Cargo API crate (the preceding callback/data
  suite plus the host-independent borrowed-event/submission contract).
- 5 unified-package guards pass, including exact event module/API bytes and
  actor gates before reads, tree loading before player facts, native dispatch,
  and absence of automatic event hooks or TConstruct IDs in shared code.
- 4 bounded adapter/consumer tests pass using the actual PlutoLib native adapter,
  real shared crate and production TConstruct policy. Only the native Game and
  advancement progress/predicate machinery are modeled. They exercise exact
  type matching, modeled local/guest actor isolation, repeated completed
  criteria, remote/absent-actor suppression before reads, explicit submission
  and Submitted without any award. This is not game or LAN transport evidence.
- 3 static consumer wiring guards pass: existing local validation/guest exclusion,
  guest real-menu check and as_guest ordering, and host-independent policy module.
- Source review confirms no direct Game, World, Entities or native condition
  handling remains in the TConstruct policy. Its controller/type/trigger policy
  has not been moved into the library.

## Complete shared-library official proof

The later complete Pluto package at
`14c01d9f0232f7821b8f16f61a21ce9831eedadf` passed official plan, mandatory preflight,
apply and host library/binary checking on Horizon
624c4f6265c02296fbe8a80f029dd0d987fb94c0 with Mixins
e8cd0aa38cffef49cb2e088ef716baa107148c02. That package includes this exact event
adapter and API source (byte-compared), the predicate facade and default-off
inventory events. See the [shared foundation report](shared-foundation-validation-20261105.md).

The complete package contains 2 crates, 23 modules, 21 injections and 1 state.
Its archive SHA-256 is
`30cf9fbff8b047a16e95045a67bbf9f3bc436e9fce21ab9c0efe38acdbf909c2`.
The host check exited 0 in 78 seconds; its disk guard did not stop the run.

This closes the native type-check gate for the shared event slice only. It does
not include TConstruct consumer commit47deda7b or the subsequent tag/index
follow-up. Full machine callsite typing, native menu opening, dedicated/LAN
attribution, saving, network delivery and display remain unverified. Consumer
callsite changes stay on the feature branch, not in the shared foundation, and
do not reclassify the original acquisition PR heads.
