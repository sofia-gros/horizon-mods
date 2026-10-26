"""Check state specimens over the live server connection and save/reopen.

These are vanilla wooden pickaxes carrying core-state fixtures, not playable
TConstruct tools. Runs one game at a time and never touches existing saves.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import tempfile
import time

from probe_horizon_server_mods import Game, ROOT


def states(inventory):
    result = []
    for slot in inventory["hotbar"] + inventory["main"]:
        components = slot.get("components", "")
        if "state_json:" in components:
            encoded = components.split("state_json:", 1)[1].lstrip()
            quote = encoded[0]
            if quote not in ("'", '"'):
                raise ValueError("state_json is not an SNBT string")
            chars, escaped = [], False
            for char in encoded[1:]:
                if escaped:
                    chars.append({"n": "\n", "r": "\r", "t": "\t"}.get(char, char))
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == quote:
                    break
                else:
                    chars.append(char)
            else:
                raise ValueError("unterminated state_json")
            source = "".join(chars)
            result.append(json.loads(source))
    return sorted(result, key=lambda state: (state["materials"], state["damage"]))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--game", type=Path, default=Path("A:/Project/horizon/target/fast/horizon-client.exe"))
    parser.add_argument("--ctl", type=Path, default=Path("A:/Project/horizon/target/fast/horizon-ctl.exe"))
    parser.add_argument("--resources", type=Path, default=Path(os.environ["APPDATA"]) / "horizon-launcher/versions/v0.2.66/.local/horizon-client/26.3")
    parser.add_argument("--client-mod", type=Path, default=ROOT / "Tinkers' onstruct/target/horizon-mod/tconstruct.hmod")
    parser.add_argument("--in-frame", action="store_true")
    args = parser.parse_args()
    args.server_mod = None
    for key in ("game", "ctl", "client_mod", "resources"):
        setattr(args, key, getattr(args, key).resolve())
    parent = ROOT / ".local/item-state-tests"
    parent.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix="in-frame-" if args.in_frame else "integrated-", dir=parent))
    report = {"ok": False, "checks": [], "mode": "in-frame" if args.in_frame else "integrated"}
    for key in ("game", "client_mod"):
        report[key + "_sha256"] = hashlib.sha256(getattr(args, key).read_bytes()).hexdigest()
    game = None
    print(f"RUN={run}", flush=True)

    def check(label, condition):
        if not condition:
            raise AssertionError(label)
        report["checks"].append(label)

    def snapshot(label):
        value = game.call("inventory")
        (run / (label + ".json")).write_text(json.dumps(value, indent=2), encoding="utf-8")
        return value

    expected = []
    for head, damage, broken in (("wood", 0, False), ("flint", 17, False), ("flint", 85, True)):
        expected.append({"schema": 1, "tool": "tconstruct:pickaxe", "materials": ["tconstruct:" + head, "tconstruct:wood", "tconstruct:wood"], "damage": damage, "broken": broken})
    expected.sort(key=lambda state: (state["materials"], state["damage"]))
    try:
        game = Game(args, run, "StateTester", in_frame=args.in_frame)
        game.ready()
        for head, damage in (("wood", 0), ("flint", 17), ("flint", 85)):
            game.call("chat", f"/tconstruct specimen {head} wood wood {damage}")
        game.call("command", "give @s minecraft:diamond 3")
        deadline = time.monotonic() + 8
        while True:
            inventory = snapshot("before-save")
            if states(inventory) == expected or time.monotonic() > deadline:
                break
            time.sleep(0.2)
        check("three separate core states reach the client intact", states(inventory) == expected)
        check("vanilla inventory unaffected", inventory["counts"].get("minecraft:diamond") == 3)
        game.call("chat", "/tconstruct specimen stone wood wood")
        time.sleep(0.3)
        check("unsupported material rejected", "unsupported material tconstruct:stone" in json.dumps(game.messages()))
        check("rejected request creates no item", states(snapshot("after-invalid")) == expected)
        game.call("save")
        game.stop()
        game.cleanup()
        (run / "StateTester/control.json").unlink(missing_ok=True)
        game = Game(args, run, "StateTester", in_frame=args.in_frame, resume=True)
        game.ready()
        inventory = snapshot("after-reopen")
        check("material damage and broken state survive save/reopen", states(inventory) == expected)
        check("vanilla items survive save/reopen", inventory["counts"].get("minecraft:diamond") == 3)
        game.stop()
        report["ok"] = True
    except Exception as error:
        report["error"] = f"{type(error).__name__}: {error}"
        if game and game.proc.poll() is None:
            try:
                (run / "messages.json").write_text(json.dumps(game.messages(), indent=2), encoding="utf-8")
            except Exception:
                pass
    finally:
        if game:
            game.cleanup()
        (run / "report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
        print(json.dumps({"ok": report["ok"], "report": str(run / "report.json"), "error": report.get("error")}), flush=True)
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
