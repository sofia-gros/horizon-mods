# Developing mods

Each mod lives in a directory at the repository root. The repository currently contains one mod, `tconstruct/`.

```text
horizon-mods/
├── .github/              Shared CI configuration
├── tools/                Shared CI tools
├── pluto_lib/            Shared mod library
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

See [pluto lib](../pluto_lib/README.md) for the Creative tab registration API.
