---
name: release
description: Release a new version of Loom — bumps version, runs checks, tags, and pushes
argument-hint: <version>
disable-model-invocation: true
allowed-tools: Bash(*), Read, Edit, Grep, Glob
---

# Release Loom

Cut a new release of Loom. The version argument must be a valid semver string (e.g. `0.2.0`).

## Steps

1. **Validate version argument.** The argument must match `^[0-9]+\.[0-9]+\.[0-9]+$` (no leading `v`). Abort with a clear error if it doesn't.

2. **Check preconditions.** All of these must pass — abort on failure:
   - On the `main` branch
   - Working tree is clean (`git status --porcelain` is empty)
   - Tag `v<version>` does not already exist (`git tag -l v<version>`)
   - Pull latest: `git pull --ff-only origin main`

3. **Bump version.** Edit the workspace `Cargo.toml` at the repo root — change `version = "..."` in `[workspace.package]` to the new version. This is the single source of truth; crates inherit via `version.workspace = true`.

4. **Sync lockfile.** Run `cargo generate-lockfile` to update `Cargo.lock`.

5. **Run checks.** All must pass:
   ```bash
   cargo fmt --all --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --locked
   ```

6. **Commit.** Stage `Cargo.toml` and `Cargo.lock`, then:
   ```bash
   git commit -m "release: v<version>"
   ```

7. **Tag.**
   ```bash
   git tag v<version>
   ```

8. **Push.**
   ```bash
   git push origin main
   git push origin v<version>
   ```

9. **Report.** Print:
   - The GitHub Actions workflow URL: `https://github.com/acartine/loom/actions/workflows/release.yml`
   - Smoke-test command to run after the workflow completes:
     ```
     curl -fsSL https://raw.githubusercontent.com/acartine/loom/main/install.sh | sh && loom --version
     ```
