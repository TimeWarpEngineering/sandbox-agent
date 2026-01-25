set dotenv-load := true

# Build a single target via Docker
release-build target="x86_64-unknown-linux-musl":
    ./docker/release/build.sh {{target}}

# Build all release binaries
release-build-all:
    ./docker/release/build.sh x86_64-unknown-linux-musl
    ./docker/release/build.sh x86_64-pc-windows-gnu
    ./docker/release/build.sh x86_64-apple-darwin
    ./docker/release/build.sh aarch64-apple-darwin

# Upload binaries from dist/ (requires AWS creds + aws cli)
release-upload-binaries version latest="auto":
    {{~ if latest == "auto" ~}}
    npx tsx scripts/release/main.ts --version {{version}} --upload-binaries
    {{~ else if latest == "true" ~}}
    npx tsx scripts/release/main.ts --version {{version}} --latest --upload-binaries
    {{~ else if latest == "false" ~}}
    npx tsx scripts/release/main.ts --version {{version}} --no-latest --upload-binaries
    {{~ else ~}}
    @echo "latest must be auto|true|false" && exit 1
    {{~ endif ~}}

# Upload TypeScript artifacts + install.sh
release-upload-artifacts version latest="auto":
    {{~ if latest == "auto" ~}}
    npx tsx scripts/release/main.ts --version {{version}} --upload-typescript --upload-install
    {{~ else if latest == "true" ~}}
    npx tsx scripts/release/main.ts --version {{version}} --latest --upload-typescript --upload-install
    {{~ else if latest == "false" ~}}
    npx tsx scripts/release/main.ts --version {{version}} --no-latest --upload-typescript --upload-install
    {{~ else ~}}
    @echo "latest must be auto|true|false" && exit 1
    {{~ endif ~}}

# Full local release test: build all, then upload binaries + artifacts
release-test version latest="auto":
    just release-build-all
    just release-upload-binaries {{version}} {{latest}}
    just release-upload-artifacts {{version}} {{latest}}
