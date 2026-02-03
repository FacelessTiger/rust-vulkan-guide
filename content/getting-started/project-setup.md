+++
date = '2026-02-02T14:41:45-07:00'
title = 'Project Setup'
+++
## Workspace setup
For reasons, we'll get into later, we are gonna want to organize our project into a workspace. At the root of your project make a `Cargo.toml` and put this inside:
```toml
[workspace]
resolver = "3"
members = [
	"guide"
]

[patch.crates-io]
ash = { git = "https://github.com/FacelessTiger/ash.git" }
```
Note the patch, since Vulkan is a C API we're gonna want to use Rust bindings to actually interface with it. Luckily the `ash` crate is great bindings! An issue though is that as of writing this article it's pretty out-of-date due to the maintainers focusing on rewriting how they generate the bindings. So, we're going to patch in a custom fork that just merges [this pr that upgrades the Vulkan headers to 1.4.341](https://github.com/ash-rs/ash/pull/1028), [this one that adds wrappers for Vulkan 1.4 device functions](https://github.com/ash-rs/ash/pull/1000), and finally [this one that adds functions for descriptor heap](https://github.com/ash-rs/ash/pull/1027). 

If you don't know what all that means don't worry about it, it effectively just makes it up to date enough for our purposes.

## Project setup
Now create the `guide` binary crate as usual, with the given `Cargo.toml`
```toml
[package]
name = "guide"
version = "0.1.0"
edition = "2024"

[dependencies]
ash = { git = "https://github.com/FacelessTiger/ash.git" }
gpu-allocator = "0.28.0"
```
Vulkan normally requires you to manually handle memory allocations for images and buffers and whatever else. Normally making a whole custom allocator is overboard for most projects, so we'll just use the `gpu-allocator` crate for that.

## End result
If you did everything correct your directory layout should look something like below:
````tree
- project_name | folder
  - guide | folder
    - src | folder
      - main.rs | fa-fw fab fa-rust | accent
    - Cargo.toml | file-alt | secondary
  - Cargo.lock | file-alt | secondary
  - Cargo.toml | file-alt | secondary
````
## Validation layers
Finally, to end this quick section off, you're gonna want to install the [latest Vulkan SDK](https://vulkan.lunarg.com/sdk/home) then have the `Vulkan Configurator` open in the background with the `Validation` configuration selected. When this tool is open it injects into any Vulkan app that runs and outputs in the console any potential issues (note anything after one validation error is undefined behavior). Feel free to modify any of the default configuration checks to your liking, one of note is the `Break` option under the `Debug Action` section. That'll cause a debug break when there's an error so you can figure out the exact problematic line when connected to a debugger.