"""Verify upstream GUI attribution in the built TConstruct package."""
from pathlib import Path
from zipfile import ZipFile
import hashlib
import sys

root = Path(__file__).resolve().parents[1]
archive = Path(sys.argv[1]) if len(sys.argv) > 1 else root / "Tinkers' onstruct/target/horizon-mod/tconstruct.hmod"
expected = {
    "assets/tconstruct/textures/gui/tinker.png": "db597ebec5a5a6cb06e0d702a63ee1a2371f0efb006878e65dc176fac86e8336",
    "assets/tconstruct/textures/gui/part_builder.png": "971d1b29f4cec328602fbfee415382cdef822b5afec89b63eb46a2d545cf03aa",
}
with ZipFile(archive) as package:
    names = package.namelist()
    license_path = "assets/tconstruct/license.txt"
    notice_path = "assets/tconstruct/source_notice.txt"
    for path in [license_path, notice_path, *expected]:
        if names.count(path) != 1:
            raise SystemExit(f"missing or duplicate archive entry: {path}")
    license_bytes = package.read(license_path)
    reference = (root / "Tinkers' onstruct/third_party/tconstruct/LICENSE").read_bytes()
    if license_bytes != reference:
        raise SystemExit("packaged MIT notice differs from the pinned upstream license")
    notice = package.read(notice_path).decode("utf-8")
    if "f4e77d43f39ba1047bf93ae816b7e1beaab514c2" not in notice:
        raise SystemExit("packaged notice lacks the pinned source revision")
    for path, digest in expected.items():
        if hashlib.sha256(package.read(path)).hexdigest() != digest:
            raise SystemExit(f"unexpected upstream texture bytes: {path}")
print(f"verified upstream license, source and GUI textures in {archive}")
