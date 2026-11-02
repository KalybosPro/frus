---
slug: /
title: Introduction
sidebar_label: Introduction
---

# frus documentation

frus is a cross-platform UI framework written entirely in Rust. One codebase runs on the
desktops (Windows, Linux, macOS), on Android and in the browser, drawn on the GPU by frus
itself. The renderer, layout, widgets, gestures, theming, animation and accessibility are all
Rust: there is no embedded VM, no second language for the application's logic, and no bridge
between your code and the pixels.

:::warning[Pre-alpha]

The crates are on [crates.io](https://crates.io/crates/frus), and any release may break the
API. The [roadmap](https://github.com/KalybosPro/frus/blob/master/ROADMAP.md) says what is
next, and what is known to be missing.

:::

## Where to begin

| Page | What it covers |
| --- | --- |
| [Starting a frus application](getting-started.md) | Install the `frus` command, make a project, run it on this computer, a phone and the browser |
| [The guide](guide/README.md) | One topic per page — layout, style, input, lists, navigation, network, animation, testing, the system around the window |
| [Structuring an application](app-structure.md) | Where each part of a growing application goes |
| [The `frus` command](tooling/cli.md) | `create`, `doctor`, `devices`, `run`, `build`, hot reload, and the keys while it runs |
| [Architecture](internals/architecture.md) | How the crates fit together, and where a given kind of code belongs |

## Platforms

| Platform | State |
| --- | --- |
| Windows, Linux, macOS | Working: winit and wgpu, screen readers through AccessKit, notifications, the tray, file dialogs, hot reload |
| Android | Working: native activity, Vulkan or OpenGL ES, a real IME, insets, notifications, hot reload |
| Web (wasm and WebGPU) | Functional: rendering, input, `fetch`, the clipboard, notifications, accessibility through a DOM bridge, hot reload |
| iOS, native macOS | Not started: the shell is isolated, so a new target is a contained job |

Every example in the guide is a complete piece of Rust, and frus's own test suite compiles it,
so none of them is out of date.
