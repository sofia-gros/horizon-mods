# Fire Protection transient state constructor repair

Brief static review of the defense stack found that `fire-duration-context`
targeted `PlayerMotion::new` with a `Self` struct literal selector. That function
is a factory delegating to `with_attribute_identities`; it has no such literal.
It also contains closures, which this constructor transformation rejects.
Pointing the selector at the other constructor is insufficient because that
body contains unsupported `?` expressions and closures.

Move the optional context onto the existing per-player `StatusFlags` instead.
Its actual literal is in `PlayerEffects::status`, already used by the shared
protection runtime. Default construction starts with no bonus. Update all three
read/write access sites and refresh the context after `apply_effect_status`
replaces the status value, in addition to equipment, tick and reload refresh.

This changes storage and lifecycle only. Fire level calculation, baseline
attribute guards, explicit ignition and fire-contact arithmetic are unchanged.
The state is transient and never written to the saved player or item. It must
accompany the Fire duration/contact stack before integration.

Static source review and `git diff --check` only. No tests, builds, planner runs,
CI inspection, version checks or merge. Publication remains paused.
