"""Run the M2 probe in isolated worlds; no real player data is used."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]


def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


class Game:
    def __init__(self, args, root, label, server=None, dedicated=False, in_frame=False, mods=True, resume=False):
        self.args, self.root, self.label = args, root / label, label
        self.root.mkdir(exist_ok=resume)
        self.endpoint = self.root / "control.json"
        self.env = dict(os.environ, HORIZON_NO_AUDIO="1", HORIZON_LAUNCHER_NO_SELF_UPDATE="1",
                        HORIZON_NO_DISCORD="1", HORIZON_TELEMETRY_URL="off")
        temporary = self.root / "tmp"
        temporary.mkdir(exist_ok=resume)
        self.env.update(TEMP=str(temporary), TMP=str(temporary))
        for key in ("HORIZON_HOME", "HORIZON_LAUNCHER_HOME", "XDG_DATA_HOME", "XDG_CONFIG_HOME"):
            folder = self.root / key.lower()
            folder.mkdir(exist_ok=resume)
            self.env[key] = str(folder)
        command = [str(args.game), "--resources", str(args.resources), "--control-socket", str(self.endpoint),
                   "--test-terrain", "--render-distance", "2"]
        probe = self.root / "tconstruct_probe.hmod"
        if mods and args.server_mod and (dedicated or not server):
            # Package-relative storage must also stay inside this test run.
            shutil.copy2(args.server_mod, probe)
        if dedicated:
            self.port = free_port()
            (self.root / "eula.txt").write_text("eula=true\n", encoding="ascii")
            (self.root / "server.properties").write_text(
                f"online-mode=false\nserver-ip=127.0.0.1\nserver-port={self.port}\n"
                "gamemode=creative\nlevel-name=Probe\nview-distance=2\nsimulation-distance=2\nspawn-protection=0\n"
                "white-list=false\nenforce-whitelist=false\npause-when-empty-seconds=0\n",
                encoding="ascii")
            if mods and args.server_mod:
                command += ["--dev-mod", str(probe)]
            command += ["--dedicated", "--nogui"]
        else:
            command += ["--offscreen", "--width", "640", "--height", "360", "--name", label]
            if mods:
                command += ["--dev-mod", str(args.client_mod)]
            if server:
                command += ["--server", server]
            else:
                command += ["--world", "Probe", "--seed", "42", "--creative"]
                if mods and args.server_mod:
                    command += ["--dev-mod", str(probe)]
            if in_frame:
                command += ["--in-frame"]
        self.log = (self.root / "game.log").open("a" if resume else "w", encoding="utf-8")
        self.proc = subprocess.Popen(command, cwd=self.root, env=self.env, stdin=subprocess.PIPE,
                                     stdout=self.log, stderr=subprocess.STDOUT,
                                     creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))

    def ready(self):
        deadline = time.monotonic() + 90
        while not self.endpoint.exists():
            if self.proc.poll() is not None:
                raise RuntimeError(f"{self.label} exited {self.proc.returncode}; see game.log")
            if time.monotonic() > deadline:
                raise TimeoutError(f"{self.label}: no control endpoint")
            time.sleep(0.2)
        if not hasattr(self, "port"):
            self.call("wait-ready", "90", timeout=95)
        else:
            deadline = time.monotonic() + 90
            while True:
                try:
                    with socket.create_connection(("127.0.0.1", self.port), timeout=1):
                        break
                except OSError:
                    if self.proc.poll() is not None or time.monotonic() > deadline:
                        raise RuntimeError("dedicated server did not listen")
                    time.sleep(0.3)

    def call(self, *words, timeout=20):
        result = subprocess.run([str(self.args.ctl), "--socket", str(self.endpoint), "--timeout", str(timeout), *map(str, words)],
                                cwd=self.root, env=self.env, capture_output=True, text=True, encoding="utf-8", errors="replace",
                                timeout=timeout + 5, creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
        value = json.loads(result.stdout)
        if result.returncode or value.get("error"):
            raise RuntimeError(f"{self.label} {words[0]}: {value}")
        return value

    def messages(self):
        return self.call("messages", "100")

    def status(self):
        request = uuid.uuid4().hex
        self.call("chat", "/tprobe status " + request)
        deadline = time.monotonic() + 8
        while True:
            value = self.messages()
            # `chat` can be the server's history; styled is the window's
            # presentation, which includes local client Mod feedback.
            for line in value.get("chat", []):
                try:
                    status = json.loads(line)
                except (ValueError, TypeError):
                    continue
                if isinstance(status, dict) and status.get("request") == request:
                    return status
            if time.monotonic() > deadline:
                raise AssertionError(f"{self.label}: no current server probe reply; {value}")
            time.sleep(0.15)

    def stop(self):
        if self.proc.poll() is None:
            if hasattr(self, "port"):
                self.proc.stdin.write(b"stop\n")
                self.proc.stdin.flush()
            else:
                self.call("quit")
            self.proc.wait(timeout=45)
        if self.proc.returncode != 0:
            raise RuntimeError(f"{self.label} exited {self.proc.returncode}")

    def cleanup(self):
        if self.proc.poll() is None:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait(timeout=10)
        self.log.close()


def exercise(client, report, paused=False, actions=True, dimensions=True):
    def check(name, condition):
        if not condition:
            raise AssertionError(name)
        report.append(name)
    first = client.status()
    check("single server instance starts once", first["boots"] == 1)
    check("one join per online player, online actor", first["joins"] == len(first["online"]) and first["actor"] in first["online"])
    time.sleep(0.6)
    second = client.status()
    check("server tick advances exactly once per clock tick", second["ticks"] > first["ticks"]
          and second["ticks"] - first["ticks"] == second["last_tick"] - first["last_tick"]
          and second["duplicate_ticks"] == 0)
    client.call("chat", "/tconstruct stats wood wood wood")
    time.sleep(0.2)
    check("legacy client diagnostic", "durability=60, mining=2.000, damage=1.000, speed=1.200" in json.dumps(client.messages()))
    client.call("chat", "M2 accepted chat")
    time.sleep(0.2)
    check("chat callback", client.status()["chats"] == second["chats"] + 1)
    client.call("chat", "/tprobe tell")
    time.sleep(0.2)
    check("targeted output", "M2 targeted reply" in json.dumps(client.messages()))
    client.call("chat", "/tprobe error")
    time.sleep(0.2)
    check("command error", "M2 expected diagnostic error" in json.dumps(client.messages()))
    if actions:
        client.call("command", "tp @s 0.5 72 0.5")
        client.call("command", "setblock 0 71 2 minecraft:stone")
        before = client.status()
        client.call("break", "0", "71", "2", "--timeout", "15", timeout=20)
        time.sleep(0.2)
        after = client.status()
        check("one player break event", after["broken"] == before["broken"] + 1)
        check("break dimension and state", "minecraft:overworld:0,71,2:minecraft:stone" in after["last"])
        client.call("command", "tp @s 0.5 69 0.5")
        client.call("command", "setblock 0 70 2 minecraft:air")
        client.call("command", "setblock 0 69 2 minecraft:stone")
        client.call("command", "item replace entity @s hotbar.0 with minecraft:cobblestone 8")
        time.sleep(0.3)
        client.call("select", "minecraft:cobblestone")
        before = client.status()
        client.call("place", "0", "70", "2", "--item", "minecraft:cobblestone", timeout=25)
        time.sleep(0.3)
        after = client.status()
        check("one player placement event", after["placed"] == before["placed"] + 1)
        check("placement state", "minecraft:overworld:0,70,2:minecraft:cobblestone" in after["last"])
    if actions and dimensions:
        client.call("command", "execute in minecraft:the_nether run tp @s 0.5 72 0.5")
        # This checks the server's player snapshot, not completion of all
        # renderer lighting/meshing work in the in-frame fallback.
        deadline = time.monotonic() + 15
        while True:
            after = client.status()
            if after["dimension"] == "minecraft:the_nether" or time.monotonic() > deadline:
                break
            time.sleep(0.2)
        check("player dimension updates without a second join", after["dimension"] == "minecraft:the_nether" and after["joins"] == first["joins"])
    if paused:
        client.call("pause")
        time.sleep(0.3)
        before = client.status()
        time.sleep(0.7)
        after = client.status()
        check("pause stops server callbacks", before["ticks"] == after["ticks"])
        client.call("resume")
        time.sleep(0.3)
        check("resume restarts callbacks", client.status()["ticks"] > after["ticks"])
    return client.status()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--game", type=Path, default=Path("A:/Project/horizon/target/fast/horizon-client.exe"))
    parser.add_argument("--ctl", type=Path, default=Path("A:/Project/horizon/target/fast/horizon-ctl.exe"))
    parser.add_argument("--resources", type=Path, default=Path(os.environ["APPDATA"]) / "horizon-launcher/versions/v0.2.66/.local/horizon-client/26.3")
    parser.add_argument("--server-mod", type=Path, default=ROOT / "Tinkers' onstruct/probes/server-lifecycle/target/horizon-mod/tconstruct_probe.hmod")
    parser.add_argument("--client-mod", type=Path, default=ROOT / "Tinkers' onstruct/target/horizon-mod/tconstruct.hmod")
    parser.add_argument("--mode", choices=["integrated", "in-frame", "dedicated", "lan-integrated", "lan-in-frame"], default="integrated")
    args = parser.parse_args()
    for name in ("game", "ctl", "server_mod", "client_mod"):
        path = getattr(args, name).resolve()
        if not path.is_file():
            parser.error(f"Missing {name}: {path}")
        setattr(args, name, path)
    parent = ROOT / ".local/server-mods-tests"
    parent.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix=args.mode + "-", dir=parent))
    report = {"mode": args.mode, "ok": False, "checks": [], "game_sha256": hashlib.sha256(args.game.read_bytes()).hexdigest()}
    games = []
    print(f"RUN={root}", flush=True)
    try:
        host = Game(args, root, "Host", dedicated=args.mode == "dedicated", in_frame=args.mode.endswith("in-frame"))
        games.append(host)
        host.ready()
        if args.mode.startswith("lan-"):
            reply = host.call("lan", "open", "0", "commands")
            (root / "lan.json").write_text(json.dumps(reply), encoding="utf-8")
            port = reply.get("port")
            if port is None:
                port = host.call("lan", "status").get("port")
            if port is None:
                raise ValueError("LAN response did not report its port")
        elif args.mode == "dedicated":
            port = host.port
        else:
            port = None
        if port:
            client = Game(args, root, "Guest", server=f"127.0.0.1:{port}")
            games.append(client)
            client.ready()
            # Remote clients have no operator access; lifecycle/output checks
            # must work without granting elevated command permissions.
            report["last"] = exercise(client, report["checks"], actions=False)
            if args.mode.startswith("lan-"):
                if "M2 targeted reply" in json.dumps(host.messages()):
                    raise AssertionError("guest's targeted output leaked to host")
                report["checks"].append("targeted output stays with guest")
            client.call("chat", "/tprobe broadcast")
            time.sleep(0.3)
            if args.mode.startswith("lan-"):
                if "M2 broadcast reply" not in json.dumps(host.messages()):
                    raise AssertionError("host did not receive guest broadcast")
                before = host.status()
                if args.mode == "lan-integrated":
                    host.call("command", "execute in minecraft:the_nether run tp @s 0.5 72 0.5")
                    host.call("wait-ready", "60", timeout=65)
                    nether = host.status()
                    overworld = client.status()
                    if nether["dimension"] != "minecraft:the_nether" or overworld["dimension"] != "minecraft:overworld":
                        raise AssertionError("players did not remain in their separate dimensions")
                    time.sleep(0.4)
                    end = client.status()
                    if end["duplicate_ticks"] or end["ticks"] - overworld["ticks"] != end["last_tick"] - overworld["last_tick"]:
                        raise AssertionError("multiple active dimensions duplicated or skipped callbacks")
                    report["checks"].append("two active dimensions share one server callback per tick")
                client.stop()
                time.sleep(0.4)
                after = host.status()
                if after["leaves"] != before["leaves"] + 1:
                    raise AssertionError("guest leave callback")
                report["checks"].append("guest broadcast and single leave callback")
        else:
            report["last"] = exercise(host, report["checks"], paused=True, dimensions=args.mode != "in-frame")
            if args.mode == "in-frame":
                report["not_tested"] = ["in-frame Nether travel: player did not recover in the test-terrain fixture"]
        for game in reversed(games):
            game.stop()
        report["ok"] = True
    except Exception as error:
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        for game in reversed(games):
            game.cleanup()
        (root / "report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
        print(json.dumps({"ok": report["ok"], "report": str(root / "report.json"), "error": report.get("error")}), flush=True)
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
