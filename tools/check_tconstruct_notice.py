"""Verify upstream GUI and derived pickaxe attribution in the built package."""
from pathlib import Path
from zipfile import ZipFile
import hashlib
import sys

root = Path(__file__).resolve().parents[1]
archive = Path(sys.argv[1]) if len(sys.argv) > 1 else root / "Tinkers' onstruct/target/horizon-mod/tconstruct.hmod"
expected = {
    "assets/tconstruct/textures/gui/tinker.png": "db597ebec5a5a6cb06e0d702a63ee1a2371f0efb006878e65dc176fac86e8336",
    "assets/tconstruct/textures/gui/part_builder.png": "971d1b29f4cec328602fbfee415382cdef822b5afec89b63eb46a2d545cf03aa",
    "assets/tconstruct/textures/gui/panel.png": "6dce4d43b21133c75d774dfb3043bec34f9b326b9b3dd9475ff6973a8b329916",
    "assets/tconstruct/textures/gui/tinker_pattern/pick_head.png": "ab1d44ba8937357be7cbc02fcb314ce6ee5ce144073156d4eb790a37f45c8dfb",
    "assets/tconstruct/textures/gui/tinker_pattern/tool_handle.png": "06bb82ef9c82cf984549ba23a8b991290c160cffe8e7dbb98ad062a9a763fc0e",
    "assets/tconstruct/textures/gui/tinker_pattern/tool_binding.png": "5587625a90f5e2a677bbe323f504ecb80f80c3c0c45e99a7efb43cf96b4f3313",
    "assets/tconstruct/textures/gui/tinker_pattern/button_repair.png": "d2f95c50add16864c753c3e0df017ca4bfe07d2ed575a43c4629227de382824b",
    "assets/tconstruct/textures/gui/tinker_pattern/pickaxe.png": "e5908883fed304c5d5abc673dffd0ec90b3b5ab71f07c72fe531df1570d56986",
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
    for part in ("handle", "head", "binding", "broken"):
        for material in ("wood", "flint"):
            path = f"assets/tconstruct/textures/item/pickaxe_visual/{part}_{material}.png"
            if names.count(path) != 1 or package.read(path) != (root / "Tinkers' onstruct" / path).read_bytes():
                raise SystemExit(f"missing or unexpected derived sprite: {path}")
    for path, digest in expected.items():
        if hashlib.sha256(package.read(path)).hexdigest() != digest:
            raise SystemExit(f"unexpected upstream texture bytes: {path}")
print(f"verified upstream license, source, GUI textures and pickaxe sprites in {archive}")
