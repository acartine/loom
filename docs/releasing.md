# Releasing Loom

Loom releases are built from git tags and published as GitHub release artifacts.

## What ships

The release workflow publishes tarballs for:

- `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-musl`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`

Each release also includes `loom-checksums.txt`, which the install script uses for checksum verification.

## Release checklist

1. Update the version in [`Cargo.toml`](/Users/cartine/loom/Cargo.toml).
2. Run:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked
```

3. Commit the release changes.
4. Create and push a version tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

5. Wait for the `Release` GitHub Actions workflow to finish.
6. Confirm the release contains all tarballs and `loom-checksums.txt`.
7. Smoke-test the installer:

```bash
curl -fsSL https://raw.githubusercontent.com/acartine/loom/main/install.sh | sh
loom --version
```

## Install paths

The default install path is `~/.local/bin/loom`.

You can override it:

```bash
BIN_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/acartine/loom/main/install.sh | sh
```

To install a specific release:

```bash
LOOM_VERSION=v0.1.0 curl -fsSL https://raw.githubusercontent.com/acartine/loom/main/install.sh | sh
```
