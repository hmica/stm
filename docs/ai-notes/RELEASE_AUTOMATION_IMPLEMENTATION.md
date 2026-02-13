# Release Automation Implementation

**Date**: February 2025
**Version**: 0.1.0
**Status**: Complete

## Summary

Implemented automated cross-platform binary builds and GitHub releases for STM using GitHub Actions. This enables one-command releases for Linux, macOS (Intel + Apple Silicon), and Windows.

## Files Created

### `.github/workflows/release.yml` (NEW)

Automated release workflow triggered on git tags matching `v*` pattern.

**Key Features**:
- **Validate job**: Extracts version from tag, verifies it matches Cargo.toml
- **Build job**: Matrix build for 4 platforms (runs in parallel)
  - `x86_64-unknown-linux-gnu` on `ubuntu-latest`
  - `x86_64-apple-darwin` on `macos-13`
  - `aarch64-apple-darwin` on `macos-14`
  - `x86_64-pc-windows-msvc` on `windows-latest`
- **Release job**: Creates GitHub release with all binaries and checksums

**Build Process per Platform**:
1. Check out code
2. Install Rust toolchain for target
3. Build: `cargo build --release --target <target>`
4. Strip binary (Unix only) using `strip`
5. Create staging directory with binary, README.md, and config.example.toml
6. Archive: tar.gz (Unix) or zip (Windows)
7. Generate SHA256 checksum
8. Upload as artifact

**Release Creation**:
- Uses `softprops/action-gh-release@v2`
- Auto-generates release notes from commits
- Uploads all platform binaries + checksums.txt
- Auto-detects pre-release from tag (beta/alpha/rc)

## Files Modified

### `Cargo.toml`

Added:
- `repository` field for GitHub link (needs to be customized with actual repo URL)
- Release profile optimizations:
  ```toml
  [profile.release]
  lto = true          # Link-time optimization
  codegen-units = 1   # Better optimization
  strip = true        # Strip symbols
  ```

**Impact**: ~20-30% smaller binaries, ~2x longer build time

### `README.md`

Updated install section with:
- Binary download instructions for all 4 platforms
- Example commands for macOS (ARM64 & Intel), Linux, Windows
- Kept cargo install and build-from-source options
- All binary URLs use placeholder `yourusername` (needs updating)

### `.github/workflows/ci.yml`

Enhanced test job:
- Added matrix strategy to run tests on `ubuntu-latest`, `macos-latest`, `windows-latest`
- Catches platform-specific issues before release

## Usage

### Creating a Release

1. **Update version in `Cargo.toml`** to match desired release (e.g., 0.1.0 → 0.2.0)

2. **Create and push tag**:
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

3. **Watch workflow**:
   - GitHub Actions runs automatically
   - Validate job checks tag version
   - Build job compiles all 4 platforms in parallel (~5-10 minutes)
   - Release job creates GitHub release with all artifacts

4. **Verify release**:
   - Visit [Releases](https://github.com/yourusername/stm/releases)
   - Download binaries for your platform
   - Verify SHA256 checksums: `sha256sum -c checksums.txt`

### Pre-Release Tags

Tags with `-beta`, `-alpha`, or `-rc` are automatically marked as pre-release:
```bash
git tag v0.2.0-beta.1
git push origin v0.2.0-beta.1
```

## Binary Artifacts

Each release includes:
- `stm-{version}-x86_64-unknown-linux-gnu.tar.gz` (~2MB)
- `stm-{version}-x86_64-apple-darwin.tar.gz` (~2MB)
- `stm-{version}-aarch64-apple-darwin.tar.gz` (~2MB)
- `stm-{version}-x86_64-pc-windows-msvc.zip` (~2MB)
- `checksums.txt` (SHA256 hashes for all artifacts)

Each archive contains:
- `stm` (or `stm.exe` for Windows)
- `README.md`
- `config.example.toml`

## Configuration URLs

The following URLs in `README.md` need to be customized before first release:

1. `Cargo.toml`: `repository` field
2. `README.md`:
   - GitHub Releases URL
   - Binary download URLs (4 places)
   - Repository clone URL

Replace `yourusername` with actual GitHub username or org name.

## Testing the Workflow

To test before real release:

```bash
# Create test tag
git tag v0.1.0-test
git push origin v0.1.0-test

# Wait for workflow to complete on GitHub Actions
# Verify binaries download and run correctly
# Delete test release from GitHub UI
# Delete tag locally and remotely
git tag -d v0.1.0-test
git push origin :refs/tags/v0.1.0-test
```

## Future Enhancements

Potential improvements for v0.2+:
- musl Linux builds for static binaries
- Universal macOS binary (lipo x86_64 + aarch64)
- Installation script (install.sh)
- Homebrew tap / AUR package
- cargo publish to crates.io
- Automated changelog generation from commits
- Binary size optimization tracking

## Notes

- Release builds benefit from Cargo.toml optimizations (LTO, strip)
- Multi-platform CI tests catch issues early
- Workflow uses native runners (no cross-compilation) for reliability
- All platforms build in parallel for fast releases
- SHA256 checksums provide integrity verification
