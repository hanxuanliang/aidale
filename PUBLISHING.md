# Publishing Guide for Aidale

This guide outlines the complete process for publishing Aidale crates to crates.io.

## Prerequisites

### 1. Get crates.io API Token

1. Visit https://crates.io/me
2. Click "Account Settings" → "API Tokens"
3. Generate a new token
4. Run:
   ```bash
   cargo login
   ```
5. Paste your API token when prompted

### 2. Verify Git Status

```bash
git status  # Ensure working directory is clean
git log     # Verify all commits are pushed
```

## Pre-Publication Checklist

### ✅ Code Quality

- [ ] All tests pass: `cargo test --workspace`
- [ ] No clippy warnings: `cargo clippy --workspace --all-targets --all-features`
- [ ] Code is formatted: `cargo fmt --all`
- [ ] Documentation builds: `cargo doc --workspace --no-deps`

### ✅ Metadata

- [ ] Version number updated in `Cargo.toml` (workspace.package.version)
- [ ] CHANGELOG.md updated (if exists)
- [ ] README.md files present in all crates:
  - aidale-core/README.md
  - aidale-provider/README.md
  - aidale-layer/README.md
  - aidale-plugin/README.md
  - aidale/README.md

### ✅ Dependencies

- [ ] All path dependencies include version:
  ```toml
  aidale-core = { version = "0.1.0", path = "../aidale-core" }
  ```
- [ ] All workspace dependencies are pinned to appropriate versions

### ✅ License & Legal

- [ ] LICENSE file exists in workspace root
- [ ] All source files have correct license headers (optional)
- [ ] No proprietary or sensitive code included

## Publication Process

### Option 1: Automated (Recommended)

```bash
# Step 1: Run pre-flight checks
./scripts/check.sh

# Step 2: Dry-run to see what will be published
./scripts/dry-run.sh

# Step 3: Review generated packages in target/package/
ls -lh target/package/

# Step 4: Publish all crates
./scripts/publish.sh
```

### Option 2: Manual

Publish crates **in dependency order**:

```bash
# 1. Core (no dependencies)
cd aidale-core
cargo publish
cd ..
sleep 30  # Wait for crates.io index update

# 2. Provider (depends on core)
cd aidale-provider
cargo publish
cd ..
sleep 30

# 3. Layer (depends on core)
cd aidale-layer
cargo publish
cd ..
sleep 30

# 4. Plugin (depends on core)
cd aidale-plugin
cargo publish
cd ..
sleep 30

# 5. Main crate (depends on all)
cd aidale
cargo publish
cd ..
```

## Scripts Overview

### `scripts/check.sh`
Runs comprehensive pre-publication checks:
- Code formatting
- Clippy linting
- Tests
- Documentation build
- Package preview

### `scripts/dry-run.sh`
Simulates publication without uploading:
- Packages all crates
- Shows what will be uploaded
- Safe to run multiple times

### `scripts/publish.sh`
Performs actual publication:
- Runs pre-flight checks
- Publishes in dependency order
- Waits for index updates between crates
- Provides confirmation prompt

## Common Issues & Solutions

### Issue: "crate not found in registry"

**Cause**: Dependent crate not yet published or index not updated

**Solution**:
- Ensure previous crates are published
- Wait 30-60 seconds for crates.io to update
- Retry

### Issue: "failed to verify package"

**Cause**: Tests fail during packaging

**Solution**:
```bash
cargo test --workspace
cargo package --allow-dirty  # For debugging
```

### Issue: "path dependencies not allowed"

**Cause**: Path dependency missing version field

**Solution**: Add version to path dependencies:
```toml
aidale-core = { version = "0.1.0", path = "../aidale-core" }
```

### Issue: "authentication required"

**Cause**: Not logged in to crates.io

**Solution**:
```bash
cargo login
# Enter your API token
```

## Post-Publication

### 1. Verify Publication

Visit and verify each crate:
- https://crates.io/crates/aidale-core
- https://crates.io/crates/aidale-provider
- https://crates.io/crates/aidale-layer
- https://crates.io/crates/aidale-plugin
- https://crates.io/crates/aidale

### 2. Check Documentation

Documentation is automatically built at:
- https://docs.rs/aidale-core
- https://docs.rs/aidale-provider
- https://docs.rs/aidale-layer
- https://docs.rs/aidale-plugin
- https://docs.rs/aidale

Wait ~5-10 minutes for docs.rs to build.

### 3. Tag Release

```bash
git tag -a v0.1.0 -m "Release version 0.1.0"
git push origin v0.1.0
```

### 4. Create GitHub Release

1. Go to: https://github.com/hanxuanliang/aidale/releases
2. Click "Draft a new release"
3. Select tag `v0.1.0`
4. Add release notes
5. Publish release

## Version Updates

When publishing a new version:

1. Update version in `Cargo.toml`:
   ```toml
   [workspace.package]
   version = "0.1.1"  # or 0.2.0, 1.0.0
   ```

2. Update CHANGELOG.md

3. Commit changes:
   ```bash
   git add Cargo.toml Cargo.lock CHANGELOG.md
   git commit -m "chore: bump version to 0.1.1"
   git push
   ```

4. Run publication process again

## Semantic Versioning (SemVer)

- **Patch** (0.1.0 → 0.1.1): Bug fixes, no API changes
- **Minor** (0.1.0 → 0.2.0): New features, backward compatible
- **Major** (0.1.0 → 1.0.0): Breaking changes

For 0.x versions, breaking changes are allowed in minor versions.

## Emergency: Yanking a Release

If you published a broken version:

```bash
# Yank a version (hides from new projects, doesn't break existing ones)
cargo yank --vers 0.1.0 aidale-core

# Undo yank
cargo yank --undo --vers 0.1.0 aidale-core
```

**Note**: You cannot delete or replace a published version. Once published, it's permanent.

## Resources

- [Cargo Publishing Guide](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [crates.io Policies](https://crates.io/policies)
- [Semantic Versioning](https://semver.org/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

## Support

For issues or questions:
- GitHub Issues: https://github.com/hanxuanliang/aidale/issues
- Cargo Documentation: https://doc.rust-lang.org/cargo/
