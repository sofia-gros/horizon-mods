# Play on Windows

The launcher prepares a separate test world and starts Horizon with Tinkers' Construct. Windows has not yet been tested on a PC. A reported startup error is still unresolved.

## Before you start

You need a working Horizon 26.3 resource folder, Git, Python 3.12 or later, the Rust stable MSVC toolchain, Visual Studio Build Tools with Desktop development with C++, and a Vulkan-capable GPU. The launcher does not install these tools for you.

Java and Forge do not need to run. The launcher uses your existing Horizon resources and bundled mod data; it does not fetch a Minecraft 1.20.1 client.

## Start the game

1. [Download the playtest branch](https://github.com/sofia-gros/horizon-mods/archive/refs/heads/windows-all-usertest.zip) and extract it, or use an existing checkout of `windows-all-usertest`.
2. Open the `tconstruct` folder and double-click **Play-TConstruct.cmd**.
3. On the first run, wait while the launcher prepares and builds the game. If it cannot find your Horizon resources, select the prepared resource folder when prompted.

The first build can take time. Later runs reuse the build when possible. If an error occurs, the window stays open so you can read it. The current build has no sound or controller support.

The launcher stores its files in `%LOCALAPPDATA%\TConstruct-Horizon-Playtest` and uses a separate `TConstruct-Playtest` world. It does not overwrite your existing Horizon or Minecraft worlds.

Once the game starts, try [making a part](playing.md).
