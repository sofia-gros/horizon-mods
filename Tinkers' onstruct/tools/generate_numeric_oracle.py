"""Run verbatim upstream FloatToolStat arithmetic methods in a small Java host.

This verifies numeric aggregation, not a running Forge/Mantle/ToolStack instance.
The host supplies only the map-to-builder plumbing; upstream method bodies are
extracted without rewriting. --check detects fixture drift without replacing it.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
UP = ROOT / "third_party/tconstruct"


def block(source, marker):
    start = source.index(marker)
    opening = source.index("{", start)
    depth = 1
    end = opening + 1
    while depth:
        depth += (source[end] == "{") - (source[end] == "}")
        end += 1
    return source[start:end]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--java", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if not args.java.is_file():
        parser.error("--java must point to the actual JDK java executable")
    source_path = UP / "src/main/java/slimeknights/tconstruct/library/tools/stat/FloatToolStat.java"
    source = source_path.read_text(encoding="utf-8")
    methods = [block(source, marker) for marker in (
        "public FloatBuilder makeBuilder()", "public void update(", "public void add(",
        "public void percent(", "public void multiply(", "public Float build(",
        "protected static class FloatBuilder")]
    cases = [
        {"name": "default", "default": 9, "operations": []},
        {"name": "update_cancels_default", "default": 9, "operations": [["update", 2]]},
        {"name": "updates_add", "default": 9, "operations": [["update", 2], ["update", 3]]},
        {"name": "double_narrowing", "default": 9, "operations": [["add", .1], ["update", 10], ["percent", -.15], ["multiply", 1.1]]},
        {"name": "percent_after_modifier_add", "default": 0, "operations": [["update", 1.25], ["percent", .1], ["update", .5], ["add", .5]]},
        {"name": "fractional_durability", "default": 1, "operations": [["update", 85], ["percent", -.15]]},
        {"name": "separate_percent_and_multiply", "default": 1, "operations": [["update", 99.1], ["percent", .123456789012], ["percent", .2], ["multiply", 1.23456789012]]},
    ]
    # Exercise material-derived stat operations, independently of the Rust core.
    data = UP / "src/generated/resources/data/tconstruct/tinkering"
    material_stats = {name: json.loads((data / f"materials/stats/{name}.json").read_text())["stats"] for name in ("wood", "flint")}
    for head in ("wood", "flint"):
        for handle in ("wood", "flint"):
            h = material_stats[head]["tconstruct:head"]
            g = material_stats[handle]["tconstruct:handle"]
            for stat, default, head_key, handle_key, base in (
                ("durability", 1, "durability", "durability", None),
                ("mining_speed", 1, "mining_speed", "mining_speed", None),
                ("attack_damage", 0, "melee_attack", "melee_damage", .5),
                ("attack_speed", 1, None, "melee_speed", 1.2),
            ):
                ops = [] if head_key is None else [["update", h[head_key]]]
                # Head/Handle apply float-scaled values, widening only at the API.
                ops.append(["percent_float", g[handle_key]])
                if base is not None:
                    ops.append(["update", base])
                if stat == "attack_damage":
                    ops.append(["add_float", .5])  # pierce level 1
                cases.append({"name": f"{head}_{handle}_{stat}", "default": default, "operations": ops})
    lines = []
    for case in cases:
        lines.append("{ var s = new ReferenceStat(Float.parseFloat(\"%s\")); var b = new ModifierStatsBuilder();" % case["default"])
        for op, value in case["operations"]:
            as_float = op == "update" or op.endswith("_float")
            method = op.removesuffix("_float")
            literal = f'Float.parseFloat("{value}")' if as_float else f'Double.parseDouble("{value}")'
            lines.append(f"s.{method}(b, {literal});")
        lines.append('float result = b.data == null ? s.defaultValue : s.build(b, b.data);')
        lines.append(f'System.out.println("{case["name"]} " + Integer.toUnsignedString(Float.floatToRawIntBits(result))); }}')
    java = """import java.util.function.Consumer;
public class ReferenceOracle {
  static class ModifierStatsBuilder {
    Object data;
    @SuppressWarnings("unchecked")
    <B> void updateStat(ReferenceStat stat, Consumer<B> consumer) {
      if (data == null) data = stat.makeBuilder();
      consumer.accept((B)data);
    }
  }
  static class ReferenceStat {
    final float defaultValue;
    ReferenceStat(float value) { defaultValue = value; }
    %s
  }
  public static void main(String[] ignored) { %s }
}
""" % ("\n".join(methods), "\n".join(lines))
    work_parent = ROOT / "target"
    work_parent.mkdir(exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="numeric-oracle-", dir=work_parent))
    generated = work / "ReferenceOracle.java"
    generated.write_text(java, encoding="utf-8")
    result = subprocess.run([str(args.java.resolve()), str(generated.resolve())], capture_output=True,
                            text=True, encoding="utf-8", check=True, timeout=45)
    outputs = dict(line.split() for line in result.stdout.splitlines())
    for case in cases:
        case["expected_bits"] = int(outputs[case["name"]])
    fixture = {"schema": 1, "scope": "verbatim FloatToolStat aggregation methods; not full Forge gameplay",
               "revision": "f4e77d43f39ba1047bf93ae816b7e1beaab514c2",
               "upstream_source_sha256": hashlib.sha256(source_path.read_bytes()).hexdigest(),
               "extracted_methods_sha256": hashlib.sha256("\n".join(methods).encode()).hexdigest(),
               "cases": cases}
    target = ROOT / "tests/fixtures/numeric-oracle.json"
    text = json.dumps(fixture, indent=2) + "\n"
    if args.check:
        if target.read_text(encoding="utf-8") != text:
            raise ValueError("Java numeric oracle fixture changed")
    else:
        target.parent.mkdir(exist_ok=True)
        target.write_text(text, encoding="utf-8")
    print(f"Java oracle: {len(cases)} bit-exact numeric cases {'verified' if args.check else 'generated'}")


if __name__ == "__main__":
    main()
