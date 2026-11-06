#!/usr/bin/env python3
"""Manual standard-runner compilation of the real applied-v15 Game probes.

No game/renderer execution, source mocks, raw rustc replay or local cache reuse.
The positive compile must pass before one specific negative E0499 is accepted.
"""
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import signal
import subprocess
import sys
import time
import zipfile
import importlib.util

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
TERRAIN = HERE.parent / "terrain-cell-cpu"
spec = importlib.util.spec_from_file_location("shared_official_ci", TERRAIN / "ci.py")
shared = importlib.util.module_from_spec(spec)
spec.loader.exec_module(shared)
checks = shared.load("game_borrow_checks", HERE / "checks.py")
SOURCE = "crates/horizon-client/src/game.rs"
GAME_SHA = "e02eef2d2d72b5e01b6ef9efc46de5652f33270966b2fbbb91398035d4cb3fdb"
LOCK_SHA = "3996049ef015fefe6fb83b4d14344f2be670a2a530ed62eb5e811eaf1469765d"
# Canonical sorted-key serialization of the complete verified v15 source map.
STAGE_SEAL_SHA = "ff7c2320c2e9d603ca99138c5d722e7241166da89bd99bab44946b2e6397bdde"
PROBES = {
    "positive": "2199396795b3c26b94f618bbb243b00ed980705c45dce7028d807158a9518b1e",
    "negative": "5033febfe81224d4582f59a41bf470a35c19e5452c201d4314461b400032d01f",
}


def digest_bytes(raw):
    return hashlib.sha256(raw).hexdigest()


def file_map(root):
    found = {}
    for path in sorted(root.rglob("*")):
        if path.is_symlink():
            raise ValueError("unexpected source/generated-input symlink")
        if path.is_file():
            found[path.relative_to(root).as_posix()] = shared.digest(path)
    return found


def checked_probe(kind):
    if kind not in PROBES:
        raise ValueError("unknown probe")
    raw = (HERE / f"game_borrow_{kind}.rs").read_bytes()
    if digest_bytes(raw) != PROBES[kind]:
        raise ValueError("frozen borrow probe changed")
    return raw


def expected_overlay(base, original, kind):
    if base.get(SOURCE) != GAME_SHA or base.get("Cargo.lock") != LOCK_SHA:
        raise ValueError("not the reviewed v15 Game/lock")
    if digest_bytes(original) != GAME_SHA:
        raise ValueError("original Game bytes changed")
    content = original + b"\n" + checked_probe(kind)
    return content, {**base, SOURCE: digest_bytes(content)}


def unpack(bundle, output):
    """Only exact packaged production Mixins sources; no fixture hook added."""
    if output.exists() or shared.digest(bundle) != shared.BUNDLE_SHA:
        raise ValueError("new destination and exact v15 package required")
    prefix = "mixins/pluto_native_creative/"
    members = {}
    with zipfile.ZipFile(bundle) as archive:
        for item in archive.infolist():
            if not item.filename.startswith(prefix):
                continue
            relative = PurePosixPath(item.filename[len(prefix):])
            if (relative.is_absolute() or ".." in relative.parts or not relative.parts
                    or item.is_dir() or relative.as_posix() in members):
                raise ValueError("invalid or duplicate package path")
            members[relative.as_posix()] = archive.read(item)
    if "horizon-mixin.toml" not in members:
        raise ValueError("missing production manifest")
    for relative, data in members.items():
        path = output / "pluto_native_creative" / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


def generated_inputs(target):
    # The complete native build-script OUT_DIR set, not just direct externs.
    # First compile creates these normally. The negative must preserve them.
    return {str(path): file_map(path) for path in sorted((target / "debug/build").glob("horizon-*/out"))
        if path.is_dir()}


