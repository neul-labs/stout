# stout-resolve

Dependency resolution for stout.

## Overview

This crate handles dependency resolution for formula packages, computing the full dependency graph and installation order.

## Features

- Resolve transitive dependencies
- Detect circular dependencies
- Compute installation order (topological sort)
- Handle optional and build dependencies
- Version constraint satisfaction

## Usage

This crate is primarily used internally by stout-install for computing what packages need to be installed.

```rust
use stout_resolve::Resolver;

let resolver = Resolver::new(&index);
let plan = resolver.resolve(&["jq", "curl"])?;
```

## License

MIT License - see the repository root for details.
