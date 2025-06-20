#!/bin/bash

# Release preparation script for mongo-rabbitmq-connector
# This script helps prepare a new release by updating versions and creating a git tag

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if version argument is provided
if [ $# -eq 0 ]; then
    print_error "Usage: $0 <version>"
    print_error "Example: $0 0.2.0"
    exit 1
fi

VERSION=$1

# Validate version format (basic semver check)
if ! [[ $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.-]+)?$ ]]; then
    print_error "Invalid version format. Please use semantic versioning (e.g., 1.0.0, 1.0.0-beta.1)"
    exit 1
fi

print_status "Preparing release v$VERSION"

# Check if we're on the main/master branch
CURRENT_BRANCH=$(git branch --show-current)
if [[ "$CURRENT_BRANCH" != "main" && "$CURRENT_BRANCH" != "master" ]]; then
    print_warning "You're not on the main/master branch (current: $CURRENT_BRANCH)"
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_error "Release preparation cancelled"
        exit 1
    fi
fi

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
    print_error "You have uncommitted changes. Please commit or stash them first."
    exit 1
fi

# Update version in Cargo.toml files
print_status "Updating version in Cargo.toml files..."

# Update mrcon/Cargo.toml
sed -i "s/^version = \".*\"/version = \"$VERSION\"/" mrcon/Cargo.toml
if grep -q "version = \"$VERSION\"" mrcon/Cargo.toml; then
    print_status "Updated mrcon/Cargo.toml to version $VERSION"
else
    print_error "Failed to update mrcon/Cargo.toml"
    exit 1
fi

# Update mrcond/Cargo.toml
sed -i "s/^version = \".*\"/version = \"$VERSION\"/" mrcond/Cargo.toml
if grep -q "version = \"$VERSION\"" mrcond/Cargo.toml; then
    print_status "Updated mrcond/Cargo.toml to version $VERSION"
else
    print_error "Failed to update mrcond/Cargo.toml"
    exit 1
fi

# Update mrcon dependency version in mrcond/Cargo.toml for publishing
print_status "Updating mrcon dependency version in mrcond/Cargo.toml..."
sed -i "s/mrcon = { path = \"..\/mrcon\" }/mrcon = { version = \"$VERSION\", path = \"..\/mrcon\" }/" mrcond/Cargo.toml
if grep -q "mrcon = { version = \"$VERSION\"" mrcond/Cargo.toml; then
    print_status "Updated mrcon dependency to version $VERSION"
else
    print_warning "Could not update mrcon dependency version - you may need to do this manually"
fi

# Update Cargo.lock
print_status "Updating Cargo.lock..."
cargo update --workspace

# Run tests to make sure everything still works
print_status "Running tests..."
if ! make test; then
    print_error "Tests failed. Please fix the issues before releasing."
    exit 1
fi

# Run semver checks
print_status "Running semver checks..."
if command -v cargo-semver-checks &> /dev/null; then
    if ! cargo semver-checks check-release; then
        print_warning "Semver checks failed. This might indicate breaking changes."
        read -p "Continue anyway? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_error "Release preparation cancelled"
            exit 1
        fi
    fi
else
    print_warning "cargo-semver-checks not installed. Skipping semver checks."
    print_warning "Install with: cargo install cargo-semver-checks"
fi

# Commit the version changes
print_status "Committing version changes..."
git add mrcon/Cargo.toml mrcond/Cargo.toml Cargo.lock
git commit -m "chore: bump version to $VERSION"

# Create and push tag
print_status "Creating and pushing tag v$VERSION..."
git tag -a "v$VERSION" -m "Release v$VERSION"

print_status "Release preparation complete!"
print_status ""
print_status "Next steps:"
print_status "1. Push the changes: git push origin $CURRENT_BRANCH"
print_status "2. Push the tag: git push origin v$VERSION"
print_status "3. The GitHub Actions workflow will automatically:"
print_status "   - Run semver checks"
print_status "   - Perform dry-run publish"
print_status "   - Publish to crates.io"
print_status "   - Create GitHub release"
print_status ""
print_status "Make sure you have set the CARGO_REGISTRY_TOKEN secret in your GitHub repository!"
