#!/usr/bin/env python3
"""Explicit serial-slot experiment; all generated source/targets/temp live in /tmp.

Never starts CI, downloads dependencies, edits official sources, or copies a
shared target. Cargo's access bookkeeping can change; registry contents cannot.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
from pathlib import PurePosixPath
import sys
import time

sys.dont_write_bytecode = True
import ci

TMP_FLOOR = 512 * 1024**2
MEM_FLOOR = 1536 * 1024**2
CLI_SHA = "50fd3105daafd09525889c11c2374a727dbd41b0bad0a5b8714c01b0d06866ff"
FROZEN = {
    "ci.py": "14f27b62322dbd26e6e22e7773845025c4217ab1bdbe4706ab856b80ba470836",
    "fixture.rs": "faa12fa95062dc8f9bf38f28e770107e8246fedbd658de630d6875b858357cbe",
    "prepare.py": "8e03f07f677c3a8c90c37b3b3df5e05068abe93641658b8ea37e1edf1d89993b",
    "run.py": "b322c54fa95b5840f0d58f2dcd5caf9bd4c7b17be2a41dae9f9337586db1d036",
    "check_plan.py": "71ac5f2ac043c4c8c0170ba05e2812a8b4c93793deea4ed11bbc3d3ba40f923e",
}


def free(path):
    stat = os.statvfs(path)
    return stat.f_bavail * stat.f_frsize


def memory_status():
    values = dict(line.split(":", 1) for line in Path("/proc/meminfo").read_text().splitlines())
    host = int(values["MemAvailable"].split()[0]) * 1024
    memberships = [line.split(":", 2) for line in Path("/proc/self/cgroup").read_text().splitlines()]
    constraints = []
    for line in Path("/proc/self/mountinfo").read_text().splitlines():
        left, right = line.split(" - ", 1)
        fields, fs = left.split(), right.split()
        version = 2 if fs[0] == "cgroup2" else 1 if fs[0] == "cgroup" and "memory" in fs[2].split(",") else None
        if version is None:
            continue
        candidates = [path for _, controllers, path in memberships
            if (version == 2 and controllers == "") or (version == 1 and "memory" in controllers.split(","))]
        if len(candidates) != 1:
            raise ValueError("ambiguous applicable memory cgroup")
        # Proc escapes spaces in paths. Do not guess an inaccessible owner.
        if "\\" in fields[3] or "\\" in fields[4]:
            raise ValueError("unsupported escaped cgroup mount")
        relative = PurePosixPath(candidates[0]).relative_to(PurePosixPath(fields[3]))
        mount = Path(fields[4])
        current = mount.joinpath(*relative.parts)
        while True:
            limit_file, usage_file = (("memory.max", "memory.current") if version == 2
                else ("memory.limit_in_bytes", "memory.usage_in_bytes"))
            limit = (current / limit_file).read_text().strip()
            usage = int((current / usage_file).read_text().strip())
            bound = None if limit == "max" else int(limit)
            remaining = None if bound is None else max(0, bound - usage)
            constraints.append({"version": version, "path": str(current), "limit": bound,
                "usage": usage, "remaining": remaining})
            if current == mount:
                break
            current = current.parent
    return {"host_available": host, "cgroups": constraints,
        "cgroup_visibility": "observed" if constraints else "not mounted; limit unknown",
        "effective_available": min([host] + [c["remaining"] for c in constraints if c["remaining"] is not None])}


def available_memory():
    status = memory_status()
    if not any(item["limit"] is not None for item in status["cgroups"]):
        raise ValueError("finite effective cgroup memory bound is not observable; native attempt is blocked")
    return status["effective_available"]


class GuardedRunner(ci.Runner):
    def __init__(self, root, env, overlay):
        super().__init__(root, env)
        self.overlay = overlay
        self.minima = {"overlay": free(overlay), "tmpfs": free(root), "memory": available_memory()}
        self.minimum = self.minima["overlay"]
        self.started = time.monotonic()

    def check_space(self):
        current = {"overlay": free(self.overlay), "tmpfs": free(self.root), "memory": available_memory()}
        for name, amount in current.items():
            self.minima[name] = min(self.minima[name], amount)
        self.minimum = self.minima["overlay"]
        for name, floor in [("overlay", ci.FLOOR), ("tmpfs", TMP_FLOOR), ("memory", MEM_FLOOR)]:
            if current[name] < floor:
                raise ValueError(f"{name} resource floor reached: {current[name]} < {floor}")
        if time.monotonic() - self.started > 3600:
            raise ValueError("one-hour bounded experiment deadline reached")


def registry_seal(source, output):
    """Content hashes, not access times. Excludes Cargo's root bookkeeping."""
    if output.exists() or output.resolve().is_relative_to(source.resolve()):
        raise ValueError("cache seal must be new and outside registry")
    rows = {}
    for path in sorted(source.rglob("*")):
        if path.is_symlink():
            raise ValueError("unexpected registry symlink")
        if path.is_file():
            rows[path.relative_to(source).as_posix()] = ci.digest(path)
    if not rows:
        raise ValueError("existing offline registry is required")
    output.write_text(json.dumps(rows, sort_keys=True) + "\n")
    print(f"Sealed {len(rows)} existing registry content files.")


