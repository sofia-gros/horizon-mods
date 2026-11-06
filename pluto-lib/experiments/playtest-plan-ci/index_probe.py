"""Pure preparation/admission only. This module never invokes Cargo or a process."""
import hashlib
import json
from pathlib import Path
import re
import tomllib

OFFICIAL = "5a64af938ec86b00d53d16a408aabc0038fb3e57"
LOCK_SHA = "35846f4a621cf8e1e54f6f0f7e13ac848db590d4ec401620501c107954eae6ca"
CORE_MANIFEST_SHA = "f34267351c8490596c1dfd3529427a3ce4c65fe8427227ceaeeaeabbccb8e044"
MAIN_SHA = "b5d7710643e336eca922fb4dcfb70fec83eaf501cb3c5dc31f8948937e4b375d"
NAME = "pluto-packet-index-observer"
VERSION = "0.0.0"
CORE = "horizon-mixins-core"
CORE_VERSION = "0.6.0"
TARGETS = [
    ("horizon_client::game::native_hooks::Game<'_>::native_client_packet", "crates/horizon-client/src/game/native_hooks.rs", [40, 349], "6439828d2b9197605942c9bbfff9c2a32b93c95d934c8554fb52cc72ef85827f"),
    ("horizon_client::game::remote::Game<'_>::handle_packet", "crates/horizon-client/src/game/remote.rs", [44, 68], "9f16b281b32524d06842ac401582ce22a2c022e4b8dc45a43bfc82bd98eb9724"),
    ("horizon_client::game::remote::Game<'_>::poll_remote", "crates/horizon-client/src/game/remote.rs", None, None),
]


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def prepare(root, mixins, main_source):
    """Create a fresh external one-member package; never change an official tree."""
    root = Path(root)
    mixins = Path(mixins).resolve(strict=True)
    parent = root.parent.resolve(strict=True)
    root = parent / root.name
    if root.exists() or root.is_symlink() or root.is_relative_to(mixins):
        raise ValueError("Harness path must be fresh and outside official Mixins")
    core = mixins / "crates/horizon-mixins-core"
    if core.resolve(strict=True) != core:
        raise ValueError("Official core path is redirected")
    seed = (mixins / "Cargo.lock").read_bytes()
    main = Path(main_source).read_bytes()
    if sha(seed) != LOCK_SHA or sha(main) != MAIN_SHA:
        raise ValueError("Official lock or harness source differs")
    if sha((core / "Cargo.toml").read_bytes()) != CORE_MANIFEST_SHA:
        raise ValueError("Official core manifest differs")
    manifest = ('[package]\nname = "' + NAME + '"\nversion = "' + VERSION +
        '"\nedition = "2024"\n\n[workspace]\nmembers = ["."]\ndefault-members = ["."]\nresolver = "2"\n\n[dependencies]\n' +
        CORE + ' = { path = ' + json.dumps(str(core)) + ' }\nserde_json = "=1.0.154"\n')
    root.mkdir(mode=0o700, exist_ok=False)
    (root / "src").mkdir()
    (root / "Cargo.toml").write_bytes(manifest.encode())
    (root / "Cargo.lock").write_bytes(seed)
    (root / "src/main.rs").write_bytes(main)
    return {"manifest": str(root / "Cargo.toml"), "manifest_sha256": sha(manifest.encode()),
            "main_sha256": sha(main), "seed_lock_sha256": sha(seed), "core_manifest": str(core / "Cargo.toml")}


def lock_packages(raw):
    parsed = tomllib.loads(raw.decode("utf-8"))
    if set(parsed) != {"version", "package"} or parsed["version"] != 4:
        raise ValueError("Unexpected lock shape/version")
    entries = parsed["package"]
    if not isinstance(entries, list) or not entries:
        raise ValueError("Lock has no packages")
    result = {}
    for entry in entries:
        if not isinstance(entry, dict) or not isinstance(entry.get("name"), str) or not isinstance(entry.get("version"), str):
            raise ValueError("Invalid lock package identity")
        key = (entry["name"], entry["version"], entry.get("source"))
        if key in result:
            raise ValueError("Duplicate lock package identity")
        result[key] = entry
    return result


def verify_lock(adapted, seed):
    """Allow an exact official subset plus one new local harness, no new versions.

    Retained official entries, including dependency edges/checksums, are exact.
    Unused CLI/transform and registry entries may be pruned, never rewritten.
    """
    if sha(seed) != LOCK_SHA:
        raise ValueError("Wrong official lock seed")
    old = lock_packages(seed); new = lock_packages(adapted)
    harness = (NAME, VERSION, None)
    expected = {"name": NAME, "version": VERSION, "dependencies": [CORE, "serde_json"]}
    if new.get(harness) != expected:
        raise ValueError("New local harness lock identity differs")
    if (CORE, CORE_VERSION, None) not in new:
        raise ValueError("Missing actual official core")
    serde = [key for key in old if key[:2] == ("serde_json", "1.0.154")]
    if len(serde) != 1 or serde[0] not in new:
        raise ValueError("Missing exact official serde_json")
    for key, entry in new.items():
        if key == harness: continue
        if key not in old or entry != old[key]:
            raise ValueError("Official package identity, checksum or dependency edges differ")
        if key[2] is None and key != (CORE, CORE_VERSION, None):
            raise ValueError("Unexpected local lock package")
    return {"lock_sha256": sha(adapted), "packages": len(new)}


