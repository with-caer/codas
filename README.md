Codas are Markdown texts that document the structure of related data and their fields.

Codas aren't _just_ documentation, though: Every Coda can auto-generate efficient binary
codecs and APIs for a wide range of platforms, like
[TypeScript](codas/src/langs/typescript.rs),
[Python](codas/src/langs/python.rs),
and [Rust](codas/src/langs/rust.rs).

## What's Here (Crates!)

This repository is a [Cargo Workspace](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)
of several closely related crates:

- [`codas`](codas/): Compact and streamable data format that works anywhere--from web apps to robots.

- [`codas-macros`](codas-macros/): Macros for generating Rust data structures for codas.

- [`codas-flow`](codas-flow/): Low-latency, high-throughput bounded queues (\"data flows\") for (a)synchronous and event-driven systems.

Refer to individual crates' READMEs for more detailed info.

## License

Copyright 2025 With Caer, LLC.

Licensed under the MIT license. Refer to [the license file](LICENSE.txt) for more info.