# stout-state

Local state management for stout.

## Overview

This crate manages local installation state, tracking which packages are installed, their versions, and installation metadata.

## Features

- Track installed formulas and casks
- Store installation receipts
- Manage pinned packages
- Handle package linking state
- Configuration file management

## Usage

This crate is used internally by other stout crates to query and update installation state.

```rust
use stout_state::State;

let state = State::load()?;
let installed = state.installed_formulas();
let is_pinned = state.is_pinned("jq");
```

## License

MIT License - see the repository root for details.
