# nightshade-worker-leptos

The [Nightshade](https://github.com/matthewjberger/nightshade) engine running inside a web worker via WebAssembly, with a [Leptos](https://leptos.dev) frontend. No graphics code on the main thread: the worker owns an `OffscreenCanvas`, drives the render loop with `requestAnimationFrame`, and renders the whole engine through WebGPU. The Leptos app on the main thread only transfers the canvas and forwards events.

Live demo: https://matthewberger.dev/nightshade-worker-leptos/

This is the engine-in-the-worker variant of [webgpu-worker-leptos](https://github.com/matthewjberger/webgpu-worker-leptos). That project hand-writes a small wgpu renderer in the worker. This one runs all of Nightshade instead, the same way [bevy-worker-leptos](https://github.com/matthewjberger/bevy-worker-leptos) runs Bevy. The page and the worker exchange messages defined once in a shared `protocol` crate, serialized over `postMessage`.

## How it works

The worker never touches winit. Nightshade's renderer is built straight from the transferred canvas with `create_wgpu_renderer`, whose only bound is `Into<wgpu::SurfaceTarget>`, and an `OffscreenCanvas` satisfies it. The frame loop runs through the engine's offscreen driver (`initialize_offscreen`, `tick_offscreen`, `resize_offscreen`), which shares its body with the normal winit loop, so there is no windowing layer in the worker at all.

The page captures pointer drag (orbit) and wheel (zoom), coalesces them to at most one message per frame, and forwards them alongside resize, rotation speed, and color. The worker streams frame counters back to drive the fps readout, and answers an on-demand stats request so the "jam" button can measure exactly how many frames the engine advanced while the main thread was blocked.

## Workspace

- `protocol` holds the message and data types both sides share, so the page and worker can never disagree on the wire format.
- `worker` is the wasm module that runs inside the web worker. It owns the Nightshade `World`, the `WgpuRenderer`, and a `State` that spawns a lit cube and a pan-orbit camera, then drives the engine one frame per `requestAnimationFrame`.
- The root crate is the Leptos app. It renders the control panel, transfers the canvas with `transferControlToOffscreen`, spawns the worker as an ES module, and forwards pointer, wheel, and control events.

The worker wasm and the page wasm are two separate modules built by two toolchains. Trunk builds the page. The worker is built with raw `wasm-bindgen` plus `wasm-opt`, which is why the `justfile` builds it explicitly before invoking Trunk.

## Quickstart

Tooling is pinned in [`mise.toml`](mise.toml). Install [mise](https://mise.jdx.dev) and [just](https://github.com/casey/just), then:

```bash
just init        # fetch the pinned toolchain (node, rust, wasm-bindgen, wasm-opt, trunk)
just run         # build the worker, the stylesheet, and serve at http://127.0.0.1:8080
```

Because the worker compiles the full engine, the first build is large and the worker wasm is multiple megabytes even after `wasm-opt -Oz`. That size is the tradeoff for running a complete engine off the main thread instead of a single shader.

Needs a browser with WebGPU and `OffscreenCanvas`-in-workers support (Chromium 113+, Firefox 141+).

## Credits

The `DamagedHelmet` model (`worker/assets/DamagedHelmet.glb`) is from the [glTF Sample Models](https://github.com/KhronosGroup/glTF-Sample-Models) by theblueturtle_.

## License

Dual-licensed under MIT or Apache-2.0, at your option. The bundled model is under its own license, see Credits.
