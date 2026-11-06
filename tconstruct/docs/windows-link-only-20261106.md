# Asset-free guarded Windows link mode

Source-only follow-up to `3da8fe527e286dd433e076771b0e75351503ce78`.
No Windows process, PowerShell parser, formatter, native planner, Cargo compiler,
linker, installer, asset preparation or game is run for this source checkpoint.

## Same official build, earlier stopping point

The existing `play-windows.ps1` / `play_windows_latest.py` entry now accepts
`-BuildOnly` / `--build-only`. It uses the same exact official 41f/5a64 sources,
Rust 1.99.1 Windows GNU toolchain, authenticated formatter archive, same-checkout
TConstruct and Pluto packages, source seals and environment. It retains the
default gameplay audio/controllers, GNU x64 target, `fast` profile and normal
optional-mods feature test. It does not switch to a headless or no-default build.

The existing guarded stage is unchanged: build the official Mixins CLI, create
the complete plan, pass **every** mandatory guard, apply, then compile/link the
client. A rejected stage is still fatal and never enters the game compile. This
mode cannot substitute a previously accepted selected-node projection for the
complete guard. The native planner and build remain real future execution gates.

Only external game resource admission/preparation and game startup are omitted.
Repository content/assets needed by the existing two packagers are still part of
their normal source inputs. No licensed external game assets are needed merely
to compile/link; build scripts still use the complete official source checkout
and its normal locked dependencies. This does not prove that an arbitrary
missing dependency can be ignored.

`-BuildOnly` rejects a simultaneous `-Resources` argument to avoid silently
ignoring selected user data. Normal startup still requires resources and keeps
its existing preparation/startup sequence. No drive or fork search is added.

## Separate output meaning

The mode is included in the input/cache identity. Build-only writes `.linked.json`
with the x64 PE executable hash after compilation and a full input recheck.
It does not write `.ready.json`, prepare resources, create a world directory,
or start the executable. On a later link-only cache hit it rechecks all inputs
and the recorded binary; that is not a fresh compilation measurement. The
normal playtest has a different identity and cannot reuse the link-only receipt
as a ready game. Shared Cargo artifacts remain governed by Cargo's source/lock
fingerprints and the explicit build command.

An x64 PE header plus successful Cargo link does not prove DLL resolution,
startup, rendering, input, audio, multiplayer, frame rate or playability. Even
after a successful link, those remain distinct execution gates. No native build
has succeeded for this new mode yet.

## Explicit invocation after preparation and release

From the single combined checkout, using already prepared inputs:

```powershell
& .\tconstruct\tools\play-windows.ps1 `
  -BuildOnly `
  -HorizonSource 'C:\Prepared\horizon-41f' `
  -MixinsSource 'C:\Prepared\mixins-5a64' `
  -Toolchain 'C:\Prepared\rust-1.99.1-gnu' `
  -RustfmtArchive 'C:\Prepared\archives\rustfmt-1.99.1-x86_64-pc-windows-gnu.tar.gz' `
  -PlaytestHome 'C:\TConstructLinkCheck'
```

Do not supply `-Resources`. Toolchain/formatter, official source and locked
dependency preparation are still required, exactly as described in
[the launch prerequisites](windows-latest-launch.md). The controller performs
no automatic fetch/install and does not bypass missing prerequisites. For a
remote Windows build, the coordinator must separately select the exact integrated
head, verify all complete guards and approve a serial execution slot with measured
disk/RAM/time/process bounds. Existing interactive source code is not a hard
process-group or disk controller. No new trigger/workflow or unbounded CI build
is introduced here.

## Bounded source validation

Nine pure tests use recorded **mock** commands and tiny synthetic PE bytes to
check mode admission, exact two-file inverse, stage-before-link ordering,
guard/link failure, changed input rejection, binary/cache separation, no asset or
game calls in link mode, normal startup retention, and explicit PS routing.
They do not execute the actual stage guard, compiler, Windows API or game.

The seven UTF-8, thirteen formatter and twenty existing latest-launch pure tests
also pass. One existing UTF-8 inverse test first reverses this exact pinned mode
delta, then checks the original UTF-8 delta against its old hash; neither prior
boundary is replaced with a new arbitrary expected body. PowerShell syntax for
the changed entry and the actual Windows build are still unrun.
