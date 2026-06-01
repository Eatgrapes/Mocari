# Rusty Live2D

A pure Rust Live2D/Cubism runtime experiment with a `wgpu` renderer.

> [!WARNING]
> This project is still a work in progress.

## Why

Live2D Cubism Core is closed source. For a long time, developers have mostly had to use **native bindings** to call it from languages like Rust.
That approach limits portability, integration, and optimization, which is why Rusty-Live2D exists.
Rust is fast and reliable, making it a strong choice for rebuilding a Live2D-compatible runtime.

## Goal

Rusty-Live2D aims to become a practical **Rust library**.
It should be easy to use, easy to call, and simple to integrate without complicated native runtime setup.

## Build

You need:

- **Rust**
- **Cargo**

```bash
git clone https://github.com/Eatgrapes/Rusty-Live2d.git
cd Rusty-Live2D
cargo build
```

## Status

This project currently implements:

- [x] Reading `.moc3` files
- [x] Rendering models correctly
- [ ] Rendering model motions
- [ ] Rendering model expressions
- [ ] More Cubism runtime features

## Statement

Rusty Live2D is an unofficial and independent experimental project.
This project is not affiliated with, endorsed by, sponsored by, or certified by Live2D Inc. "Live2D" and "Cubism" are trademarks or registered trademarks of their respective owners.
This repository does not contain Live2D Cubism Core, Live2D SDK binaries, official source code, or any proprietary files distributed by Live2D Inc.
The purpose of this project is to explore a pure Rust runtime and renderer for compatible 2D model data. It is provided for educational, research, and interoperability purposes only.
Users are responsible for ensuring that their use of this project complies with the licenses of Live2D Inc., model creators, asset owners, and applicable laws.
If you are a rights holder and believe that any content in this repository should not be published, please contact the maintainer or open an issue, and the relevant content will be reviewed.
