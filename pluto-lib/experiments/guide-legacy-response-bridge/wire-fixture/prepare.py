"""Source-only response fixture preparer; the outer runner owns all Git/Cargo.

No subprocess, metadata, compiler, host writes or untrusted resource loading.
"""
import hashlib
import importlib.util
import json
from pathlib import Path, PurePosixPath
import stat
import tomllib

HERE = Path(__file__).resolve().parent
HORIZON = "41f822b659789780f58d32c63ddfb6887ea1b8bf"
PACKAGE = "pluto-guide-reply-tests"
LOCAL = {(PACKAGE, "0.0.0", None, None), ("pluto-inventory-api", "0.1.0", None, None)}
NATIVE = "horizon-client-core"


def require(value, message):
    if not value: raise ValueError(message)


def sha(raw): return hashlib.sha256(raw).hexdigest()
def blob(raw): return hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()


def safe_name(name):
    path = PurePosixPath(name)
    require(isinstance(name, str) and name and not path.is_absolute() and ".." not in path.parts and "\\" not in name and str(path) == name, "invalid relative source")


def read(root, name):
    safe_name(name); path = root / name
    require(not path.is_symlink() and path.resolve().is_relative_to(root.resolve()), "escaping/symlink input")
    info = path.lstat(); require(stat.S_ISREG(info.st_mode), "nonregular source")
    return path.read_bytes()


def files(root):
    require(root.is_dir() and not root.is_symlink(), "invalid source root")
    result = {}
    for path in root.rglob("*"):
        require(not path.is_symlink(), "symlink in source set")
        info = path.lstat()
        if stat.S_ISDIR(info.st_mode): continue
        require(stat.S_ISREG(info.st_mode), "nonregular source set member")
        result[str(path.relative_to(root))] = blob(path.read_bytes())
    return result


def config():
    value = json.loads((HERE / "prepare-inputs.json").read_text())
    require(value["official_revision"] == HORIZON and value["fixture_name"] == PACKAGE, "wrong fixture identity")
    require(set(value["official_git_blobs"]) == {"Cargo.toml", "Cargo.lock", "crates/horizon-client-core/Cargo.toml", "crates/horizon-client-core/src/text.rs"}, "official input set drift")
    for path, digest in value["fixture_source_sha256"].items():
        require(sha(read(HERE, path)) == digest, "fixture input changed: " + path)
    for path, digest in value["bridge_source_sha256"].items():
        require(sha(read(HERE.parent, path)) == digest, "bridge input changed: " + path)
    return value


def check_official(host, value):
    for path, digest in value["official_git_blobs"].items():
        require(blob(read(host, path)) == digest, "official input changed: " + path)


def manifest(host):
    patches = tomllib.loads(read(host, "Cargo.toml").decode())["patch"]["crates-io"]
    require(set(patches) == {"wgpu-hal", "naga", "libmimalloc-sys"}, "official patch set changed")
    value = '[package]\nname="' + PACKAGE + '"\nversion="0.0.0"\nedition="2024"\n[workspace]\nexclude=["inventory-api"]\ndefault-members=["."]\n[lib]\npath="lib.rs"\n[dependencies]\npluto-inventory-api={path="inventory-api"}\nserde_json="=1.0.151"\nlog="=0.4.34"\n'
    value += NATIVE + '={path=' + json.dumps(str(host / "crates" / NATIVE)) + '}\n[patch.crates-io]\n'
    for name, entry in patches.items():
        require(set(entry) == {"path"}, "unsupported official patch")
        path = (host / entry["path"]).resolve()
        require(path.is_relative_to(host.resolve()), "escaping official patch")
        value += json.dumps(name) + '={path=' + json.dumps(str(path)) + '}\n'
    return value


def rendered(legacy, value):
    spec = importlib.util.spec_from_file_location("reply_wire_source", HERE / "render_wire.py")
    renderer = importlib.util.module_from_spec(spec); spec.loader.exec_module(renderer)
    sources = {name: read(legacy, path) for name, path in value["legacy_paths"].items()}
    sources.update(bridge=read(HERE.parent, "native-bridge.rs"), consumer=read(HERE.parent, "consumer.rs"), tests=read(HERE, "regression_tests.rs"))
    return renderer.compose(sources)


