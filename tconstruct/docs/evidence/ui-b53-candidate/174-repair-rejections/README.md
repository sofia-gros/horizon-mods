# Undamaged and malformed repair rejection

Same combined candidate, client SHA, #263 content and isolated corrections as [the Flint and Brick fixture](../174-flint-and-brick/README.md). Not an unmodified exact-head build.

A fresh Station fixture rejected an undamaged Flint and Brick with Seared Brick and retained both inputs. Separately, another fresh fixture explicitly verified the damaged tool's `tic_materials:17` component before placing it with a Seared Brick; this malformed list also produced no output and retained both inputs. Actual client screenshots and control logs are included. The reusable script's final PASS text names both cases even in the second, single-case run; only the malformed case was executed there, while the undamaged evidence comes from the first log. Other #174 gates remain open.
