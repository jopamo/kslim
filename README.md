<p align="center">
  <img src="kslim.png" alt="kslim logo" width="220">
</p>

# kslim

Linux kernel slimdown tool.

`kslim` is a staged kernel-tree reducer for planning, generating, and publishing slimmer Linux kernel trees.

## Quick start

```bash
cargo build
cargo run -- --help
```

Typical flow:

```bash
kslim init --upstream-url /path/to/linux/.git
kslim validate-config
kslim plan
kslim generate
```
