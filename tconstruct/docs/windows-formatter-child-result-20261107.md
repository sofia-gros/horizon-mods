# Windows formatter raw-status observation

Run [42329829285](https://github.com/sofia-gros/horizon-mods/actions/runs/42329829285), job 128273554364, exact controller 9846c1d38083b791efb809d9156c8a1e5cf539e6, ended in failure at 2026-11-07 02:33:49 UTC.

The 33 controller and 12 stat tests passed. The same pinned official rustfmt archive and executable passed byte/layout admission. The direct rustfmt --version child returned 3221225781 (0xC0000135, Windows STATUS_DLL_NOT_FOUND), while the Python worker returned 4294967295. Output was zero bytes and elapsed time was about 0.047 seconds. This separates the raw loader status from the wrapper conversion. It does not identify the missing DLL, its supplying component, or prove that the earlier eb19 failure had the same raw status.

All four owned Jobs were empty after cleanup. The final receipt is passed=false and verified=true; source and prepared-input seals matched. Version validation and all seven damage/terrain regeneration calls were not reached. No current-composition, apply, Windows client link or game success is claimed.

No automatic retry, additional DLL download or alternate toolchain was attempted. The ordinary job-log tool returned 21,946 bytes (SHA256 f71a55ffab636a31211bf12290c77dbe5b4029ebac62efdfa8089dfb8b4102bc). Raw console is retained locally and in the job, not republished here. The adjacent JSON is a bounded project-specific result summary.
