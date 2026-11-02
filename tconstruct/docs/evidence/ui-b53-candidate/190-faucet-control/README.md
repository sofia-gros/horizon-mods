# Faucet and copper casting control

Same combined candidate, client SHA and isolated corrections as [the lower-tank fixture](../205-lower-tank/README.md), using a copy of a world whose saved Melter contained 900 mB molten copper. Not an unmodified exact-head build.

The faucet and table were replaced in the disposable copied world, an ingot cast was inserted, the faucet clicked, and a copper ingot recovered after cooling. The table reported both ingot cast and copper output, and the saved Melter afterwards held 810 mB. Actual client screenshots and command logs are included. This establishes the ordinary faucet/casting setup for a control only. Earlier iron-to-copper helmet part-swap attempts produced no swap; those materials have differing source traits and may meet the conservative rejection policy, so those attempts do not establish a #190 regression. An eligible positive swap and independent #190 gates remain unverified.
