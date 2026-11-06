# Windows path/handle stat compatibility

Source-only follow-up to the actual Windows formatter failure in run 42253259752,
controller 38e1b11d. The original false outcome is retained in result 571b10f8.
No formatter, compiler, archive download or Windows execution occurs here.

CPython 3.12.10 Modules/posixmodule.c update_st_mode_from_path adds 0111 to the
mode of regular paths ending in .exe/.bat/.cmd/.com, case-insensitively. Its
Python/fileutils.c _Py_fstat_noraise has no filename and does not add those bits.
This is a concrete cross-API discrepancy. The failed run did not record numeric
stat fields, so this candidate does not assume away any additional difference.

The original signature function is byte-for-byte unchanged. A new, narrowly used
require_opened_file adapter translates only that known Windows filename mode
projection when comparing a path stat to its freshly opened handle. It does not
mask any permission bits, accept unknown extensions, or change POSIX comparison.
File type, device, inode, size, nanosecond mtime, link count, file attributes and
reparse tag must still agree. Symlinks/reparse/special types remain rejected.
Failure includes only the bounded numeric signatures, not file contents.

All three affected read paths keep exact path-before/path-after comparison and
exact handle-opened/handle-after comparison, including execute bits. The three
paths are formatter admission, Pluto input fingerprinting, and the latest
launcher file hashing. The latter hashes the selected toolchain containing .exe
files, so the same source defect would otherwise block the real launcher even
after a formatter-only repair. No cache key, directory traversal, byte budget,
guard algorithm, package, profile, toolchain choice, environment or PS entry is
changed. nlink/attributes/tag checks strengthen the previous read signature.

Twelve pure/model tests pass. They cover all four extensions, POSIX, unsupported
extensions, non-execute mode changes, every other identity field, same-API drift,
all three file-backed read paths, budgets, and exact whole-file inverse of the
three production changes. Synthetic Windows stats are not actual Windows proof.
All three source files are exact delta-only from controller 38e1b11d; inputs.json
retains before/after hashes and reversible changes. Native retry remains blocked
until source review and explicit CI release. Existing metadata, archive pins,
formatter output oracles and historical evidence remain unchanged.

References:
- https://github.com/python/cpython/blob/v3.12.10/Modules/posixmodule.c
- https://github.com/python/cpython/blob/v3.12.10/Python/fileutils.c
