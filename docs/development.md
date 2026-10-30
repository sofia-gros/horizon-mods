# Developing mods

Each mod lives in a directory at the repository root. The repository currently contains one mod, `tconstruct/`.

```text
horizon-mods/
├── .github/              Shared CI configuration
├── tools/                Shared CI tools
├── pluto-lib/            Shared mod library
└── tconstruct/
    ├── README.md         Mod overview
    ├── docs/             Guides and screenshots
    ├── mixins/           Horizon Mixins and Rust code
    ├── assets/           Mod assets
    ├── content/          Reference data
    ├── tests/            Mod tests
    └── tools/            Build and resource preparation tools
```

Start with the [Tinkers' Construct development guide](../tconstruct/docs/development.md) for its requirements, build commands, and tests. A new mod should keep its own implementation, resources, tests, and documentation in its own top-level directory. Shared repository configuration belongs at the root.

See [pluto lib](../pluto-lib/README.md) for the Creative tab registration API.

## Validate a change

After preparing a branch, run the full Mixins checks once for its final commit:

```sh
gh workflow run mixins.yml --ref BRANCH
gh run list --workflow mixins.yml --branch BRANCH
```

Check that the run's head SHA matches the commit being reviewed and that the
run succeeds before merging. A later commit needs its own validation. The
workflow runs on request, so ordinary pushes do not start a build.
