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
import uuid

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
    parser.add_argument("--server-mod", type=Path, help="Optional tconstruct_tools inspector package")
    parser.add_argument("--state-edits", action="store_true", help="Test Creative-only core state damage/repair commands")
    args = parser.parse_args()
    if args.state_edits and not args.server_mod:
        parser.error("--state-edits requires --server-mod")
    for key in ("game", "ctl", "client_mod", "resources"):
        setattr(args, key, getattr(args, key).resolve())
    if args.server_mod:
        args.server_mod = args.server_mod.resolve()
    parent = ROOT / ".local/item-state-tests"
    parent.mkdir(parents=True, exist_ok=True)
    prefix = ("server-" if args.server_mod else "") + ("in-frame-" if args.in_frame else "integrated-")
    run = Path(tempfile.mkdtemp(prefix=prefix, dir=parent))
    report = {"ok": False, "checks": [], "mode": "in-frame" if args.in_frame else "integrated", "state_edits": args.state_edits}
    for key in ("game", "client_mod"):
        report[key + "_sha256"] = hashlib.sha256(getattr(args, key).read_bytes()).hexdigest()
    if args.server_mod:
        report["server_mod_sha256"] = hashlib.sha256(args.server_mod.read_bytes()).hexdigest()
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

    def inspect_server(phase):
        if not args.server_mod:
            return
        for slot, (head, damage, broken) in enumerate(fixtures, 1):
            game.call("select", slot)
            game.call("wait", "3")
            request = uuid.uuid4().hex
            game.call("chat", "/tconstruct_tool inspect " + request)
            deadline = time.monotonic() + 8
            value = None
            while value is None:
                messages = game.messages()
                for line in messages.get("chat", []):
                    try:
                        candidate = json.loads(line)
                    except (ValueError, TypeError):
                        continue
                    if isinstance(candidate, dict) and candidate.get("request") == request:
                        value = candidate
                        break
                if value is None:
                    if time.monotonic() > deadline:
                        raise AssertionError("no current server inspector reply: " + json.dumps(messages))
                    time.sleep(0.15)
            (run / f"{phase}-server-slot-{slot}.json").write_text(json.dumps(value, indent=2), encoding="utf-8")
            expected_report = dict(request=request, parts=[head, "wood", "wood"], damage=damage, broken=broken,
                                   durability=60 if head == "wood" else 85, mining=2.0 if head == "wood" else 3.5,
                                   attack=1.0 if head == "wood" else 2.25, speed=1.2, vanilla_damage=0, hooks=False)
            check(f"{phase}: server reads slot {slot} state and base stats", value == expected_report)

    fixtures = [["wood", 0, False], ["flint", 17, False], ["flint", 85, True]]
    def expected_states():
        result = [dict(schema=1, tool="tconstruct:pickaxe", materials=["tconstruct:" + head, "tconstruct:wood", "tconstruct:wood"], damage=damage, broken=broken) for head, damage, broken in fixtures]
        return sorted(result, key=lambda state: (state["materials"], state["damage"]))
    expected = expected_states()
    try:
        game = Game(args, run, "StateTester", in_frame=args.in_frame)
        game.ready()
        # In-frame creative starts with a stocked hotbar. This world was
        # freshly created by this test; give the fixtures deterministic slots.
        if game.call("inventory")["counts"]:
            game.call("command", "clear @s")
        for head, damage in (("wood", 0), ("flint", 17), ("flint", 85)):
            game.call("chat", f"/tconstruct specimen {head} wood wood {damage}")
        deadline = time.monotonic() + 8
        while True:
            inventory = snapshot("before-save")
            if states(inventory) == expected or time.monotonic() > deadline:
                break
            time.sleep(0.2)
        check("three separate core states reach the client intact", states(inventory) == expected)
        game.call("command", "give @s minecraft:diamond 3")
        inventory = snapshot("with-vanilla")
        check("vanilla inventory unaffected", inventory["counts"].get("minecraft:diamond") == 3)
        inspect_server("before-save")
        if args.state_edits:
            for operation, points, damage, broken in (("test-damage", 4294967295, 85, True), ("test-repair", 20, 65, False), ("test-repair", 4294967295, 0, False), ("test-damage", 20, 20, False)):
                game.call("select", "2")
                game.call("wait", "3")
                game.call("chat", f"/tconstruct_tool {operation} {points}")
                fixtures[1] = ["flint", damage, broken]
                expected = expected_states()
                phase = f"{operation}-{points}-{damage}"
                inspect_server(phase)
                check(phase + ": changed only the selected core state", states(snapshot(phase)) == expected)
            game.call("command", "gamemode survival @s")
            game.call("wait", "3")
            game.call("chat", "/tconstruct_tool test-repair 5")
            deadline = time.monotonic() + 5
            while "test edits require creative mode" not in json.dumps(game.messages()):
                if time.monotonic() > deadline:
                    raise AssertionError("survival test edit was not rejected")
                time.sleep(0.15)
            check("survival cannot invoke Creative test repairs", states(snapshot("survival-rejected")) == expected)
            game.call("command", "gamemode creative @s")
            game.call("wait", "3")
        if args.server_mod:
            game.call("select", "4")
            game.call("wait", "3")
            game.call("chat", "/tconstruct_tool inspect vanilla")
            deadline = time.monotonic() + 5
            while "no TConstruct state in main hand" not in json.dumps(game.messages()):
                if time.monotonic() > deadline:
                    raise AssertionError("vanilla hand was not rejected")
                time.sleep(0.15)
            check("server rejects a vanilla item without tool state", True)
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
        inspect_server("after-reopen")
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
