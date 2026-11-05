"""Focused real ItemStack/tank planner fixture; not a Game or LAN integration test."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
from run_gold_guard import verify_lock, verify_source


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--horizon", required=True, type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    horizon = args.horizon.resolve()
    verify_source(horizon, json.loads((root / "compatibility.json").read_text())["horizon_revision"])
    target = root / "target/overburn-fixture"
    target.mkdir(parents=True, exist_ok=True)
    (target / "Cargo.toml").write_text('[package]\nname="tconstruct-overburn-fixture"\nversion="0.1.0"\nedition="2024"\n'
        '[dependencies]\nserde_json="1"\n' + ''.join(f'{n}={{path={json.dumps(str(horizon / "crates" / n))}}}\n' for n in ("horizon-world", "horizon-items")) + '[lib]\npath="lib.rs"\n')
    shutil.copyfile(horizon / "Cargo.lock", target / "Cargo.lock")
    modules = [(f"tconstruct_source_{name.replace('-', '_')}", root / f"mixins/tconstruct/source-{name}.rs") for name in ("overgrowth", "overburn", "overburn-stack")]
    modules.append(("pluto_stack_fluid", root.parent / "pluto-lib/mixins/pluto_native_creative/stack-fluid.rs"))
    (target / "lib.rs").write_text(''.join(f'#[path={json.dumps(str(path))}]\nmod {name};\n' for name, path in modules)
        + f'include!({json.dumps(str(root / "tests/rust/overburn_stack.rs"))});\n')
    subprocess.run(["cargo", "metadata", "--offline", "--format-version", "1", "--manifest-path", str(target / "Cargo.toml")], stdout=subprocess.DEVNULL, check=True)
    verify_lock(horizon / "Cargo.lock", target / "Cargo.lock")
    result = subprocess.run(["cargo", "test", "--locked", "--offline", "--manifest-path", str(target / "Cargo.toml"), "overburn_stack_tests"], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    print(result.stdout, end="")
    result.check_returncode()
    if "test result: ok. 3 passed" not in result.stdout:
        raise RuntimeError("Expected all three Overburn stack cases to pass")


if __name__ == "__main__":
    main()
