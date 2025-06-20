# Release Process

This document describes the release process for the mongo-rabbitmq-connector project.

## Overview

The project uses GitHub Actions to automate the release process, which includes:

1. **Semantic Version Check** - Ensures changes follow semantic versioning
2. **Cargo Publish Dry Run** - Validates packages can be published
3. **Cargo Publish** - Publishes packages to crates.io
4. **GitHub Release** - Creates a release with changelog

## Prerequisites

Before releasing, ensure you have:

1. **Cargo Registry Token**: Set up `CARGO_REGISTRY_TOKEN` secret in GitHub repository
   - Generate token at https://crates.io/settings/tokens
   - Add as repository secret in GitHub Settings → Secrets and variables → Actions

2. **Permissions**: Write access to the repository for creating releases

3. **Dependencies**:
   - `cargo-semver-checks` installed for local testing: `cargo install cargo-semver-checks`

## Release Steps

### 1. Prepare the Release

Use the provided script to prepare a new release:

```bash
./prepare-release.sh <version>
```

For example:
```bash
./prepare-release.sh 0.2.0
```

This script will:
- Validate version format
- Update version in `mrcon/Cargo.toml` and `mrcond/Cargo.toml`
- Update `Cargo.lock`
- Run tests to ensure everything works
- Run semver checks (if available)
- Commit changes
- Create a git tag

### 2. Push Changes

After the script completes successfully:

```bash
# Push the version bump commit
git push origin main

# Push the tag to trigger the release workflow
git push origin v<version>
```

### 3. Monitor the Release

The GitHub Actions workflow will automatically:

1. **Semver Check**: Verify semantic versioning compliance
2. **Dry Run**: Test publishing without actually publishing
3. **Publish**: Publish both `mrcon` and `mrcond` to crates.io
4. **GitHub Release**: Create a release with changelog

Monitor the progress at: `https://github.com/f-squirrel/mrcond/actions`

## Release Workflow Details

### Semantic Version Checking

The workflow uses `cargo-semver-checks` to ensure:
- Public API changes follow semantic versioning rules
- Breaking changes bump major version
- New features bump minor version
- Bug fixes bump patch version

### Publishing Order

Packages are published in dependency order:
1. `mrcon` (library) is published first
2. Wait for availability on crates.io
3. `mrcond` (CLI) is published second

### GitHub Release

The workflow automatically creates a GitHub release with:
- Release notes from CHANGELOG.md (if available)
- Installation instructions
- Links to published crates

## Manual Release (Emergency)

If the automated workflow fails, you can publish manually:

```bash
# Publish mrcon first
cargo publish -p mrcon

# Wait a few minutes for propagation, then publish mrcond
cargo publish -p mrcond

# Create GitHub release manually through the web interface
```

## Version Strategy

This project follows [Semantic Versioning](https://semver.org/):

- **MAJOR** version for incompatible API changes
- **MINOR** version for backwards-compatible functionality additions
- **PATCH** version for backwards-compatible bug fixes

### Pre-release Versions

For pre-release versions, use suffixes like:
- `1.0.0-alpha.1` for alpha releases
- `1.0.0-beta.1` for beta releases
- `1.0.0-rc.1` for release candidates

## Troubleshooting

### Common Issues

1. **Semver Check Fails**
   - Review breaking changes in your code
   - Consider if version bump is appropriate
   - Update version number if needed

2. **Publish Fails**
   - Check if crate name is already taken
   - Verify CARGO_REGISTRY_TOKEN is set correctly
   - Ensure all dependencies are available on crates.io

3. **GitHub Release Fails**
   - Check repository permissions
   - Verify GITHUB_TOKEN has sufficient permissions

### Getting Help

- Check GitHub Actions logs for detailed error messages
- Review crates.io publishing guidelines
- Consult Cargo documentation for publishing issues

## Changelog Management

If you maintain a `CHANGELOG.md` file:
- Follow [Keep a Changelog](https://keepachangelog.com/) format
- Update it before each release
- The release workflow will extract relevant sections automatically

Example changelog entry:
```markdown
## [0.2.0] - 2024-01-15

### Added
- New feature X
- Support for Y

### Changed
- Improved performance of Z

### Fixed
- Bug in component A
```
