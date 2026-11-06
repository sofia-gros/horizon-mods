# Owned Guide opening: the native tail is part of the operation

Read-only design on official Horizon `f4dcf8a2`. The five exact source regions
and the observed TConstruct caller are pinned in the adjacent JSON. This is a
separate follow-up to excluded candidate `29f8b52d`; neither it nor CI candidate
`c5e4094b` is edited here. No hook, native extension, public opening API, locale
listener or LAN connection is installed. Latest type/model execution is pending.

## What must happen once

Native `books::open_book` assigns the new BookScreen, clears keys, sets `right`
to false, then calls `capture(false)`. It does not merely assign a book pointer.
`capture(false)` first handles guest context, cancels inventory press/drag state,
stops mining and relinquishes the agent pointer; it then handles split-screen,
window, cursor position/grab and visibility. Missing windows and non-keyboard
players have native early returns. Those are native behavior, not evidence that
no book was installed.

`stop_mining` may update the tutorial, stops the current destroying operation,
clears attack/use clicks, requests a remote mining stop and clears local mining.
The remote request sets a flag consumed by a later remote input phase. This is
not proof that a network packet was sent by the opening itself. This pass checks
the named direct functions, not every transitive native callback, destructor or
foreign Mixins hook. The whole call tree/composition still needs review.

Current TConstruct builds pages, increments its wrapping sequence, clears back
history, invokes native open and only then sends the remote hand/sequence request.
Moving an owner into this sequence must not silently move the send before native
side effects, run the tail twice, or consume a pending request before admission.
Calling native open after an owned installation would create another opening and
invalidate that owner. It is not an acceptable way to reuse the tail.

## Smallest future operation, not yet an implementation

A private Game operation would need one authorized installation and one exact
native tail. A shared private tail extracted mechanically from native open could
avoid duplicated key/capture logic, but that is another proposed host delta. It
must preserve the original open path too, and requires inverse-source and hook
composition guards. No ordinary Pluto API can claim that it exists yet.

The existing input-only current-book gate is not automatically the right opening
or retained-refresh policy. A future operation needs an explicit client and actor
admission contract, plus an actual caller authorized to open that book. Display
titles, root markers, matching pages, read-only observations and an accepted
mutable binding cannot mint that authorization. This design adds no such gate.

The phases and return contracts must stay distinct:

| Phase | Possible result and allowed behavior |
| --- | --- |
| Admission and checked identity allocation, before installation | Reject with the original candidate; old slot and pending consumer state remain unchanged. No native tail or send has run. |
| Installation | New content has entered the slot. The old value can be retained for explicit disposal. This is the first effect boundary, not a reversible preparation. |
| Native tail, exactly once | Input, mining and window effects may occur. A panic has an attempted/unknown outcome; do not retry or restore the old screen automatically. |
| Final owner/context validation | Check attachment and opening after all possible tail callbacks. Revision-only mutation is not an identical reopen. Replacement/reset during the tail means the newly minted owner must not escape as a valid handle. |
| Return and old-value disposal | Release any exclusive storage/context borrow before generic old-value destruction. A later destructor can cause another legitimate mutation or panic; it cannot trigger a hidden rollback. |

Once installation or tail effects occurred, rejection cannot promise to return an
untouched candidate. The slot or a callback may have consumed or replaced it.
A superseded outcome can return any still-owned old value, but must not take an
unrelated current book merely to manufacture a candidate return. Failure-before-
effect and superseded/unknown-after-effect therefore need different result types.
A final same-owner observation establishes only that instant, not future validity.
Every subsequent replacement must revalidate its own owner and current revision.

## The destructor-order difference cannot be hidden

Original native assignment drops the previous concrete BookScreen before the
three-line input/capture tail. The generic owned installer instead returns the
previous value without dropping it. Keeping that value until the operation returns
moves destruction after the tail. This is a real ordering difference, even if
current Text/EditBox values appear inert; this pass has not proved their complete
Drop graph or foreign-hook behavior.

Two choices require separate review: retain exact native destruction timing for
this trusted concrete type, or explicitly define a new operation that returns the
old screen after the tail. The latter is not automatically native-open equivalence.
Do not add a generic callback or early drop merely to make a modeled oracle pass.
The current model's safe-return contract and original native ordering are both
preserved as facts until that decision is justified.

## Tests that would answer the remaining questions

A next candidate should separately test: allocation/admission failure with exact
candidate return; one installation/tail; a tail replacing the book with identical
pages; attachment reset; page-only mutation; tail panic; previous-value destruction
before versus after the native tail; and no owner/send after supersession. A model
can check ordering and ownership, but cannot prove actual Game/window behavior.
Actual BookScreen typing and complete-plan checks are another gate; real Client
prompt/overlay/controller/close/reopen behavior still needs an appropriate fixture.

Locale rebuild and LAN reply commit remain later operations. They additionally
need catalog/resource consistency, a stable book identity, pending-request binding,
post-observer revalidation and the legacy full-envelope admission contract. An
owned-open tail alone fixes none of those. The original LAN feature remains active
and unchanged while this source-only boundary is investigated.
