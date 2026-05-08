# stout-resolve

[![Crates.io](https://img.shields.io/crates/v/stout-resolve)](https://crates.io/crates/stout-resolve)
[![Docs.rs](https://docs.rs/stout-resolve/badge.svg)](https://docs.rs/stout-resolve)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../../LICENSE)

Fast dependency graph resolution for Homebrew-style packages, with topological ordering, circular dependency detection, and version constraint satisfaction.

**Keywords:** dependency-resolution, topological-sort, package-manager, homebrew, graph, resolver, rust, formula, constraint-satisfaction

## Why stout-resolve?

Installing a single package like `wget` often requires 5-10 transitive dependencies. Getting the order wrong means build failures. Missing an optional dependency means broken features. `stout-resolve` computes the exact set of packages to install and the correct order to install them, handling all the edge cases that make dependency management hard.

This crate powers the `stout install` command but is designed as a general-purpose resolver for any project that needs to turn a list of desired packages into an ordered installation plan.

## Features

- **Transitive Dependency Resolution** — Recursively resolves all required dependencies
- **Topological Ordering** — Computes the exact installation order (dependencies first)
- **Circular Dependency Detection** — Fails fast with detailed cycle reporting
- **Version Constraint Satisfaction** — Handles `>=`, `~>`, and exact version constraints
- **Optional & Build Dependencies** — Respects dependency types and installation context
- **Conflict Detection** — Identifies incompatible version requirements across the graph
- **Fast Graph Algorithms** — Tarjan's SCC for cycle detection, Kahn's algorithm for sorting
- **Reusable Plans** — Resolution results are serializable and cacheable

## Installation

```bash
cargo add stout-resolve
```

Or in your `Cargo.toml`:

```toml
[dependencies]
stout-resolve = "0.2"
```

## Quick Start

```rust
use stout_index::Index;
use stout_resolve::Resolver;

// Open the package index
let index = Index::open_default()?;

// Create a resolver
let resolver = Resolver::new(&index);

// Resolve dependencies for installation
let plan = resolver.resolve(&["wget", "jq"])?;

// The plan is ordered: dependencies come before dependents
for step in plan.steps {
    println!("Install: {} {}", step.name, step.version);
}

println!("Total packages to install: {}", plan.steps.len());
```

## API Overview

### Basic Resolution

```rust
use stout_resolve::{Resolver, ResolveOptions};

let resolver = Resolver::new(&index);

// Default resolution (install all required dependencies)
let plan = resolver.resolve(&["postgresql@16"])?;
```

### Resolution Options

```rust
use stout_resolve::{Resolver, ResolveOptions, DependencyFilter};

let options = ResolveOptions {
    // Include build dependencies (for source builds)
    include_build_deps: true,
    
    // Include optional dependencies
    include_optional: false,
    
    // Include recommended dependencies
    include_recommended: true,
    
    // Only resolve if not already satisfied
    filter: DependencyFilter::Missing,
    
    // Existing packages to treat as satisfied
    installed: vec!["openssl".to_string()],
};

let plan = resolver.resolve_with_options(&["curl"], &options)?;
```

### Working with the Resolution Plan

```rust
let plan = resolver.resolve(&["node"])?;

// Ordered installation steps
for (i, step) in plan.steps.iter().enumerate() {
    println!("{}. {} ({})", i + 1, step.name, step.version);
    
    // Installation source
    match &step.source {
        InstallSource::Bottle(url) => println!("   → Download bottle: {}", url),
        InstallSource::Source => println!("   → Build from source"),
    }
    
    // Why this package is needed
    for reason in &step.reasons {
        println!("   → Required by: {}", reason);
    }
}

// Check if a specific package is in the plan
let has_openssl = plan.contains("openssl");

// Get the planned version of a package
let jq_version = plan.version_of("jq");

// Total download size estimate
println!("Estimated download: {}", humansize::format_size(plan.total_size));
```

### Dependency Graph Inspection

```rust
use stout_resolve::DependencyGraph;

let graph = resolver.build_graph(&["wget"])?;

// Inspect the raw graph
for node in &graph.nodes {
    println!("{} depends on:", node.name);
    for edge in &node.dependencies {
        println!("  - {}", edge.name);
    }
}

// Find all transitive dependencies of a package
let deps = graph.transitive_dependencies("wget")?;

// Find all packages that depend on a given package (reverse deps)
let dependents = graph.reverse_dependencies("openssl")?;

// Check for optional dependencies that could be enabled
let optional = graph.optional_dependencies("postgresql@16")?;
```

### Circular Dependency Detection

```rust
match resolver.resolve(&["some-formula"]) {
    Ok(plan) => { /* proceed with installation */ }
    Err(ResolveError::CircularDependency(cycle)) => {
        eprintln!("Circular dependency detected!");
        for (i, pkg) in cycle.iter().enumerate() {
            eprintln!("  {} → {}", pkg, cycle[(i + 1) % cycle.len()]);
        }
    }
    Err(e) => eprintln!("Resolution failed: {}", e),
}
```

### Version Constraint Handling

```rust
use stout_resolve::VersionConstraint;

// Parse version constraints
let constraint = VersionConstraint::parse(">= 1.0, < 2.0")?;

// Check if a version satisfies a constraint
let satisfies = constraint.is_satisfied_by("1.5.0")?; // true
let not_satisfies = constraint.is_satisfied_by("2.1.0")?; // false

// Resolution automatically handles version constraints from the index
// and will fail if no satisfying version exists
```

## Performance

Resolution performance depends on graph complexity:

| Scenario | Packages | Time |
|----------|----------|------|
| Simple formula (no deps) | 1 | <1ms |
| Common tool (jq) | 3-5 | 2-5ms |
| Complex app (node) | 10-20 | 5-15ms |
| Heavy dependency tree (postgresql) | 30-50 | 20-50ms |
| Large bundle (100+ formulas) | 100+ | 100-300ms |

The resolver caches index metadata across resolutions, so repeated resolves of overlapping package sets are faster.

## Algorithm

`stout-resolve` uses a multi-phase resolution algorithm:

1. **Graph Construction** — Build a dependency DAG from the index, expanding transitive dependencies
2. **Cycle Detection** — Run Tarjan's strongly connected components algorithm; any SCC with >1 node is a cycle
3. **Constraint Propagation** — Validate that all version constraints can be simultaneously satisfied
4. **Topological Sort** — Use Kahn's algorithm to produce a valid installation order
5. **Plan Generation** — Enrich the sorted nodes with installation sources, sizes, and reasons

## Integration with the Stout Ecosystem

`stout-resolve` sits at the heart of stout's installation flow:

- **stout-index** provides the metadata that the resolver queries
- **stout-fetch** downloads the packages in the resolved plan
- **stout-install** executes the installation steps in the resolved order
- **stout-state** checks which packages are already installed to avoid redundant work

You can use `stout-resolve` standalone for any project that needs to turn package names into ordered installation plans.

## License

MIT License — see the [repository root](../../LICENSE) for details.
