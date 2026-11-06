# Wire source metadata for exact official 41f

This intake changes only `horizon_revision` and `file_sha256` in `pluto-lib/tools/wire-read-source.json`. The old 624c metadata and its first diagnostic refusal remain historical evidence. Runtime, borrowed reader, declaration, and guard are unchanged. The receipt retains both complete metadata objects and hashes; restoring the two original fields reconstructs the exact old bytes.

The source review in commit399aa28e found the complete private Wire declaration/new/patch/PartialEq region unchanged. Other item_data changes, including component canonicalization, are not erased or claimed equivalent.

Pure verification now uses the actual recovered complete plan7139236d, 6,383,021bytes, not a selected projection. The unchanged17f checker accepted its three exact modules, owner IDs, visibility and full copied source against immutable17f sources. There are no additional injections or states in item_data or its namespace. The pristine41f item_data SHA744c7ae0 and unchanged reader SHA4d245759 were checked.

This is a Python source/full-IR guard check, not native type checking, transport, actor, gameplay, or complete compatibility proof. The guard-only remote job remains fixed to17f and intentionally keeps its original Wire refusal; this new metadata is a separate next-integration candidate. No package, compiler, metadata, or planner was executed locally.

`verify.py PLAN OFFICIAL_ROOT MOD_ROOT` verifies the exact historical full plan and original17f guard/modules using the new metadata supplied by this intake. MOD_ROOT may contain old or adopted metadata; the verifier passes the candidate explicitly and never writes source.
