# Install required dev tools (run this once after cloning)
setup:
    cargo install cargo-release

# Dry-run a release (no changes made) — pick: patch, minor, or major
release-dry level="patch":
    cargo test
    cargo release {{level}}

# Execute a release — bumps version, commits, tags, and pushes
release level="patch":
    cargo test
    cargo release {{level}} --execute
