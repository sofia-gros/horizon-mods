# Mining order fixture

These tests exercise TConstruct's existing mining rule and trait math using representative material data. They do not simulate a complete game tick or verify LAN.

- A matching effective stone rule yields speed 2 and allows drops. With host speed 4, Maintained adds 3 before Jagged multiplies by 0.95, giving 6.65.
- A matching incorrect-tier rule takes precedence over a later effective rule: TConstruct rejects drops and reports no effective speed. Broken tools do not receive conditional mining traits.

The future PlutoLib evaluator must use the actual Tool component. TConstruct keeps its tier policy, broken state, and material/modifier ordering around the shared evaluation; vanilla stacks retain host behavior.
