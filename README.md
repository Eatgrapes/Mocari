# Mocari

A pure Rust Live2D/Cubism runtime experiment

## Why

Live2D Cubism Core is closed source. For a long time, developers have mostly had to use **native bindings** to call it from languages like Rust.
That approach limits portability, integration, and optimization, which is why **Mocari** exists.
Rust is fast and reliable, making it a strong choice for rebuilding a Live2D-compatible runtime.

## Goal

**Mocari** aims to become a practical **Rust library**.
It should be easy to use, easy to call, and simple to integrate without complicated native runtime setup.

## Install

**Mocari** is published on [crates.io](https://crates.io/crates/mocari):

```bash
cargo add mocari
```

API documentation is hosted on [docs.rs/mocari](https://docs.rs/mocari).

## Build from source

You need:

- **Rust**
- **Cargo**

```bash
git clone https://github.com/Eatgrapes/Mocari.git
cd Mocari
cargo build
```

## Renderer backends

By default **Mocari** builds **without** a renderer: it gives you parsing, the
parameter/motion runtime, and a backend-agnostic `render::common` layer
(vertices, draw-order sorting, clipping/mask layout) so you can drive any
graphics API yourself.

A built-in `wgpu` backend is available behind a feature:

```toml
[dependencies]
mocari = { version = "0.1", features = ["wgpu"] }
```

To write your own backend, build on the
types in `mocari::render::common` and feed them `Moc3DrawableMesh` data from the
runtime.

## Status

This project currently implements:

- [x] Reading `.moc3` files
- [x] Rendering models correctly
- [x] Rendering model motions
- [x] Rendering model expressions
- [x] Playing tap-triggered hit-area motions
- [ ] More Cubism runtime features

## Statement

**Mocari** is an unofficial and independent experimental project.
This project is not affiliated with, endorsed by, sponsored by, or certified by Live2D Inc. "Live2D" and "Cubism" are trademarks or registered trademarks of their respective owners.
This repository does not contain Live2D Cubism Core, Live2D SDK binaries, official source code, or any proprietary files distributed by Live2D Inc.
The purpose of this project is to explore a pure Rust runtime and renderer for compatible 2D model data. It is provided for educational, research, and interoperability purposes only.
Users are responsible for ensuring that their use of this project complies with the licenses of Live2D Inc., model creators, asset owners, and applicable laws.
If you are a rights holder and believe that any content in this repository should not be published, please contact the maintainer or open an issue, and the relevant content will be reviewed.
