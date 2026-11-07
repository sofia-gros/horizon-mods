# Guide lifetime: actual Client validation route

## Finding and scope

An existing official real-client book harness can be reused as a baseline:
`tools/lan_e2e.py --check books`. This is a more concrete route than inventing a
mock Game. It has **not been run here**, and it does not establish a guide
session's lifetime or authorize delayed replies.

This read-only audit uses official Horizon
`624c4f6265c02296fbe8a80f029dd0d987fb94c0` and Mixins
`e8cd0aa38cffef49cb2e088ef716baa107148c02`. Exact file hashes, line ranges and
bounded environment observations are in
[the prerequisite record](guide-client-validation-prerequisites-20261106.json).
No fork, unknown asset tree, official source edit, build or game run was used.

The ordinary correlation API's [v12 proof](guide-request-correlation.md) remains
closed and unchanged. The native lifetime experiment remains uninstalled.
Nothing in this audit enables screen updates, reply application or transport.

## Existing test routes and their actual limits

- `tools/lan_e2e.py:7543` drives two real client windows, one backed by its
  integrated server and one connected over LAN. The `books` case edits and signs
  a writable book, opens its written form, closes it, checks server/other-player
  synchronization and captures screenshots. The harness launches offscreen when
  requested, retains Client execution and isolates game homes.
- This case calls `book sign` and `book done` through the control socket.
  `Game::book_control` calls native methods directly. Its response contains
  page index, page count and lectern position, **not page content or an opening
  generation**. Despite the command documentation's wording, that response
  alone cannot distinguish identical books or validate keyboard/mouse admission.
- `tools/agent_control_e2e.py --scenario ui` exercises native menus through widget
  control. It is another renderer/control smoke route, not a guide lifetime test.
- `remote.rs::mod_chat_needs_a_joined_player_and_what_the_chat_option_allows`
  is a resource-dependent ignored test using actual Client role and `Pipe::pair`.
  It loads with the headless resource flag and constructs Game without a renderer.
  It is useful as a connection-test reference, but cannot pass the current-book
  presentation gate honestly.
- No ready-made custom-guide delayed-payload/opening-generation test was found
  in these inspected harnesses, control endpoints or existing Pluto guide tests.
  This is a bounded search conclusion, not a claim about all upstream code.

`--offscreen` is not `--headless`. The former still constructs a window and a
Renderer, drawing to a texture; the latter has neither. On Linux, an X/Wayland
display and functioning wgpu backend are therefore still required for the
offscreen renderer route. Official documentation describes Xvfb plus software
Vulkan. Offscreen success would establish native renderer/control behavior,
not physical window focus, OS input delivery or audible sound.

## Prerequisites and bounded availability check

1. A provenance-checked, privately prepared resource tree for this exact official
   target is required. Official preparation consumes the user's own matching
   client/server jars, version manifest, libraries and asset store. It validates
   the inputs against `research-lock.json`; it is not an asset download route.
   The prepared tree must not be committed or publicly redistributed.
2. `horizon-prepare --check --root OFFICIAL_ROOT --output APPROVED_RESOURCES`
   checks layout/version, output stamp and selected extras. Its own help says
   it does **not** validate all file contents. Record the preparation receipt and
   asset hashes as well; a manifest's existence is not provenance proof.
3. The actual loader needs the full GUI/font/panorama and render/gameplay resource
   graph. The synthetic BlockRegistry/Behaviours fixture cannot replace it.
   LAN testing additionally requires `registries.json` and `commands.json`
   together in the same directory chosen by `require_server_reports`.
4. A runnable client and `horizon-ctl` built from the exact approved official
   plus Pluto package are needed. Previous `cargo check` success does not supply
   those linked executables. A new build must be budgeted separately.
5. A renderer-capable display/backend and adequate disk/memory are needed. Keep
   the existing serial build slot and continuous 10 GiB +128 MiB disk floor.

At audit time, `HORIZON_TEST_RESOURCES`, `DISPLAY` and `WAYLAND_DISPLAY` were
unset. The known official default resource manifest and imported-game directory
were absent. `vscreen`, `Xvfb`, `xvfb-run`, `vulkaninfo`, `horizon-client` and
`horizon-ctl` were not in PATH, and `/dev/dri` was absent. This does not prove
that software Vulkan is impossible or that no resources exist elsewhere. No
unapproved search of other asset directories was performed. Free space was
11,053,649,920 bytes at the recorded observation, leaving little build margin.

