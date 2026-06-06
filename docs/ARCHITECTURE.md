# Architecture

This explains how `nightshade-worker-leptos` is put together: the crate layout, the thread split, the message protocol, the startup handshake, and the per-frame flow. Paths are relative to the repository root.

## What runs where

Every piece of GPU work runs on a web worker, never on the browser's main thread. The worker owns an `OffscreenCanvas` and runs the entire Nightshade engine and render loop. The main thread runs a Leptos UI whose only jobs are to transfer the canvas to the worker once, forward input events, and display stats streamed back from the worker.

The payoff is the "Jam main thread for 3 s" button: it freezes the main thread in a busy-loop, and the cube keeps spinning at full framerate because the render loop lives on another thread entirely.

## Engine in a worker, not a hand-written renderer

The worker runs the whole engine, not a small purpose-built wgpu crate. Running a full engine in a worker usually means fighting its windowing layer. Nightshade is winit-based, but its renderer is decoupled at the right seam, so the worker needs no winit, no `raw-window-handle`, and none of the unsafe window-handle wrapper an engine-in-worker port often carries.

## Workspace layout

| Crate | Path | Runs on | Role |
|---|---|---|---|
| `protocol` | `protocol/` | both | Shared message and data types. The wire contract, including the `postMessage` envelope keys. |
| `worker` | `worker/` | worker | The wasm module inside the web worker. Owns the engine `World`, the `WgpuRenderer`, a `ShowcaseWorld` of game state, and the render loop. |
| root | `src/` | main thread | The Leptos UI, split into components over grouped signal state. |

Nightshade is the published [`nightshade`](https://crates.io/crates/nightshade) crate, built with its default features (`engine`, `wgpu`).

## The thread split

```
MAIN THREAD                                  WEB WORKER
-----------                                  ----------
Leptos wasm app (src/)                       runtime/worker.js (bootstrap shim)
  src/app.rs        shell                      loads engine.js + engine_bg.wasm
  src/components/*  viewport, panel
  src/bridge.rs     postMessage, decode      worker/src/lib.rs    message pump, render loop
  src/state.rs      grouped signals          worker/src/state.rs  Showcase, the State impl
                                             worker/src/ecs.rs    ShowcaseWorld (freecs)
                                             worker/src/systems/* camera, picking, controls

  transfer_control_to_offscreen()  --------->  create_wgpu_renderer(OffscreenCanvas)
  worker.post_message(ClientMessage) ------->  initialize_offscreen, then handle_message
  worker.onmessage(WorkerMessage)  <--------   spawn_animation_frame_loop -> tick_offscreen
```

After the one-time canvas transfer, the main thread can never draw to the canvas again. From that point the two threads communicate only by `postMessage`.

## The engine seam

The worker reaches the engine through `nightshade::run::offscreen`:

- `initialize_offscreen(world, state, renderer, viewport, scale_factor)` sets the cached viewport, installs the default frame and retained-UI schedules, configures the render graph, and runs `State::initialize`.
- `tick_offscreen(world, state, renderer)` advances timing and runs the shared frame body: per-frame engine systems, `State::run_systems`, the frame schedule, and the render graph.
- `resize_offscreen(world, renderer, width, height)` resizes the surface and updates the cached viewport.
- `spawn_animation_frame_loop(on_frame)` drives the worker's render loop, calling `on_frame` once per `requestAnimationFrame`. The worker calls `tick_offscreen` inside it and throttles a stats message back to the page.

`tick_offscreen` and the winit event loop call the same `run_frame_body`, so there is a single frame loop, not two copies. The body renders whenever a viewport size is known, whether or not a winit window handle exists.

Forwarded pointer, wheel, and keyboard events can be fed in with the `input_inject_*` helpers, which write the same input state the winit loop produces. This demo drives the pan-orbit camera's target fields directly, which is the simplest path for coalesced per-frame deltas.

## The message protocol

`protocol/src/lib.rs` defines two enums, the data they carry, and the envelope key constants. The page builds an envelope object keyed by `MESSAGE_KEY` (and `CANVAS_KEY` on `Init`) so the framing has one definition both sides import.

```rust
// page -> worker
enum ClientMessage {
    Init { width, height },     // sent once, with the OffscreenCanvas in the transfer list
    Resize { width, height },
    SetSpeed { speed },
    SetColor { red, green, blue },
    Orbit { yaw, pitch },       // accumulated input deltas
    Zoom { amount },
    Pick { x, y },              // click position in physical pixels
    SetHelmet { enabled },      // swap the helmet model in for the cube
    StatsRequest { id },        // the one request that wants a reply
}

// worker -> page
enum WorkerMessage {
    Ready { info, context },    // engine is up, names the worker scope
    Stats { stats },            // pushed about twice a second
    StatsReply { id, stats },   // answer to StatsRequest
    Picked { hit },             // the picked entity, or none
}
```

One-way streams cover everything that does not need an answer. One request and response round-trip, `StatsRequest` to `StatsReply`, backs the jam measurement.

## Startup handshake

1. Trunk builds the page and copies the prebuilt `runtime/` folder into `dist/`.
2. The Leptos app mounts, and a setup `Effect` in the viewport component fires once the `<canvas>` exists. It sizes the backing store by `device_pixel_ratio`, calls `transfer_control_to_offscreen()`, spawns the worker as an ES module, wires `worker.onmessage`, and sends `Init` with the `OffscreenCanvas` in the transfer list.
3. `runtime/worker.js` buffers any messages that arrive before the wasm installs its real handler, then replays them.
4. On `Init` the worker builds the renderer from the transferred canvas, builds the `World` and `Showcase` state, runs `initialize_offscreen`, posts `Ready`, and starts the `requestAnimationFrame` loop.

## Picking

A click sends `Pick { x, y }` in physical pixels. The worker requests a GPU pick with `gpu_picking.request_pick`, and on a later frame reads the result with `take_result`: the entity id under the cursor and the world position from the depth buffer. It reports the hit entity name and position as `Picked`. The picking resource is reset before each request so a single click is not smoothed against the previous one. This is the same GPU picking path the Nightshade editor uses, not a CPU raycast.

## The jam demonstration

`on_jam` in the control panel reads the worker's frame count over a `StatsRequest`, busy-loops the main thread for three seconds, reads the count again, and reports how many frames the engine advanced during the freeze. The two render loops are independent. The main thread's loop only batches input and ticks the heartbeat, while the worker's loop is what renders.
