# Windows formatter retry: version phase failed

Fixed head eb19d445ab8d8e152218022c1d22cb2896e16ac0, [run 42255794408](https://github.com/sofia-gros/horizon-mods/actions/runs/42255794408), job 128022509253, ended with FAILURE at 2026-11-06 16:49:01 UTC.

The Windows run passed 20 controller tests and 12 stat tests. Archive/source preparation succeeded: the exact official archive and selected executable bytes were admitted. The following version phase returned wrapper exit 4294967295 in about 0.094 seconds, with 0 output bytes. No version was observed and none of the seven helper regeneration calls ran.

The final receipt remained passed=false and verified=true. All four owned jobs were empty at phase completion, and source/prepared-input seals remained unchanged. This is not proof of current composition, Windows linking or game startup.

The failure cause is unresolved. The wrapper exit alone does not establish the raw formatter process status or a missing DLL. No fallback component, extra download, automatic retry or threshold relaxation was performed.

On 2026-11-07, the normal job-log connector recovered the same decoded console: 22,054 bytes, SHA256 8562061289482e84500df0bf75edd9e8b60ad699855435bfceffaef50295f91d. Only this small summary and structured phase facts are published, not the full console. The prior source checkpoints remain unchanged.