## Reproducible next run, once those prerequisites are provided

Use an approved isolated generated official execution tree, never modify the
pristine checkout to install assets. The upstream harness resolves resources
relative to its own tree as `.local/horizon-client/<pinned version>`; it does not
read `HORIZON_TEST_RESOURCES` as a replacement for that harness path. Arrange an
approved read-only resource link there, and use `--bin-dir` to avoid an implicit
unbudgeted build. The following is a **future command**, not an executed result:

```sh
HORIZON_E2E_HEADLESS=1 xvfb-run -a python3 "$APPROVED_STAGE/tools/lan_e2e.py" \
  --bin-dir "$EXACT_LINKED_BIN_DIR" --check books --out "$SCRATCH_RESULTS"
```

Here the harness's `HORIZON_E2E_HEADLESS` selects **offscreen**, not the game's
rendererless `--headless`. Verify the resolved adapter and actual Renderer in
the run evidence. Keep all game homes, game folders, saves, options and telemetry
settings isolated with the official harness environment; do not use player data.

First run the unchanged native baseline. Then run the same baseline with the
ordinary Pluto package and no guide consumer, checking for regressions. Only a
separately reviewed test overlay may activate the lifetime candidate. It must
retain the production Client/renderer/actor gates and must not reinterpret the
old TConstruct protocol. No such activation overlay is implemented by this audit.

## Required additional guide experiment assertions

The baseline's screenshots and page count are insufficient. A test-only adapter
must expose an opening/correlation trace and applied-update count without
granting those observations authority. Its expected outcomes must be independent
of the implementation under test.

- Close and reopen identical content: the old capability cannot affect the new
  opening, including two operations between ordinary frame observations.
- Direct prompt restoration and every take/save/close path: revocation does not
  resurrect on equal content. Lectern/editor/server/guest paths remain rejected.
- Same-key player reset, dimension swap, reconfiguration, reconnect and early
  closing-save: old work cannot become valid again; conservatively early
  invalidation remains an explicitly allowed availability loss.
- Use actual native incoming-packet dispatch for delayed, reordered and duplicate
  replies. Verify the official observer occurs exactly once before any effect.
  A real consumer-defined test protocol or explicit test transport is required;
  the in-process RequestId API is not already a wire envelope or sender identity.
- While hidden/loading/reconfiguring, a matching reply is dropped and cannot
  replay when the context becomes visible. A wrong reply must not consume the
  newer valid pending request.
- Two clients and independent openings remain isolated. Checked token exhaustion
  fails closed; the existing pure exhaustion test remains useful but is not
  evidence that native invalidators execute.
- Exercise keyboard, mouse/page links and relevant controller/IME paths
  separately. Direct `book next` calls prove native action semantics, not
  whether a physical input was admitted, consumed once or correctly ignored.
- Record no-consumer behavior and conflicting whole-plan hook rejection. Keep
  visible focus/OS input and audible output distinct from offscreen evidence.

## Safe library boundaries and what is still missing

The installed bounded Text codec, read-only post-native payload observer,
synchronous expected-current page turn and caller-owned request tracker are
useful and already have separate proofs. They do not need a second registry,
parallel component schema or guessed persistent owner.

The missing durable boundary is an **explicit presentation capability** created
only by a reviewed opening operation, revoked at all native replacements/resets,
and consumed by conditional reply application in the same validated current
context. The present read-only payload subscriber must not silently gain mutable
Game access. A new effectful route must preserve native observer ordering and
single-use revalidation; its test overlay and whole-plan guard are still work,
not safe installed API today.

A separate admitted-input capability would need proof at every relevant input
route; stored book presence is not permission to consume an input. The current
single-call page-turn API deliberately makes no such claim. Adding a generic
open/replace/close convenience method now would not solve either lifetime or
input admission, and this audit does not recommend enabling it as a shortcut.

For a future stable official API, exact per-presentation generation, replacement
and close notifications (including identical reopening/direct restoration),
client player-reset notification independent of level epoch, and conditional
operations tied to those identities would reduce maintenance. These are concrete
upstream interface proposals, not proof that official changes are mandatory for
the narrower Mixins candidate and not changes made to the official repository.

**Stopping point:** prerequisite and test-route investigation is complete. The
live guide UI/lifetime/transport proof remains blocked and unrun. Preserve the
ordinary verified APIs and the excluded experiment; do not enable unproved
screen or network behavior merely to complete this checklist.
