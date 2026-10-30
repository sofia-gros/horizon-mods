# Build and test Tinkers' Construct

The mod applies its Mixins to the Horizon revision in `compatibility.json`. Builds normally use the latest official Horizon Mixins revision.

## Run the lightweight tests

From the repository root:

```sh
python3 -m unittest discover -s tests -v
python3 -m unittest discover -s tconstruct/tests -v
```

## Prepare a modded source tree

You need Python, Rust, Git, a checkout of the pinned Horizon revision, and a Horizon Mixins repository. From the repository root, replace the two paths with your local checkouts:

```sh
python3 tconstruct/tools/stage.py --horizon /path/to/horizon --mixins /path/to/horizon-mixins
```

The generated source is placed in `tconstruct/target/modded`. The resolved Mixins commit is printed and saved in `tconstruct/target/mixins-revision.txt`. To reproduce a build, pass `--mixins-revision FULL_COMMIT_SHA` and ensure that commit exists in the Mixins repository. Build the client with:

```sh
cargo check --locked --manifest-path tconstruct/target/modded/Cargo.toml -p horizon-client --bin horizon-client --no-default-features
```

Preparing game resources and running the game also require Horizon 26.3 resources. On Windows, the [playtest launcher](windows-playtest.md) handles this preparation for the playtest.
