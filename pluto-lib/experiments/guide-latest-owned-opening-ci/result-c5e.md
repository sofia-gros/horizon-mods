# Latest Guide ownership/type attempt: safely stopped

Exact head `c5e4094bc3c4cb1eb55188a13b6dae75fb2a630a`, official Horizon
`f4dcf8a2fb4a68b2d28422f4d34d2be2970e26bd`, run
[42207740778](https://github.com/sofia-gros/horizon-mods/actions/runs/42207740778)
ended **failure**, before Rust fixture compilation or actual Game type checking.
The final receipt has `passed: false`. Its `verified: true` reports final source
seal verification, not native success. No retry was started.

The 13 runner checks, 15 prior-runner checks, original 12-file source guard,
toolchain setup and exact official fetch succeeded. The fresh complete game
subtree scan then found two unreviewed files and refused to continue:

- `game/boat_authority_fixture.rs`: line 75 replaces the player.
- `game/boat_rider_fixture.rs`: line 27 constructs/replaces the player; line 86
  drops the remote connection. The player RHS can fail before assignment.

Both exact official blobs were subsequently retrieved read-only and hash-checked.
They add no book-field accesses, but contain three attachment transitions absent
from the provisional inventory. They are real native fixture methods, not merely
`cfg(test)` text. The Game startup dispatch calls them after mod startup, so
absence of a retained book must not be assumed merely from their fixture names.
`remote.entity_id` changes are also present; the inventory is explicitly an
assignment-spelling audit, not complete alias/field-mutation lifecycle proof.

The next candidate must review these sites rather than excluding them. It needs
21 pinned source files, the existing 79 book accesses and 26 assignment/take
transition entries. Any added invalidation must preserve successful RHS
construction before mutation; an error must not newly invalidate an unchanged
actor. Actual source transformation, reverse hashing and a fresh complete scan
will need a separately reviewed cutoff. No following commit is an implicit retry.

The complete compact console was retrieved (18,518 UTF-8 bytes; SHA-256 in the
adjacent JSON). Raw phase logs remain ephemeral on the CI runner; their hashes
and bounded failure diagnostic are in the console. Minimum free space was
91,223,425,024 bytes, above the unchanged 10 GiB + 128 MiB floor.

The 10 Rust tests, latest BookScreen/Game typing, ordinary Pluto composition,
actual Client/UI, reload, owned opening and delayed LAN reply behavior remain
unproved by this attempt. Existing LAN and ordinary manifests were not changed.
