# Unapplied proposal: one authoritative crafting pair

Status: **design and abstract test oracle only**. This is not an official patch,
an installed Mixins hook, a PlutoLib runtime API, or a production TConstruct port.
It has not been sent to the official project. Nothing here changes a host,
manifest, dependency, provider registration, or the existing matching guard.

- [Contract](contract.md): smallest proposed official semantic extension, its
  necessary caller changes, existing behavior and unresolved gates.
- [Acceptance matrix](acceptance.json): native tests required before any route
  can advertise support. These tests are specifications, not completed tests.
- `model.py` / `test_model.py`: dependency-free, symbolic state-machine oracle
  for a subset of the protocol. Run explicitly with:

  `python -m unittest discover -s pluto-lib/proposals/crafting-products-authority-20261106 -p 'test_model.py' -v`

The model does not import Horizon, model real ItemStack/component identity,
allocate real slots, execute native consumption, or send network/world events.
Passing it cannot establish native feasibility or close any acceptance gate.
See `validation.json` for the exact recorded scope and results.

Evidence baseline: official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0`,
official Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`, and the dated
`../../docs/crafting-products-{automatic-route-audit,route-probes,validation}-20261106`
reports. The explicit staging implementation is `54037209`; its complete v14
library gate passed. The three automatic-route probes were rejected. Neither
fact establishes the proposed authority contract.