def prepare(host, api, legacy, output):
    value = config(); host = host.resolve(); api = api.resolve(); legacy = legacy.resolve(); output = output.absolute()
    check_official(host, value)
    require(files(api) == value["api_git_blobs"], "complete actual API source changed")
    require(not output.exists() and not output.is_symlink(), "output exists")
    for root in [host, api, legacy, HERE.resolve()]:
        require(not output.resolve().is_relative_to(root) and not root.is_relative_to(output.resolve()), "overlapping input/output")
    contents, source_proof = rendered(legacy, value)
    contents.update({"inventory-api/" + name: read(api, name) for name in value["api_git_blobs"]})
    contents["Cargo.toml"] = manifest(host).encode()
    output.mkdir()
    for name, raw in contents.items():
        safe_name(name); target = output / name; target.parent.mkdir(parents=True, exist_ok=True)
        with target.open("xb") as stream: stream.write(raw)
    with (output / "Cargo.lock").open("xb") as stream: stream.write(read(host, "Cargo.lock"))
    evidence = {"official_root": str(host), "official_revision": HORIZON, "api_root": str(api), "legacy_root": str(legacy),
                "seed_lock_sha256": sha(read(host, "Cargo.lock")), "sources_sha256": {p: sha(b) for p, b in contents.items()},
                "extraction": source_proof, "scope": "Actual Text/legacy reply value differential only; no Game, callback, channel, lifecycle or transport."}
    with (output / "fixture-sources.json").open("x") as stream: stream.write(json.dumps(evidence, indent=2) + "\n")
    return evidence


def identity(package): return tuple(package.get(key) for key in ["name", "version", "source", "checksum"])


def verify_workspace(output, metadata):
    own = [p["id"] for p in metadata["packages"] if p["name"] == PACKAGE]
    require(len(own) == 1 and metadata["workspace_members"] == own and metadata["workspace_default_members"] == own, "wrong root/member/default identities")
    require(Path(metadata["workspace_root"]).resolve() == output.resolve(), "wrong workspace root")


def verify(output, metadata):
    value = config(); evidence = json.loads(read(output, "fixture-sources.json"))
    host = Path(evidence["official_root"]); api = Path(evidence["api_root"]); legacy = Path(evidence["legacy_root"])
    require(evidence["official_revision"] == HORIZON, "wrong target")
    check_official(host, value)
    require(evidence["seed_lock_sha256"] == sha(read(host, "Cargo.lock")), "seed lock receipt drift")
    require(files(api) == value["api_git_blobs"] == files(output / "inventory-api"), "actual/copied API drift")
    contents, source_proof = rendered(legacy, value)
    require(evidence["extraction"] == source_proof, "extraction receipt drift")
    contents.update({"inventory-api/" + name: read(api, name) for name in value["api_git_blobs"]})
    contents["Cargo.toml"] = manifest(host).encode()
    require(evidence["sources_sha256"] == {p: sha(b) for p, b in contents.items()}, "source receipt drift")
    require(set(files(output)) == set(contents) | {"Cargo.lock", "fixture-sources.json"}, "fixture file set drift")
    for name, raw in contents.items(): require(read(output, name) == raw, "fixture source drift: " + name)
    allowed = {identity(p) for p in tomllib.loads(read(host, "Cargo.lock").decode())["package"]}
    packages = tomllib.loads(read(output, "Cargo.lock").decode())["package"]; lock = {identity(p) for p in packages}
    require(len(lock) == len(packages) and not lock - allowed - LOCAL and LOCAL <= lock, "lock identity drift")
    require(metadata["version"] == 1, "metadata schema drift")
    local = {PACKAGE: output / "Cargo.toml", "pluto-inventory-api": output / "inventory-api/Cargo.toml"}
    for name in local:
        require(sum(p["name"] == name for p in metadata["packages"]) == 1,
                "missing/duplicate local package: " + name)
    found = set(); ids = []
    for p in metadata["packages"]:
        ids.append(p["id"]); path = Path(p["manifest_path"]).resolve()
        require((p["name"], p["version"], p["source"]) in {entry[:3] for entry in lock}, "metadata outside exact lock")
        if p["name"] in local:
            require(p["source"] is None and path == local[p["name"]].resolve(), "wrong local owner")
            found.add(p["name"])
        elif p["source"] is None: require(path.is_relative_to(host.resolve()), "unknown local dependency")
    require(found == set(local) and len(ids) == len(set(ids)), "missing/duplicate package")
    native = [p for p in metadata["packages"] if p["name"] == NATIVE]
    require(len(native) == 1 and native[0]["source"] is None and Path(native[0]["manifest_path"]).resolve() == host / "crates" / NATIVE / "Cargo.toml", "wrong actual native Text owner")
    verify_workspace(output, metadata)
    return evidence
