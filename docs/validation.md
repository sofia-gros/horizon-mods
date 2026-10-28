# Foundation validation

The fixture targets official Horizon `b6c936f82b08798057161bdfa577e3aac53fb91e`
and Horizon Mixins `0500ff24aa5933fca95a877ef41125353c4b4fe3`. The fixture is
an independent source injection into the vanilla workstation click handler. It
logs its first invocation and does not register TConstruct content.

`python3 -m unittest discover -s tests -v` verifies that pinned source exports
are built even when the original checkouts contain unrelated dirty changes.
The GitHub Actions job also stages the fixture and checks the client against the
fixed source commits. It does not run a graphical game.

In integrated-server and in-frame single-player game modes, the pinned fixture
client started with prepared Horizon 26.3 resources; interacting with a vanilla
crafting table and clicking a slot produced the fixture log message in each
mode. The README describes how to reproduce these checks. Dedicated server
behavior and arbitrary other mods are not established by this fixture.
