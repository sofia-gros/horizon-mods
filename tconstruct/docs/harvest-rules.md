# Material harvest tier and source-effective blocks

This draft connects source material mining tiers to actual harvest/drop checks
and resolves the original `is_effective` block tags for supported tool
definitions. It reuses synchronized per-stack tool data and the existing
`Items::correct_for_drops` boundary. Existing vanilla resources and recipes
remain unchanged. Area mining is a separate follow-up.

Acceptance requires Flint and Cobalt tools breaking diamond ore and receiving
only the correct drops, part replacement and save/reload, Mattock source
included/excluded blocks, broken behavior, LAN and representative vanilla
regression. Registered definitions alone are not evidence of working behavior.
Older saved tools need supported regeneration or part replacement to acquire
the new synchronized fields; no automatic migration is added here. The common calculation and tag preparation are implemented and have lightweight
tests; the official build and game acceptance are still pending.
