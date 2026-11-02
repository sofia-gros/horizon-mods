# Isolated Melter feature integration provenance

- Base source `cc868cdb5d335278df1aa819b2a2e817d75a6a74`.
- #263 content patch from parent to `e7516a84855ff7d09823d99d93dc2dcb9c967bd2`: SHA-256 `0fdb167c8ea9a72035446a5d66daebc3befab242e101fbb2173c526dd3911f2e`.
- #265 plus #269 feature patch from `e7516a84855ff7d09823d99d93dc2dcb9c967bd2` to `ac3af2fc4979a8214bfc6897b7245723a13dd2b0`, restricted to TConstruct Mixins: SHA-256 `fc350d22189ff544908613c672da8c975cb297d31c49cde4a11bd52582a64737`.
- Host retargeted to `b53cfcd14eab7b6ed2fe91a3a3773d7415402ab9`, Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`; retarget patch SHA-256 `1c27bab46158bf8457e235d958f5dd4941de55c37b9d37698ca9e04bdc9d07c8`.
- Isolated resource prepare namespace declaration patch SHA-256 `e855822d79346c55eb6e5816b263e3b2c77df7667cab1a7820eb300866eb0415`, and Pluto helper comment-only patch `efa47deffaa08a72900ad1edb05071b62fd12cc8b2827c0171229bf201f638e0`.
- This tests exact #265/#269 feature files on a narrower base, not the entire exact #269 head. Full #269 application hit the separate unsupported Splash injection context.
