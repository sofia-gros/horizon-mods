# Bacon Tasty repair on isolated integration

The exact #263 content and #265/#269 Melter feature changes were integrated onto cc868cdb with isolated corrections, then applied, built and resource-prepared as documented in [the Melter comparison](../269-isolated-melter/README.md). Client SHA-256 `7b911c13d230d3b4c7f444c5f155ad6073853164d08d8748d8510888ac7d569b`. This is not the entire exact #263/#269 head; unmodified resource preparation remains blocked by its missing namespace declarations.

In a fresh creative Station fixture, a damage-100 pig-iron pickaxe plus dirt produced no result and preserved the inputs. Replacing dirt with the registered Bacon produced the pickaxe at damage 50. Images are actual client captures and the log records slots and output. Remaining PR-wide gates are not implied.
