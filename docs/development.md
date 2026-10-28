# Repository layout and maintenance

Each mod lives directly below the repository root. The TConstruct port keeps its content/client package and Rust core in `Tinkers' onstruct/`, and its experimental server component in `Tinkers' onstruct/server-tools/`. The directory's spelling is intentional and differs from the mod IDs `tconstruct` and `tconstruct_tools`; quote it in shell commands.

The Horizon fork owns the reusable API, runtime, SDK and generated WIT reference. This repository consumes those interfaces and owns the port's material data, recipes and behavior. The server component pins a Horizon SDK Git revision in `Cargo.toml` and `Cargo.lock`. Update and test the pair together; do not assume that an upstream version number alone means the same fork APIs are available.

Run the relevant Rust and `cargo horizon-mod` tests, check built packages and exercise gameplay for changes that affect it. Keep Minecraft jars, prepared assets, saves, credentials and build outputs out of Git. The included original GUI textures are identified by the [third-party notice](<../Tinkers' onstruct/third_party/README.md>) and the license packaged with the content mod.

The [historical index](README-history.md) keeps earlier guides, plans and verification reports. Their dates, commits and local paths are part of the evidence: follow current source and the [player guide](playing-tconstruct.md) for current setup, rather than treating historical plans as completed API contracts.
