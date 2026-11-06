#!/usr/bin/env python3
"""Manual, public standard-runner CPU proof. No cache/artifacts/write credentials.

This is an experiment runner, not ordinary packaging or a gameplay/GPU proof.
All expensive subprocesses have a continuous disk floor, including preparation.
"""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
FLOOR = 10 * 1024**3 + 128 * 1024**2
HORIZON = "624c4f6265c02296fbe8a80f029dd0d987fb94c0"
MIXINS = "e8cd0aa38cffef49cb2e088ef716baa107148c02"
BUNDLE_SHA = "7d9c4c5d7bd91ec6584b9206658f9d5bdb5d04dd3f95f1cfe90ceca6daae5386"
OFFICIAL = {
    "horizon": ("https://github.com/horizon-rust/horizon.git", HORIZON),
    "mixins": ("https://github.com/horizon-rust/horizon-mixins.git", MIXINS),
}
TEST = "mesher::pluto_terrain_cell_cpu::tests::real_native_mesh_differential_and_timing"


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def admit(env, event, input_name="terrain_cpu"):
    """No alternate repository, private runner billing or injected build flags."""
    if input_name not in ("terrain_cpu", "game_borrow"):
        raise ValueError("unknown manual proof mode")
    if (env.get("GITHUB_ACTIONS") != "true"
            or env.get("GITHUB_EVENT_NAME") != "workflow_dispatch"
            or env.get("GITHUB_REPOSITORY") != "sofia-gros/horizon-mods"
            or env.get("RUNNER_OS") != "Linux"
            or event.get("repository", {}).get("full_name") != "sofia-gros/horizon-mods"
            or event.get("repository", {}).get("private") is not False
            or event.get("inputs", {}).get(input_name) not in (True, "true")):
        raise ValueError("only the explicit public-repository manual fixture mode is admitted")
    if not re.fullmatch(r"[0-9a-f]{40}", env.get("GITHUB_SHA", "")):
        raise ValueError("an immutable checked-out commit is required")
    if env.get("RUSTFLAGS") or env.get("CARGO_ENCODED_RUSTFLAGS"):
        raise ValueError("unreviewed Rust flags")


def terminate(process):
    if process.poll() is not None:
        return
    os.killpg(process.pid, signal.SIGINT)
    try:
        process.wait(timeout=20)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait()


