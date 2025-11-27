# Contributing to brewx

Thank you for your interest in contributing to brewx! This document provides guidelines and instructions for contributing.

## Getting Started

### Prerequisites

- Rust 1.75 or later
- Git
- Python 3.11+ with uv (for sync script development)

### Setting Up the Development Environment

```bash
# Clone the repository
git clone https://github.com/neul-labs/brewx.git
cd brewx

# Build the project
cargo build

# Run tests
cargo test --workspace

# Run with verbose logging
RUST_LOG=debug cargo run -- search json
```

## Project Structure

```
brewx/
├── src/                    # Main CLI binary
│   ├── main.rs            # Entry point
│   ├── cli/               # Command implementations
│   │   ├── mod.rs         # CLI definition (clap)
│   │   ├── install.rs
│   │   ├── search.rs
│   │   └── ...
│   └── output.rs          # Output formatting
├── crates/
│   ├── brewx-index/       # SQLite index management
│   ├── brewx-resolve/     # Dependency resolution
│   ├── brewx-fetch/       # Download management
│   ├── brewx-install/     # Package installation
│   └── brewx-state/       # Local state management
├── scripts/
│   ├── sync.py            # Index sync script
│   └── pyproject.toml     # Python dependencies
├── completions/           # Generated shell completions
├── docs/                  # Documentation
└── .github/workflows/     # CI/CD
```

## Development Workflow

### Making Changes

1. **Create a branch** for your changes:
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Make your changes** following the coding standards below.

3. **Write tests** for new functionality.

4. **Run the test suite**:
   ```bash
   cargo test --workspace
   ```

5. **Check formatting and lints**:
   ```bash
   cargo fmt --all --check
   cargo clippy --workspace --all-targets
   ```

6. **Commit your changes**:
   ```bash
   git commit -m "feat: add my feature"
   ```

7. **Push and create a pull request**.

### Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>: <description>

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

Examples:
```
feat: add parallel bottle downloads
fix: handle missing Cellar directory
docs: update installation instructions
test: add tests for dependency resolution
```

## Coding Standards

### Rust Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` for formatting
- Use `clippy` for linting
- Write documentation comments for public APIs

```rust
/// Extracts a bottle tarball to the Cellar.
///
/// # Arguments
///
/// * `bottle_path` - Path to the downloaded bottle tarball
/// * `cellar` - Path to the Homebrew Cellar
///
/// # Returns
///
/// The path to the extracted package directory.
///
/// # Errors
///
/// Returns an error if extraction fails or the bottle format is invalid.
pub fn extract_bottle(
    bottle_path: impl AsRef<Path>,
    cellar: impl AsRef<Path>,
) -> Result<PathBuf> {
    // ...
}
```

### Error Handling

- Use `thiserror` for error types in library crates
- Use `anyhow` for error handling in the CLI
- Provide helpful error messages

```rust
#[derive(Error, Debug)]
pub enum Error {
    #[error("Formula not found: {0}")]
    FormulaNotFound(String),

    #[error("Checksum mismatch for {path}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        path: String,
        expected: String,
        actual: String,
    },
}
```

### Testing

- Write unit tests in a `tests.rs` module or inline
- Use `tempfile` for tests that need filesystem access
- Test both success and error cases

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_something() {
        let tmp = tempdir().unwrap();
        // Test code...
        assert!(result.is_ok());
    }

    #[test]
    fn test_error_case() {
        let result = function_that_should_fail();
        assert!(matches!(result, Err(Error::SomeError(_))));
    }
}
```

## Adding a New Command

1. Create a new file in `src/cli/`:
   ```rust
   // src/cli/mycommand.rs
   use anyhow::Result;
   use clap::Args as ClapArgs;

   #[derive(ClapArgs)]
   pub struct Args {
       /// Description of argument
       pub arg: String,
   }

   pub async fn run(args: Args) -> Result<()> {
       // Implementation
       Ok(())
   }
   ```

2. Add to `src/cli/mod.rs`:
   ```rust
   pub mod mycommand;

   #[derive(Subcommand)]
   pub enum Command {
       // ...
       /// My new command
       MyCommand(mycommand::Args),
   }
   ```

3. Add to `src/main.rs`:
   ```rust
   match cli.command {
       // ...
       Command::MyCommand(args) => cli::mycommand::run(args).await,
   }
   ```

## Adding a New Crate

1. Create the crate:
   ```bash
   cargo new --lib crates/brewx-newcrate
   ```

2. Add to workspace in root `Cargo.toml`:
   ```toml
   [workspace]
   members = [
       # ...
       "crates/brewx-newcrate",
   ]

   [workspace.dependencies]
   brewx-newcrate = { path = "crates/brewx-newcrate" }
   ```

3. Set up the crate's `Cargo.toml`:
   ```toml
   [package]
   name = "brewx-newcrate"
   version.workspace = true
   edition.workspace = true
   license.workspace = true
   description = "Description of the crate"

   [dependencies]
   # Use workspace dependencies
   thiserror.workspace = true
   ```

## Working on the Sync Script

The sync script (`scripts/sync.py`) fetches data from the Homebrew API and generates the index.

```bash
cd scripts

# Install dependencies
uv sync

# Run dry-run (no file writes)
uv run python sync.py --dry-run

# Generate index to dist/
uv run python sync.py --output ../dist
```

## Running CI Locally

```bash
# Format check
cargo fmt --all --check

# Lint check
cargo clippy --workspace --all-targets -- -D warnings

# Tests
cargo test --workspace

# Build release
cargo build --release
```

## Documentation

- Update relevant docs when making changes
- Add docstrings to public APIs
- Keep README.md up to date

## Reporting Issues

When reporting issues, please include:

1. **brewx version**: `brewx --version`
2. **Operating system**: macOS/Linux, version
3. **Steps to reproduce**: Minimal commands to reproduce
4. **Expected behavior**: What you expected
5. **Actual behavior**: What happened
6. **Logs**: Output with `RUST_LOG=debug`

## Pull Request Process

1. Ensure all tests pass
2. Update documentation if needed
3. Add entry to CHANGELOG if significant
4. Request review from maintainers
5. Address review feedback
6. Squash commits if requested

## Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help others learn and grow

## Questions?

- Open an issue for questions
- Check existing issues first
- Tag with `question` label

Thank you for contributing!
