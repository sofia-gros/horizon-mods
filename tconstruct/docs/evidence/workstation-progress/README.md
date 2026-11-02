# Workstation advancement validation

PR #209 head `a7ed74f3` (full commit is on the PR), source JSON exactly matching the two files in the isolated runtime fixture. Tested Linux client SHA-256 `955f2d745b901f90a8a1858303d809c975f17c65e584f67cf08d100fe74a39b0` with prior guide integration tree `dbcfba54881049687da34d8c31462cede8b1802f`. Supplies were given through commands, not collected in survival.

An existing world saved with both workstations before the definitions were installed earned both when reopened with the PR files, and their timestamps were unchanged after another save/reload. In a separate LAN fixture the guest received both, got both achievements and saw their toasts; the host did not receive them. The package helper contains both definitions. This fixture overlays the exact data onto a previous build, not the final-main package; Windows is untested. The screenshot is an actual guest client capture.