def bookkeeping(home):
    result = {}
    for name in (".global-cache", ".global-cache-shm", ".global-cache-wal", ".package-cache", ".package-cache-mutate"):
        path = home / name
        if path.is_file():
            stat = path.stat()
            result[name] = {"bytes": stat.st_size, "mtime_ns": stat.st_mtime_ns}
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--official-git", type=Path, required=True)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--overlay-root", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    root, overlay, source, cli = (p.resolve() for p in (args.out, args.overlay_root, args.official_git, args.cli))
    if root.parent != Path("/tmp") or root.exists():
        raise ValueError("require a new direct /tmp child, no symlink or reused output")
    if os.stat("/tmp").st_dev == overlay.stat().st_dev:
        raise ValueError("/tmp must be a separate filesystem")
    if not any(" /tmp " in line and " - tmpfs " in line for line in Path("/proc/self/mountinfo").read_text().splitlines()):
        raise ValueError("/tmp must be the verified tmpfs mount")
    for name, expected in FROZEN.items():
        if ci.digest(ci.HERE / name) != expected:
            raise ValueError("frozen experiment source changed: " + name)
    if ci.digest(cli) != CLI_SHA:
        raise ValueError("not the verified official e8cd CLI binary")
    for key in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"):
        if os.environ.get(key):
            raise ValueError("unreviewed build override: " + key)
    home = Path(os.environ["CARGO_HOME"]).resolve(strict=True)
    registry = home / "registry"
    for name, value, floor in [("overlay", free(overlay), ci.FLOOR), ("tmpfs", free("/tmp"), TMP_FLOOR), ("memory", available_memory(), MEM_FLOOR)]:
        if value < floor:
            raise ValueError(f"{name} admission floor unavailable")
    root.mkdir()
    temp, target = root / "tmp", root / "target"
    temp.mkdir(); target.mkdir()
    env = os.environ.copy()
    env.update(CARGO_BUILD_JOBS="1", CARGO_INCREMENTAL="0", CARGO_PROFILE_DEV_DEBUG="0",
        CARGO_PROFILE_TEST_DEBUG="0", CARGO_PROFILE_TEST_OPT_LEVEL="0", CARGO_TERM_COLOR="never",
        CARGO_TARGET_DIR=str(target), TMPDIR=str(temp), TMP=str(temp), TEMP=str(temp),
        RUSTUP_TOOLCHAIN="1.95.0", RUSTUP_AUTO_INSTALL="0", CARGO_NET_OFFLINE="true", PYTHONDONTWRITEBYTECODE="1")
    runner = GuardedRunner(root, env, overlay)
    result = {"status": "failed", "horizon": ci.HORIZON, "mixins": ci.MIXINS,
        "cli_sha256": CLI_SHA, "bundle_sha256": ci.BUNDLE_SHA, "frozen_scripts": FROZEN,
        "runner_sha256": ci.digest(Path(__file__)), "steps": runner.steps, "runs": [],
        "profile": "test; opt-level=0; debug=0; incremental=0; jobs=1; offline",
        "floors": {"overlay": ci.FLOOR, "tmpfs": TMP_FLOOR, "memory": MEM_FLOOR},
        "scope": "Actual native renderer CPU with original synthetic assets; not GPU or production assets",
        "memory_before": memory_status(),
        "cache_bookkeeping_before": bookkeeping(home)}
    cache_before, cache_after = root / "registry-before.json", root / "registry-after.json"
    try:
        identity = runner.run("official-identity", ["git", "-C", str(source), "rev-parse", "HEAD"])
        if identity.read_text().strip() != ci.HORIZON:
            raise ValueError("official source checkout revision changed")
        origin = runner.run("official-origin", ["git", "-C", str(source), "remote", "get-url", "origin"])
        if origin.read_text().strip() != ci.OFFICIAL["horizon"][0]:
            raise ValueError("official source origin differs")
        runner.run("registry-before", [sys.executable, str(Path(__file__)), "--registry-seal", str(registry), str(cache_before)])
        runner.run("toolchain", ["rustc", "-Vv"])
        original, bundle, fixture, plan, stage = root / "official", root / "plutolib.hnmod", root / "fixture", root / "plan.json", root / "stage"
        runner.run("exact-official-export", [sys.executable, str(ci.HERE / "ci.py"), "--export", str(source), ci.HORIZON, str(original)])
        runner.run("package-v14", [sys.executable, str(ci.REPO / "pluto-lib/tools/package_native.py"), str(bundle)])
        if ci.digest(bundle) != ci.BUNDLE_SHA:
            raise ValueError("ordinary archive differs from frozen v14")
        runner.run("prepare", [sys.executable, str(ci.HERE / "prepare.py"), "--official", str(original), "--bundle", str(bundle), "--output", str(fixture)])
        runner.run("plan", [str(cli), "plan", "--workspace", str(original), "--mods", str(fixture / "mods"), "--out", str(plan)])
        runner.run("all-production-guards", [sys.executable, str(ci.REPO / "pluto-lib/tools/check_native_plan.py"), str(plan), str(original)])
        runner.run("fixture-guard", [sys.executable, str(ci.HERE / "check_plan.py"), str(fixture), str(plan)])
        runner.run("apply", [str(cli), "apply", "--workspace", str(original), "--mods", str(fixture / "mods"), "--out", str(stage)])
        runner.run("verify-before", [sys.executable, str(ci.HERE / "prepare.py"), "--output", str(fixture), "--verify"])
        before, after = root / "source-before.json", root / "source-after.json"
        runner.run("source-before", [sys.executable, str(ci.HERE / "ci.py"), "--seal", str(stage), str(before)])
        checker = ci.load("tmpfs_cpu_log_checker", ci.HERE / "run.py")
        for mode in ("no-provider", "registered"):
            command = ["cargo", "test", "--offline", "--locked", "--manifest-path", str(stage / "Cargo.toml"),
                "-p", "horizon-client-render", "--lib", ci.TEST, "--", "--exact", "--nocapture", "--test-threads=1"]
            log = runner.run("native-" + mode, command, cwd=stage, extra_env={
                "PLUTO_TERRAIN_FIXTURE_RESOURCES": str(fixture / "resources"), "PLUTO_TERRAIN_FIXTURE_MODE": mode,
                "PLUTO_TERRAIN_PROFILE_LABEL": result["profile"], "PLUTO_TERRAIN_TIMING_ITERS": "64"})
            result["runs"].append({"mode": mode, "timings": checker.checked_log(log.read_text(), mode)})
        runner.run("source-after", [sys.executable, str(ci.HERE / "ci.py"), "--seal", str(stage), str(after)])
        if before.read_bytes() != after.read_bytes():
            raise ValueError("applied source/Cargo.lock changed")
        runner.run("verify-after", [sys.executable, str(ci.HERE / "prepare.py"), "--output", str(fixture), "--verify"])
        runner.run("registry-after", [sys.executable, str(Path(__file__)), "--registry-seal", str(registry), str(cache_after)])
        if cache_before.read_bytes() != cache_after.read_bytes():
            raise ValueError("existing registry content changed")
        result.update(status="passed", source_seal_sha256=ci.digest(before), registry_seal_sha256=ci.digest(cache_before), plan_sha256=ci.digest(plan))
    except BaseException as error:
        result["error"] = f"{type(error).__name__}: {error}"
        raise
    finally:
        result["minimum_bytes"] = runner.minima
        result["memory_after"] = memory_status()
        result["cache_bookkeeping_after"] = bookkeeping(home)
        (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        print("PLUTO_TERRAIN_TMPFS_RESULT " + json.dumps(result), flush=True)


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "--registry-seal":
        registry_seal(Path(sys.argv[2]), Path(sys.argv[3]))
    else:
        main()
