# Build and test Tinkers' Construct

The mod applies its Mixins to a pinned Horizon source tree. Use the versions in `compatibility.json` when preparing a build.

## Run the lightweight tests

From the repository root:

```sh
python3 -m unittest discover -s tests -v
python3 -m unittest discover -s tconstruct/tests -v
```

## Prepare a modded source tree

You need Python, Rust, Git, a checkout of the pinned Horizon revision, and a checkout of the pinned Horizon Mixins revision. From the repository root, replace the two paths with your local checkouts:

```sh
python3 tconstruct/tools/stage.py --horizon /path/to/horizon --mixins /path/to/horizon-mixins
```

The generated source is placed in `tconstruct/target/modded`. Build the client with:

```sh
cargo check --locked --manifest-path tconstruct/target/modded/Cargo.toml -p horizon-client --bin horizon-client --no-default-features
```

Preparing game resources and running the game also require Horizon 26.3 resources. On Windows, the [playtest launcher](windows-playtest.md) handles this preparation for the playtest.
