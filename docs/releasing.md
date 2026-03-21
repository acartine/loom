# Releasing Loom

Loom releases are built from git tags and published as GitHub release artifacts.

## What ships

The release workflow publishes tarballs for:

- `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-musl`
- `aarch64-apple-darwin`

Each release also includes `loom-checksums.txt`, which the install script uses for checksum verification.
Both `install.sh` and `loom update` resolve `latest` through GitHub's redirect-based release asset URLs (`/releases/latest/download/...`) instead of the Releases API, which avoids burning API rate limits for end users.

## Recommended: `/release` skill

The easiest way to cut a release is the Claude Code skill:

```
/release 0.2.0
```

This validates preconditions, bumps the version, runs checks, commits, tags, and pushes — all in one step.

## Manual release checklist

If you prefer to release manually (or the skill isn't available):

1. Update the version in the workspace [`Cargo.toml`](/Cargo.toml) (`[workspace.package]` section).
2. Sync the lockfile:

```bash
cargo generate-lockfile
```

3. Run checks:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked
```

4. Commit the release changes:

```bash
git commit -am "release: v0.2.0"
```

5. Create and push a version tag:

```bash
git tag v0.2.0
git push origin main
git push origin v0.2.0
```

6. Wait for the [Release workflow](https://github.com/acartine/loom/actions/workflows/release.yml) to finish.
7. Confirm the release contains all tarballs and `loom-checksums.txt`.
8. Smoke-test the installer:

```bash
curl -fsSL https://raw.githubusercontent.com/acartine/loom/main/install.sh | sh
loom --version
```

To test installs before pushing a new release, use the local channel installer:

```bash
scripts/release/channel-install.sh local
```

That builds the local release binary, serves a temporary mock release endpoint, and runs the real [`install.sh`](/Users/cartine/loom/install.sh) against it. To stage both the published release and your local build side by side:

```bash
scripts/release/channel-install.sh release
scripts/release/channel-install.sh local
scripts/release/channel-use.sh show
```

## Re-running a failed release

The release workflow supports `workflow_dispatch`. If a release build fails, you can re-trigger it from the GitHub Actions UI without re-pushing the tag:

1. Go to [Actions → Release](https://github.com/acartine/loom/actions/workflows/release.yml)
2. Click **Run workflow**
3. Enter the tag (e.g. `v0.2.0`) and run

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

`loom update` only supports the published release matrix above and only updates binaries that already look installed from `~/.local/bin`, `/usr/local/bin`, `/usr/bin`, or `/opt/homebrew/bin`.