class Runner(shared.Runner):
    def check(self, label, command, cwd, expected_exit):
        """Keep Cargo JSON stdout separate from verbose rustc stderr."""
        self.check_space()
        out, err = self.root / (label + ".jsonl"), self.root / (label + ".stderr")
        entry = {"step": label, "command": list(map(str, command)), "expected_exit": expected_exit}
        self.steps.append(entry)
        process = None
        start = time.monotonic()
        print("::group::" + label, flush=True)
        try:
            with out.open("x") as stdout, err.open("x") as stderr:
                process = subprocess.Popen(command, cwd=cwd, env=self.env,
                    stdout=stdout, stderr=stderr, start_new_session=True)
                while process.poll() is None:
                    self.check_space()
                    time.sleep(0.25)
                self.check_space()
            if process.returncode != expected_exit:
                raise ValueError(f"{label}: expected {expected_exit}, got {process.returncode}")
        finally:
            if process is not None:
                shared.terminate(process)
                entry["exit"] = process.returncode
            entry["seconds"] = time.monotonic() - start
            entry["minimum_free_bytes"] = self.minimum
            for path in (out, err):
                if path.exists():
                    raw = path.read_bytes()
                    entry[path.suffix + "_sha256"] = digest_bytes(raw)
                    print(raw[-131072:].decode("utf-8", errors="replace"), flush=True)
            print("::endgroup::", flush=True)
        calls = checks.rustc_invocations(err.read_text())
        host = [call for call in calls if (call["crate"], call["crate_type"]) == ("horizon_client", "lib")]
        if len(host) != 1 or not host[0]["out_dirs"]:
            raise ValueError("actual gameplay library was not checked exactly once with native generated inputs")
        return out.read_text(), host[0]


def verify_map(stage, expected):
    if file_map(stage) != expected:
        raise ValueError("unexpected applied-source, file-set or Cargo.lock change")


