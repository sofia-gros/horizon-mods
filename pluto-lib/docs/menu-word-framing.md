# Pure string-list framing for signed menu words

`pluto_inventory_api::menu_words` extracts the codec already used by
TConstruct's `source-builder-wire.rs`. Actual send and receive callers carry
pattern IDs, station layouts, material information/JSON, costs and reason strings
through the same codec. Other mods can use this format for their own bounded
string lists without importing any TConstruct schema or screen.

## Contract

`encode_utf8_list(&[String])` returns a complete owned word vector or `None`.
Its first i16 is the nonnegative payload byte count (maximum 32,767). The payload
is a big-endian u16 count followed by big-endian u16 byte lengths and UTF-8 data.
Pairs of bytes retain all bits when represented as signed i16 words. An odd final
byte gets zero padding in the low byte. Count, length and total-size overflow
return no partial frame; the caller's input is never mutated.

`decode_utf8_list(size, words, EmptyStrings)` receives the header separately.
It requires exactly ceil(size/2) words, valid UTF-8, complete count/length/data
fields and no trailing declared bytes. Negative headers are not data frames.
Unused odd-byte padding is ignored, including nonzero padding, just as before.
This is not authenticated or canonical encoding. A malformed count may reserve
up to 65,535 String entries before bounded parsing fails; this extraction does
not claim a new tighter allocation limit.

EmptyStrings::Allow or Reject is an explicit generic structural option. The
TConstruct wrapper selects Reject at the original validation point. Its encoder
still accepts empty strings, preserving the original asymmetric behavior. An
empty list is distinct from a list containing empty fields. No callback, user
code, receiver update, transport or I/O runs inside either operation. Validation
failure only discards local allocations. This is not allocator-failure recovery
or an OOM rollback guarantee.

## Ownership stays in the consumer

Menu data-slot IDs/ranges, negative reset and final commit markers, partial frame
reassembly, sequence checks, window/actor/world lifetime, validation of IDs and
JSON, and UI updates remain in the unchanged send/receive files. The shared
codec has no registry or cache and grants no item or world mutation authority.
Encoding success is not delivery, authorization, a complete received transaction
or proof of an active native menu hook.

Official Horizon 624c4f6265c02296fbe8a80f029dd0d987fb94c0 declares
ContainerSetData with signed i16 id/value fields in
crates/horizon-protocol/src/packets/play.rs. That supports the representation,
not the TConstruct routing/lifecycle contract. No official file or native hook
is changed by this group, and no full LAN/reconnect claim is made.

## Validation and integration

The real Cargo fixture includes the actual wrapper and a byte-exact frozen
original. It retains the three original scenario tests on each implementation,
adds 390 encode/round-trip scenarios, 7,600 malformed-frame comparisons, exact
signed UTF-8 words, noncanonical padding, empty-policy and capacity boundaries.
All decoding rejection results and accepted bytes must remain identical in the
consumer mode. Additional Allow tests demonstrate a generic second policy, not
a new TConstruct behavior.

The existing standalone rustc CI step is replaced only by the fixture's locked,
offline Cargo command. The earlier PLUTO_TEST_FETCH_LOCKED inventory test seeds
the exact same registry dependencies; a static guard checks their identities and
step order. No permissions, billing, native job scope or broad CI changes are
added. During integration, preserve both this narrow step replacement and the
independent manual CPU-mode workflow changes from the rendering group.

Exact source `a709cb16ac5af4aa10bae0d27a0f7e23dee6fdb2` passed eleven Rust
tests and six Python source/dependency guards. The Rust fixture covers 390
encode/round-trip scenarios and 7,600 malformed-frame comparisons plus capacity,
UTF-8, signed-word and empty-field boundaries. All 40 captured API/fixture/lock/
caller/workflow hashes matched before and after the run. Registry package
identities match the pinned official host lock, and the continuous disk floor
held (minimum 10,884,431,872 bytes).

The Cargo fixture compiles the real API crate and actual consumer wrapper. Its
non-test library compilation reports only expected unused-wrapper warnings.
No full API unit suite, native package/apply/build, live transport, reassembly,
reconnect or complete consumer test was run. The source trace and byte codec
proof do not establish those later lifecycle guarantees. The
[receipt](menu-word-validation-20261106.json) records the immutable cutoff,
result, and before/after hashes.
