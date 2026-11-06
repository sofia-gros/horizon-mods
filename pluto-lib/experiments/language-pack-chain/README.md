# WIP: native resource-pack to Mod language chain

**Source only: six Rust cases are uncompiled and unrun. No runner or trigger is
created.** This is separate from the passed eight-case Mod overlay baseline at
`ce495a88` / run `42195249104`; no evidence is borrowed as execution of these cases.

The fixture calls real public `packs::folder`, `enabled`, `server_pack` and
`build_overlay`, then `mod_content::build` and native `Language::load`. It does
not copy a private merger, implement a dictionary, or alter precedence. Small
folder packs and a real ZIP are owned original resources. The ZIP writer is the
same `zip = 9.1.3` already in the official lock, not a new online dependency.
The `.hmod` uses actual Package/Content admission separately from server packs.

## Six intended cases

1. Selected order differs from folder discovery order: native key merging keeps
   lower-only keys and later selected conflicts; a subsequent Mod file replaces
   the whole prepared locale and loses those lower-only keys.
2. An explicitly supplied server ZIP appended above selected packs wins pack
   conflicts; Mod whole-file replacement still follows it. This reproduces the
   ordering in official `startup_root`, not actual server approval/selection.
3. A pack filter removes matching vanilla and earlier-pack files, not its own
   supplied locale. A later Mod file replaces that surviving self-layer.
4. A filter without a replacement leaves the locale absent. An explicit Mod
   file adds only its own keys, never reviving the filtered lower catalog.
5. Malformed strict pack JSON is skipped while valid lower layers survive;
   malformed Mod locale bytes still replace the whole selected locale and native
   Language falls back to English. No strict/lenient policy is silently aligned.
6. Calling the public pack builder twice with unchanged inputs preserves the
   output, while Mod English replacement leaves the selected Japanese pack
   locale intact and exact missing locale `ja` uses English, not regional guessing.

All roots and outputs are exclusively-created scratch paths. Native pack cache
locking/stamps remain in use; this fixture does not call the private uncached
helper or delete a cache to force success. Its unchanged-input output comparison
does not prove a cache hit was taken or any new policy's invalidation. Snapshots
do not follow links and include link targets, regular bytes and directories;
they make no permission-mode or timestamp preservation promise. Required assets
and pack discovery fail assertions rather than silently skipping a case.

## Separate from a repair

This is still baseline characterization. No writer hook, overwrite, language
setter, cache policy version, second registry or production consumer is added.
The minimal air facts/gameplay/items tables are not a playable resource bundle.
Game admission/pack settings, server authorization, Options, startup and staged
reload, rendering, retained Guide/LAN refresh remain unproved.

Before a later key-retention repair, the eligible strict-string locale domain,
legacy lenient fallback, intentional `{}` change and exact Mod-wins conflicts
need an explicit compatibility decision. The prepared root must be the sole
lower input; filtered/raw packs must not be resurrected. The private writer and
content cache stamp require separate exact-source composition and native proof,
not a recursion-prone HEAD hook or a presumed operation wrapper. The current
small-writer/cache candidate is documented in
`../../docs/language-overlay-compatibility.md` and remains unimplemented.

## Future validation, not run now

Use an independently reviewed, separately queued CI with the actual official
`624c4f62` crates, official seeded lock and the exact three root local patches
already established by `language-overlay-ci/prepare.py`. Every resolved package
identity and local path must match; new online substitutions are not allowed.
Require actual renderer/client-lib/core/mods and fixture compilation, all six
exact tests once, before/after checkout/lock/file-set/generated-source seals and
a receipt that distinguishes this chain from Game/UI or priority repair.

Small Python source guards are not native typing or behavior evidence. The old
eight-case fixture, corrected controller and its successful trigger are unchanged.