def verify_metadata(metadata, root, mixins, adapted, seed):
    """Require full locked metadata. The controller parses all stdout separately."""
    lock_facts = verify_lock(adapted, seed)
    root = Path(root).resolve(strict=True)
    core = Path(mixins).resolve(strict=True) / "crates/horizon-mixins-core"
    if type(metadata.get("version")) is not int or metadata["version"] != 1:
        raise ValueError("Wrong metadata version")
    if metadata.get("workspace_root") != str(root):
        raise ValueError("Wrong external workspace root")
    packages = metadata.get("packages")
    if not isinstance(packages, list) or not packages:
        raise ValueError("Missing full package metadata")
    ids = [p.get("id") for p in packages]
    if any(not isinstance(i, str) or not i for i in ids) or len(set(ids)) != len(ids):
        raise ValueError("Duplicate or invalid metadata ID")
    records = lock_packages(adapted)
    keys = [(p.get("name"), p.get("version"), p.get("source")) for p in packages]
    if len(set(keys)) != len(keys) or set(keys) != set(records):
        raise ValueError("Metadata/lock package identities differ")
    expected = {NAME: (VERSION, root / "Cargo.toml", "bin", root / "src/main.rs"),
                CORE: (CORE_VERSION, core / "Cargo.toml", "lib", core / "src/lib.rs")}
    local = [p for p in packages if p.get("source") is None]
    if len(local) != 2 or {p["name"] for p in local} != set(expected):
        raise ValueError("Unexpected local metadata packages")
    for p in local:
        version, manifest, kind, source = expected[p["name"]]
        if p["version"] != version or p.get("manifest_path") != str(manifest):
            raise ValueError("Local metadata source/version redirected")
        targets = [t for t in p.get("targets", []) if t.get("kind") == [kind]]
        name = NAME if p["name"] == NAME else CORE.replace("-", "_")
        if (len(targets) != 1 or targets[0].get("src_path") != str(source)
                or targets[0].get("name") != name or targets[0].get("crate_types") != [kind]):
            raise ValueError("Local executable/library target source differs")
        if p["name"] == NAME and len(p.get("targets", [])) != 1:
            raise ValueError("Unexpected harness build/test target")
    harness = next(p["id"] for p in local if p["name"] == NAME)
    core_id = next(p["id"] for p in local if p["name"] == CORE)
    if metadata.get("workspace_members") != [harness] or metadata.get("workspace_default_members") != [harness]:
        raise ValueError("External workspace member/default set differs")
    resolve = metadata.get("resolve")
    if not isinstance(resolve, dict) or resolve.get("root") != harness:
        raise ValueError("Missing full dependency resolution")
    nodes = resolve.get("nodes", [])
    node_ids = [n.get("id") for n in nodes]
    if len(set(node_ids)) != len(node_ids) or set(node_ids) != set(ids):
        raise ValueError("Full dependency nodes differ")
    for node in nodes:
        if not isinstance(node.get("features"), list) or any(not isinstance(f, str) for f in node["features"]):
            raise ValueError("Invalid dependency features")
        if any(d not in ids for d in node.get("dependencies", [])):
            raise ValueError("Unknown resolved dependency")
        if any(d.get("pkg") not in ids for d in node.get("deps", [])):
            raise ValueError("Unknown resolved dependency edge")
    harness_node = next(n for n in nodes if n["id"] == harness)
    serde_id = next(p["id"] for p in packages if p["name"] == "serde_json" and p["version"] == "1.0.154")
    if set(harness_node["dependencies"]) != {core_id, serde_id}:
        raise ValueError("Harness dependency set differs")
    return {**lock_facts, "harness_id": harness, "core_id": core_id,
            "core_features": next(n["features"] for n in nodes if n["id"] == core_id)}


def validate_result(raw):
    """Admit one compact whole JSON result; no line skipping or inferred values."""
    if not isinstance(raw, bytes) or not 1 <= len(raw) <= 16384 or not raw.endswith(b"\n") or b"\n" in raw[:-1]:
        raise ValueError("Wrong Index output bound or stream shape")
    def pairs(items):
        out = {}
        for k, v in items:
            if k in out: raise ValueError("Duplicate JSON field")
            out[k] = v
        return out
    def no_number(value):
        raise ValueError("Noninteger JSON number: " + value)
    data = json.loads(raw.decode("utf-8"), object_pairs_hook=pairs, parse_constant=no_number, parse_float=no_number)
    if set(data) != {"schema", "targets"} or type(data["schema"]) is not int or data["schema"] != 1:
        raise ValueError("Wrong Index result schema")
    rows = data["targets"]
    if not isinstance(rows, list) or len(rows) != 3:
        raise ValueError("Expected exactly three Index targets")
    resolved = set()
    for row, (target, file, node, fingerprint) in zip(rows, TARGETS):
        if set(row) != {"target", "resolved_id", "file", "node", "function_fingerprint"}:
            raise ValueError("Unexpected Index fields")
        if row["target"] != target or row["file"] != file:
            raise ValueError("Index target/file differs")
        actual = row["node"]
        if not isinstance(actual, list) or not 1 <= len(actual) <= 16 or any(type(n) is not int or not 0 <= n < 2**31 for n in actual):
            raise ValueError("Invalid Index node")
        if node is not None and actual != node:
            raise ValueError("Known complete-plan node differs")
        fp = row["function_fingerprint"]
        if not isinstance(fp, str) or re.fullmatch("[0-9a-f]{64}", fp) is None or (fingerprint is not None and fp != fingerprint):
            raise ValueError("Invalid or changed control fingerprint")
        ident = row["resolved_id"]
        if not isinstance(ident, str) or not 1 <= len(ident) <= 512 or any(ord(c) < 32 for c in ident) or ident in resolved:
            raise ValueError("Invalid or duplicated resolved identity")
        resolved.add(ident)
    return data
