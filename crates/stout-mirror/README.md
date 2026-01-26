# stout-mirror

Offline mirror creation and serving for stout.

## Overview

This crate enables creating and serving offline mirrors for air-gapped environments. It downloads all required packages and their dependencies to a local directory that can be served via HTTP.

## Features

- Create offline mirrors with specified packages
- Download all transitive dependencies
- Serve mirrors via built-in HTTP server
- Verify mirror integrity
- Support for both formulas and casks

## Usage

This crate is primarily used internally by the `stout` CLI through the `stout mirror` commands.

```rust
use stout_mirror::Mirror;

let mirror = Mirror::create("/path/to/mirror", &packages).await?;
mirror.serve("0.0.0.0:8080").await?;
```

## License

MIT License - see the repository root for details.