def main():
    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text())
    shared.admit(os.environ, event, "game_borrow")
    root = Path(os.environ["RUNNER_TEMP"]) / "pluto-game-borrow"
    root.mkdir(exist_ok=False)
    env = os.environ.copy()
    target = root / "host-target"
    env.update(CARGO_TARGET_DIR=str(target), CARGO_BUILD_JOBS="1", CARGO_INCREMENTAL="0",
        CARGO_PROFILE_DEV_DEBUG="0", CARGO_PROFILE_TEST_DEBUG="0", CARGO_PROFILE_RELEASE_DEBUG="0",
        CARGO_TERM_COLOR="never", RUSTUP_TOOLCHAIN="1.95.0", PYTHONDONTWRITEBYTECODE="1")
    runner = Runner(root, env)
    result = {"status": "failed", "commit": env["GITHUB_SHA"], "horizon": shared.HORIZON,
        "mixins": shared.MIXINS, "bundle_sha256": shared.BUNDLE_SHA, "steps": runner.steps,
        "positive_compile": False, "negative_expected_rejection": False,
        "runtime_executed": False, "gpu_executed": False,
        "scope": "Real applied-v15 Game library compile-only positive/negative borrow probes; no actor/continuation/gameplay authority proof"}
    source_inputs = file_map(HERE)
    try:
        for kind in PROBES:
            checked_probe(kind)
        identity = runner.run("checkout-identity", ["git", "rev-parse", "HEAD"])
        if identity.read_text().strip() != env["GITHUB_SHA"]:
            raise ValueError("checkout differs from manual dispatch commit")
        runner.run("build-prerequisites-update", ["sudo", "apt-get", "update"])
        runner.run("build-prerequisites", ["sudo", "apt-get", "install", "-y", "--no-install-recommends", "libasound2-dev", "libudev-dev"])
        runner.run("rust-toolchain", ["rustup", "toolchain", "install", "1.95.0", "--profile", "minimal", "--component", "rustfmt"])
        runner.run("toolchain-identity", ["rustc", "-Vv"])
        for name, (url, revision) in shared.OFFICIAL.items():
            checkout, original = root / (name + "-git"), root / name
            runner.run(name + "-init", ["git", "init", str(checkout)])
            runner.run(name + "-origin", ["git", "-C", str(checkout), "remote", "add", "origin", url])
            runner.run(name + "-fetch", ["git", "-C", str(checkout), "fetch", "--depth", "1", "origin", revision])
            runner.run(name + "-checkout", ["git", "-C", str(checkout), "checkout", "--detach", "FETCH_HEAD"])
            identity = runner.run(name + "-identity", ["git", "-C", str(checkout), "rev-parse", "HEAD"])
            if identity.read_text().strip() != revision:
                raise ValueError("official revision mismatch")
            runner.run(name + "-export", [sys.executable, str(TERRAIN / "ci.py"), "--export", str(checkout), revision, str(original)])
        official, mixins = root / "horizon", root / "mixins"
        cli_target = root / "mixins-target"
        runner.run("build-official-mixins", ["cargo", "build", "--release", "--locked", "-p", "horizon-mixins-cli", "--target-dir", str(cli_target)], cwd=mixins)
        cli = cli_target / "release/horizon-mixins"
        bundle, mods, plan, stage = root / "plutolib.hnmod", root / "mods", root / "plan.json", root / "stage"
        runner.run("package-exact-library", [sys.executable, str(REPO / "pluto-lib/tools/package_native.py"), str(bundle)])
        runner.run("extract-exact-library", [sys.executable, str(HERE / "ci.py"), "--unpack", str(bundle), str(mods)])
        runner.run("official-plan", [str(cli), "plan", "--workspace", str(official), "--mods", str(mods), "--out", str(plan)])
        runner.run("all-production-guards", [sys.executable, str(REPO / "pluto-lib/tools/check_native_plan.py"), str(plan), str(official)])
        runner.run("official-apply", [str(cli), "apply", "--workspace", str(official), "--mods", str(mods), "--out", str(stage)])
        before = root / "stage-source-before.json"
        runner.run("seal-applied-stage", [sys.executable, str(TERRAIN / "ci.py"), "--seal", str(stage), str(before)])
        if shared.digest(before) != STAGE_SEAL_SHA:
            raise ValueError("full actual applied stage differs from reviewed v15, before adding either probe")
        base = json.loads(before.read_text())
        original = (stage / SOURCE).read_bytes()
        result["base_source_seal_sha256"] = STAGE_SEAL_SHA
        result["base_source_files"] = len(base)
        generated = None
        for kind in ("positive", "negative"):
            verify_map(stage, base if kind == "positive" else expected)
            if file_map(HERE) != source_inputs:
                raise ValueError("probe source changed")
            content, expected = expected_overlay(base, original, kind)
            (stage / SOURCE).write_bytes(content)
            verify_map(stage, expected)
            command = ["cargo", "check", "--locked"] + (["--offline"] if kind == "negative" else [])
            command += ["--manifest-path", str(stage / "Cargo.toml"), "-p", "horizon-client", "--lib",
                "--no-default-features", "--message-format=json", "-vv"]
            messages, host = runner.check(kind, command, stage, 0 if kind == "positive" else 101)
            verify_map(stage, expected)
            current_generated = generated_inputs(target)
            if any(path not in current_generated for path in host["out_dirs"]):
                raise ValueError("unsealed actual host OUT_DIR")
            if kind == "positive":
                generated = current_generated
                result["positive_compile"] = True
                result["positive_rustc"] = host
                (root / "generated-inputs-before-negative.json").write_text(json.dumps(generated, sort_keys=True, indent=2) + "\n")
            else:
                if current_generated != generated:
                    raise ValueError("native generated inputs changed for the negative probe")
                result["negative_diagnostic"] = checks.reject_diagnostic(messages, content)
                result["negative_rustc"] = host
                result["negative_expected_rejection"] = True
                (root / "generated-inputs-after-negative.json").write_text(json.dumps(current_generated, sort_keys=True, indent=2) + "\n")
        (stage / SOURCE).write_bytes(original)
        verify_map(stage, base)
        after = root / "stage-source-after.json"
        runner.run("seal-restored-stage", [sys.executable, str(TERRAIN / "ci.py"), "--seal", str(stage), str(after)])
        if before.read_bytes() != after.read_bytes() or file_map(HERE) != source_inputs:
            raise ValueError("final applied/probe sources changed")
        if shared.digest(bundle) != shared.BUNDLE_SHA:
            raise ValueError("package changed during checks")
        result["unchanged_native_generated_inputs"] = len(generated)
        result["status"] = "passed_compile_probes_only"
    except BaseException as error:
        result["error"] = f"{type(error).__name__}: {error}"
        raise
    finally:
        result["minimum_free_bytes"] = runner.minimum
        (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        print("PLUTO_GAME_BORROW_CI_RESULT " + json.dumps(result), flush=True)
        summary = os.environ.get("GITHUB_STEP_SUMMARY")
        if summary:
            with open(summary, "a") as output:
                output.write("## Actual Game borrow probes: " + result["status"] + "\n\n")
                output.write("Compile-only on the real official Game. **No actor/continuation, CPU-renderer or GPU execution proof.**\n\n")
                output.write("Commit: " + result["commit"] + "\n\n")
                output.write(f"Positive compile: {result['positive_compile']}; exact negative E0499: {result['negative_expected_rejection']}.\n\n")
                output.write("Minimum free bytes: " + str(runner.minimum) + "\n")
                if "error" in result:
                    output.write("\nFailure: " + result["error"].replace("\n", " ") + "\n")


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "--unpack":
        unpack(Path(sys.argv[2]), Path(sys.argv[3]))
    elif len(sys.argv) == 1:
        main()
    else:
        raise SystemExit("unsupported invocation")
