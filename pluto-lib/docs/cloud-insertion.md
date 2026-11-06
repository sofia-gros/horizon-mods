# Explicit owner-bound cloud insertion

This is a type-checked generic PlutoLib service, not an enabled TConstruct
feature or an executed actor/publication proof. It builds on the separately verified staged descriptor/factory and
adds no contact effects, cloud policy preset or automatic owner selection.
See [source audit](cloud-insertion-audit.md) and [proof status](cloud-insertion-proof.json).

The game-private `pluto_cloud_insertion::insert` takes a mutable Game, an explicit
native LevelRef, an explicit Owner (`Unowned` or `Entity(EntityRef)`), and a
Validated descriptor. It returns the published EntityRef or a rejection.
Native handles are compatibility details retained inside this adapter group;
there is not yet a stable external mod facade or activated consumer.

The service checks actual official lifecycle readiness, authoritative server
role, current-level resolution and entity/guest roster lookups. It requires the
resolved attribution to equal the requested descriptor value. Unsupported or
unresolvable ownership is an error, never an ownerless fallback. A standalone
player request with no guest connection is not currently resolvable on this
server-only path. An acting guest whose host is borrowed out is also rejected.
Consumers must still prove that the explicit entity is the source of their
action: presence in the server is not permission to attribute arbitrary actions.

Only the current Ready server level and a loaded center chunk are supported.
Paused/frozen Ready worlds are not automatically rejected: caller timing policy
remains separate. No extra height, block-replaceability, radius, duration or cure
policy is added. An expired ID range or reserve failure rejects before an ID is
taken. The constructor's complete body is unchanged; its only extension is a
consuming game-private accessor. The insertion prepares the native object and
vector capacity before taking an ID, setting it and pushing the object. It
makes no rollback or delivery guarantee after that native commit sequence.

Two private tests exercise the actual lifecycle state machine and two exercise
attribution and ID-range rejection helpers. They are prepared, not executed.
Six Python checks guard source boundaries and constructor preservation only;
they do not prove native access, actor lookups or runtime insertion. Official
Mixins module placement, complete-plan preflight, apply and host library/binary
type checks passed at cutoff `3ba060df` (grouped-v11b). The archive hash, five
cloud source hashes and all 5,042 applied source files were independently
rechecked. A resources-backed Game insertion test is still pending.

## Required complete-plan preflight

`tools/check_cloud_insertion_plan.py` exports `check(plan, workspace)` for the
central dispatcher. Its separate metadata pins 17 pristine official files and
requires their exact `plan.inputs` hashes, plus the actual packaged descriptor,
catalog, factory, readiness and insertion module identities/bytes/placements.
Default private visibility uses official schema 6's omitted `pub(crate)` form.
Unexpected modules at reserved private files, hooks/state changes to the relied-on
native authority/publication files, and state constructor rewrites there reject.
Unrelated Game hooks are not blanket-disabled; this is not a claim to prevent
arbitrary behavior elsewhere in every installed mod.

Eight validator tests exercise positive and negative synthetic plan cases. A
projection onto the preceding complete package plan checked that existing
unrelated shared entries do not conflict; that projection is not an official
new plan. The real complete installed plan passed all 11 central guard groups before
apply at the stated cutoff. The pristine-source checker alone is not sufficient composition
validation, and no guard substitutes for native type or execution tests.


The private lifecycle tests cannot honestly run in a small external fixture:
the state owner is private within the gameplay crate and its source also
implements private Game methods. A fake Game, copied lifecycle structs or
source-only type model would not exercise that owner. The actual applied
`horizon-client --lib` unit-test harness is required and remains deferred;
no codegen job was forced near the disk floor. Four new private tests (and the
two earlier factory tests) therefore remain unrun. Type checking does not
establish live actor/guest resolution, insertion, later owner removal,
networking, persistence or contact gameplay.