class Runner:
    def __init__(self, root, env):
        self.root, self.env = root, env
        self.steps = []
        self.minimum = self.free()

    def free(self):
        stat = os.statvfs(self.root)
        return stat.f_bavail * stat.f_frsize

    def check_space(self):
        self.minimum = min(self.minimum, self.free())
        if self.minimum < FLOOR:
            raise ValueError("continuous 10 GiB + 128 MiB disk floor reached")

    def run(self, label, command, cwd=None, extra_env=None):
        self.check_space()
        path = self.root / (label + ".log")
        env = self.env | (extra_env or {})
        entry = {"step": label, "command": list(map(str, command))}
        self.steps.append(entry)
        start = time.monotonic()
        process = None
        print("::group::" + label, flush=True)
        try:
            with path.open("x") as log:
                process = subprocess.Popen(command, cwd=cwd or REPO, env=env,
                    stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
                while process.poll() is None:
                    self.check_space()
                    time.sleep(0.25)
                self.check_space()
            entry["exit"] = process.returncode
            if process.returncode:
                raise ValueError(f"{label} failed with exit {process.returncode}")
        finally:
            if process is not None:
                terminate(process)
                entry["exit"] = process.returncode
            entry["seconds"] = time.monotonic() - start
            entry["minimum_free_bytes"] = self.minimum
            if path.exists():
                raw = path.read_bytes()
                entry["log_sha256"] = hashlib.sha256(raw).hexdigest()
                # Logs/summary only, no durable artifact/cache storage writes.
                print(raw[-131072:].decode("utf-8", errors="replace"), flush=True)
            print("::endgroup::", flush=True)
        return path


def export(source, revision, output):
    """Executed as a monitored child, with the repository's safe tar reader."""
    if output.exists():
        raise ValueError("export destination already exists")
    archive = load("terrain_portable_archive", REPO / "tconstruct/tools/portable_archive.py")
    output.mkdir()
    process = subprocess.Popen(["git", "-C", str(source), "archive", "--format=tar", revision],
        stdout=subprocess.PIPE)
    try:
        archive.extract_git_tar(process.stdout, output)
        if process.wait():
            raise ValueError("official export failed")
    finally:
        process.stdout.close()
        if process.poll() is None:
            process.terminate()
            process.wait()
    if revision == HORIZON:
        (output / ".horizon-revision").write_text(HORIZON + "\n")


def seal(source, output):
    """All actual applied files, including manifests and Cargo.lock; no targets."""
    if output.exists() or output.resolve().is_relative_to(source.resolve()):
        raise ValueError("seal output must be new and outside the applied source")
    files = {}
    for path in sorted(source.rglob("*")):
        if path.is_symlink():
            raise ValueError("unexpected applied source symlink")
        if path.is_file():
            files[path.relative_to(source).as_posix()] = digest(path)
    if "Cargo.lock" not in files or "Cargo.toml" not in files:
        raise ValueError("applied workspace manifests missing")
    output.write_text(json.dumps(files, sort_keys=True, indent=2) + "\n")
    print(f"Sealed {len(files)} actual applied source files, including Cargo.lock.")


def main():
    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text())
    admit(os.environ, event)
    root = Path(os.environ["RUNNER_TEMP"]) / "pluto-terrain-cpu"
    root.mkdir(exist_ok=False)
    env = os.environ.copy()
    env.update(CARGO_BUILD_JOBS="2", CARGO_INCREMENTAL="0", CARGO_PROFILE_DEV_DEBUG="0",
        CARGO_PROFILE_TEST_DEBUG="0", CARGO_PROFILE_TEST_OPT_LEVEL="0",
        CARGO_PROFILE_RELEASE_DEBUG="0", CARGO_TERM_COLOR="never", RUSTUP_TOOLCHAIN="1.95.0")
    runner = Runner(root, env)
    result = {"status": "failed", "commit": env["GITHUB_SHA"], "horizon": HORIZON,
        "mixins": MIXINS, "bundle_sha256": BUNDLE_SHA, "disk_floor_bytes": FLOOR,
        "profile": "test; opt-level=0; debug=0; incremental=0; jobs=2",
        "scope": "Actual native CPU differential and diagnostic timing using original synthetic assets; no GPU/production-pack/gameplay claim",
        "steps": runner.steps, "runs": []}
    try:
        runner.run("checkout-identity", ["git", "rev-parse", "HEAD"])
        if (root / "checkout-identity.log").read_text().strip() != env["GITHUB_SHA"]:
            raise ValueError("checked-out source differs from dispatch commit")
        runner.run("build-prerequisites-update", ["sudo", "apt-get", "update"])
        runner.run("build-prerequisites", ["sudo", "apt-get", "install", "-y", "--no-install-recommends", "libasound2-dev", "libudev-dev"])
        runner.run("rust-toolchain", ["rustup", "toolchain", "install", "1.95.0", "--profile", "minimal", "--component", "rustfmt"])
        runner.run("toolchain-identity", ["rustc", "-Vv"])
        for name, (url, revision) in OFFICIAL.items():
            checkout, original = root / (name + "-git"), root / name
            runner.run(name + "-init", ["git", "init", str(checkout)])
            runner.run(name + "-origin", ["git", "-C", str(checkout), "remote", "add", "origin", url])
            runner.run(name + "-fetch", ["git", "-C", str(checkout), "fetch", "--depth", "1", "origin", revision])
            runner.run(name + "-checkout", ["git", "-C", str(checkout), "checkout", "--detach", "FETCH_HEAD"])
            identity = runner.run(name + "-identity", ["git", "-C", str(checkout), "rev-parse", "HEAD"])
            if identity.read_text().strip() != revision:
                raise ValueError("official revision mismatch")
            runner.run(name + "-export", [sys.executable, str(HERE / "ci.py"), "--export", str(checkout), revision, str(original)])
        original, mixins = root / "horizon", root / "mixins"
        cli_target, target = root / "mixins-target", root / "host-target"
        target.mkdir()
        runner.run("build-official-mixins", ["cargo", "build", "--release", "--locked", "-p", "horizon-mixins-cli", "--target-dir", str(cli_target)], cwd=mixins)
        cli = cli_target / "release/horizon-mixins"
        bundle, fixture, plan, stage = root / "plutolib.hnmod", root / "fixture", root / "plan.json", root / "stage"
        runner.run("package-exact-library", [sys.executable, str(REPO / "pluto-lib/tools/package_native.py"), str(bundle)])
        if digest(bundle) != BUNDLE_SHA:
            raise ValueError("ordinary package differs from the verified v15 archive")
        runner.run("prepare-fixture", [sys.executable, str(HERE / "prepare.py"), "--official", str(original), "--bundle", str(bundle), "--output", str(fixture)])
        runner.run("official-plan", [str(cli), "plan", "--workspace", str(original), "--mods", str(fixture / "mods"), "--out", str(plan)])
        runner.run("all-production-guards", [sys.executable, str(REPO / "pluto-lib/tools/check_native_plan.py"), str(plan), str(original)])
        runner.run("fixture-plan-guard", [sys.executable, str(HERE / "check_plan.py"), str(fixture), str(plan)])
        runner.run("official-apply", [str(cli), "apply", "--workspace", str(original), "--mods", str(fixture / "mods"), "--out", str(stage)])
        runner.run("verify-fixture-sources", [sys.executable, str(HERE / "prepare.py"), "--output", str(fixture), "--verify"])
        result["plan_sha256"] = digest(plan)
        result["fixture_sources_sha256"] = digest(fixture / "fixture-sources.json")
        before, after = root / "stage-source-before.json", root / "stage-source-after.json"
        runner.run("seal-applied-stage", [sys.executable, str(HERE / "ci.py"), "--seal", str(stage), str(before)])
        checker = load("terrain_cpu_log_checker", HERE / "run.py")
        for mode in ("no-provider", "registered"):
            # First invocation fetches only this target's locked dependencies.
            # The second runs offline. No Cargo cache is uploaded or restored.
            command = ["cargo", "test", "--locked"] + (["--offline"] if mode == "registered" else [])
            command += ["--manifest-path", str(stage / "Cargo.toml"), "-p", "horizon-client-render", "--lib", TEST, "--", "--exact", "--nocapture", "--test-threads=1"]
            log = runner.run("native-" + mode, command, cwd=stage, extra_env={
                "CARGO_TARGET_DIR": str(target), "PLUTO_TERRAIN_FIXTURE_RESOURCES": str(fixture / "resources"),
                "PLUTO_TERRAIN_FIXTURE_MODE": mode, "PLUTO_TERRAIN_PROFILE_LABEL": result["profile"],
                "PLUTO_TERRAIN_TIMING_ITERS": "64"})
            result["runs"].append({"mode": mode, "timings": checker.checked_log(log.read_text(), mode)})
        runner.run("seal-final-stage", [sys.executable, str(HERE / "ci.py"), "--seal", str(stage), str(after)])
        if before.read_bytes() != after.read_bytes():
            raise ValueError("actual applied source or Cargo.lock changed during the CPU runs")
        runner.run("verify-fixture-sources-after", [sys.executable, str(HERE / "prepare.py"), "--output", str(fixture), "--verify"])
        result["unchanged_applied_source_seal_sha256"] = digest(before)
        result["sealed_stage_files"] = len(json.loads(before.read_text()))
        result["status"] = "passed"
    except BaseException as error:
        result["error"] = f"{type(error).__name__}: {error}"
        raise
    finally:
        result["minimum_free_bytes"] = runner.minimum
        (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        print("PLUTO_TERRAIN_CI_RESULT " + json.dumps(result), flush=True)
        summary = os.environ.get("GITHUB_STEP_SUMMARY")
        if summary:
            with open(summary, "a") as output:
                output.write("## Terrain CPU fixture: " + result["status"] + "\n\n")
                output.write("Actual official CPU mesher with original synthetic assets. **No GPU display proof.**\n\n")
                output.write("Commit: " + result["commit"] + "\n\nMinimum free bytes: " + str(runner.minimum) + "\n\n")
                for run in result["runs"]:
                    for timing in run["timings"]:
                        output.write(f"- {timing['case']}: median adapter/native ratio {timing['median_ratio']:.4f}, unoptimized diagnostic.\n")
                if "error" in result:
                    output.write("\nFailure: " + result["error"].replace("\n", " ") + "\n")


if __name__ == "__main__":
    if len(sys.argv) == 5 and sys.argv[1] == "--export":
        export(Path(sys.argv[2]), sys.argv[3], Path(sys.argv[4]))
    elif len(sys.argv) == 4 and sys.argv[1] == "--seal":
        seal(Path(sys.argv[2]), Path(sys.argv[3]))
    elif len(sys.argv) == 1:
        main()
    else:
        raise SystemExit("no unreviewed CI mode or arguments")
