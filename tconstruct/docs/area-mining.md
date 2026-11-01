# Sledge Hammer and Excavator area mining

This draft implements the original 3×3, face-oriented Sledge Hammer and
Excavator mining behavior through the existing per-block harvest, drop and wear
paths. The scope includes source-effective and hardness filtering, broken tools,
and integrated/LAN server behavior. Sneaking is being checked against source:
the examined box iterator and block-break path contain no sneak condition.
Creative is the current way to obtain these four-part tools: survival assembly
requires the unfinished higher-tier station and remains on the v0.5 roadmap.
Vein, tree and Scythe behaviors and expansion modifiers are separate work.

A source-based area plane and authoritative break hooks are drafted. A focused
Mixins plan accepts both hooks and their state; Rust typing and game behavior
remain unverified.
