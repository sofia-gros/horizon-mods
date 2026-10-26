"""Exercise installed Horizon in fresh directories; never reuse player saves.

Standard library only. Run with --help for overrides and --in-frame for the
alternate single-player path. Reports and worlds stay in the output directory.
"""

import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    base = Path(os.environ.get("APPDATA", "")) / "horizon-launcher/versions/v0.2.66"
    parser.add_argument("--client", type=Path, default=base / "horizon-client.exe")
    parser.add_argument("--ctl", type=Path, default=base / "horizon-ctl.exe")
    parser.add_argument("--resources", type=Path, default=base / ".local/horizon-client/26.3")
    parser.add_argument("--output-parent", type=Path, default=Path(__file__).resolve().parents[1] / ".local/automation")
    parser.add_argument("--in-frame", action="store_true")
    args = parser.parse_args()
    for path in (args.client, args.ctl, args.resources / "horizon-resources.json"):
        if not path.is_file():
            parser.error(f"Missing required file: {path}")
    args.output_parent.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix="smoke-", dir=args.output_parent.resolve()))
    env = dict(os.environ)
    for key, child in (("HORIZON_HOME", "game"), ("HORIZON_LAUNCHER_HOME", "launcher"),
                       ("XDG_DATA_HOME", "xdg-data"), ("XDG_CONFIG_HOME", "xdg-config")):
        folder = run / child
        folder.mkdir()
        env[key] = str(folder)
    env.update(HORIZON_LAUNCHER_NO_SELF_UPDATE="1", HORIZON_NO_AUDIO="1",
               HORIZON_NO_DISCORD="1", HORIZON_TELEMETRY_URL="off")
    endpoint = run / "control.json"
    report = {
        "schema": 1, "started_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "mode": "in-frame" if args.in_frame else "integrated", "run": str(run),
        "client": str(args.client.resolve()), "ctl": str(args.ctl.resolve()),
        "resources": str(args.resources.resolve()), "steps": [], "ok": False,
    }
    for name, path in (("client", args.client), ("ctl", args.ctl)):
        digest = hashlib.sha256()
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(chunk)
        report[f"{name}_sha256"] = digest.hexdigest()
    proc = None
    log_file = None

    def ctl(label, *words, timeout=15):
        result = subprocess.run([str(args.ctl.resolve()), "--socket", str(endpoint),
                                 "--timeout", str(timeout), *words], env=env, cwd=run,
                                capture_output=True, text=True, encoding="utf-8", errors="replace",
                                timeout=timeout + 5, creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
        try:
            value = json.loads(result.stdout)
        except json.JSONDecodeError:
            value = {"stdout": result.stdout}
        (run / f"{label}.json").write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding="utf-8")
        if result.returncode != 0 or (isinstance(value, dict) and value.get("error")):
            raise RuntimeError(f"{label}: exit {result.returncode}: {result.stderr or result.stdout}")
        report["steps"].append({"step": label, "ok": True})
        return value

    def start(label):
        nonlocal proc, log_file
        log_file = (run / f"{label}.log").open("w", encoding="utf-8")
        command = [str(args.client.resolve()), "--resources", str(args.resources.resolve()),
                   "--control-socket", str(endpoint), "--offscreen", "--world", "AutomationSmoke",
                   "--seed", "42", "--creative", "--test-terrain", "--render-distance", "2",
                   "--width", "640", "--height", "360"]
        if args.in_frame:
            command.append("--in-frame")
        proc = subprocess.Popen(command, env=env, cwd=run, stdin=subprocess.DEVNULL,
                                stdout=log_file, stderr=subprocess.STDOUT,
                                creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
        deadline = time.monotonic() + 45
        while not endpoint.exists():
            if proc.poll() is not None:
                raise RuntimeError(f"{label}: game exited {proc.returncode}; see {label}.log")
            if time.monotonic() > deadline:
                raise TimeoutError(f"{label}: control endpoint did not appear")
            time.sleep(0.2)
        ctl(f"{label}-ready", "wait-ready", "45", timeout=50)

    def stop(label):
        nonlocal proc, log_file
        ctl(label, "quit", timeout=20)
        proc.wait(timeout=30)
        report["steps"].append({"step": f"{label}-exit", "ok": proc.returncode == 0,
                                "exit_code": proc.returncode})
        if proc.returncode != 0:
            raise RuntimeError(f"{label}: exit {proc.returncode}")
        proc = None
        log_file.close()
        log_file = None

    try:
        print(f"RUN={run}", flush=True)
        start("first")
        ctl("state", "state")
        ctl("set-block", "command", "setblock 0 80 0 minecraft:gold_block")
        block = ctl("block-before-save", "block", "0", "80", "0")
        if block.get("block") != "minecraft:gold_block" or not block.get("loaded"):
            raise AssertionError("Placed block was not observed")
        ctl("give-item", "command", "give @s minecraft:diamond 3")
        inventory = ctl("inventory", "inventory")
        if inventory.get("counts", {}).get("minecraft:diamond") != 3:
            raise AssertionError("Given item was not observed")
        ctl("screen", "screen")
        picture = run / "smoke.png"
        ctl("screenshot", "screenshot", str(picture), timeout=20)
        if not picture.is_file() or picture.read_bytes()[:8] != b"\x89PNG\r\n\x1a\n":
            raise AssertionError("Screenshot did not produce a PNG")
        ctl("open-inventory", "open-inventory")
        ctl("menu", "menu")
        ctl("save", "save", timeout=20)
        stop("first-quit")
        # No endpoint file is read or included in the public report (it holds a token).
        # A clean shutdown removes it; remove only this run's stale endpoint if needed.
        endpoint.unlink(missing_ok=True)
        start("reopen")
        block = ctl("block-after-reopen", "block", "0", "80", "0")
        if block.get("block") != "minecraft:gold_block" or not block.get("loaded"):
            raise AssertionError("Block did not survive save and reopen")
        inventory = ctl("inventory-after-reopen", "inventory")
        if inventory.get("counts", {}).get("minecraft:diamond") != 3:
            raise AssertionError("Item did not survive save and reopen")
        stop("reopen-quit")
        report["ok"] = True
    except Exception as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        if proc is not None and proc.poll() is None:
            # Only the child created above is stopped, never an existing game.
            proc.terminate()
            try:
                proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=10)
        if log_file is not None:
            log_file.close()
        report["finished_utc"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
        (run / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
        print(json.dumps({"ok": report["ok"], "report": str(run / "report.json"),
                          "error": report.get("error")}, ensure_ascii=False), flush=True)
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
