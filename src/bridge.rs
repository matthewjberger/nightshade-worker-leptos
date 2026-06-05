use leptos::prelude::*;
use protocol::{CANVAS_KEY, ClientMessage, MESSAGE_KEY, Stats, WorkerMessage};
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{MessageEvent, OffscreenCanvas, Worker, WorkerOptions, WorkerType};

use crate::state::WorkerState;

type StatsSlot = StoredValue<Option<js_sys::Function>, LocalStorage>;

/// The page side of the worker conversation: the worker handle and the slot
/// holding a pending stats resolver. Data only; the behavior lives in the free
/// functions below.
#[derive(Clone)]
pub struct Bridge {
    worker: Worker,
    pending_stats: StatsSlot,
}

/// Spawns the worker as an ES module, wires its `onmessage` handler to the
/// readout signals, sends `Init` with the transferred canvas, and returns the
/// bridge used to talk to it.
pub fn connect(offscreen: OffscreenCanvas, width: f32, height: f32, state: WorkerState) -> Bridge {
    let options = WorkerOptions::new();
    options.set_type(WorkerType::Module);
    let worker =
        Worker::new_with_options("runtime/worker.js", &options).expect("failed to spawn worker");

    let pending_stats: StatsSlot = StoredValue::new_local(None);
    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        let Ok(message) = serde_wasm_bindgen::from_value::<WorkerMessage>(event.data()) else {
            return;
        };
        match message {
            WorkerMessage::Ready { info, context } => {
                state
                    .adapter
                    .set(format!("{} ({})", info.adapter, info.backend));
                state
                    .context
                    .set(format!("{context} (off the main thread)"));
            }
            WorkerMessage::Stats { stats } => {
                state.fps.set(stats.fps);
                state.frames.set(stats.frames);
            }
            WorkerMessage::StatsReply { id: _, stats } => {
                let resolver = pending_stats.get_value();
                pending_stats.set_value(None);
                if let Some(resolve) = resolver
                    && let Ok(value) = serde_wasm_bindgen::to_value(&stats)
                {
                    let _ = resolve.call1(&JsValue::NULL, &value);
                }
            }
            WorkerMessage::Picked { hit } => {
                state.pick_text.set(match hit {
                    Some(hit) => {
                        format!("{} at ({:.2}, {:.2}, {:.2})", hit.name, hit.x, hit.y, hit.z)
                    }
                    None => "no hit".to_string(),
                });
            }
        }
    });
    worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    let bridge = Bridge {
        worker,
        pending_stats,
    };
    send_init(&bridge, offscreen, width, height);
    bridge
}

/// Forwards a message to the worker inside the `{ message }` envelope.
pub fn send(bridge: &Bridge, message: &ClientMessage) {
    let envelope = js_sys::Object::new();
    let value = serde_wasm_bindgen::to_value(message).unwrap_or(JsValue::NULL);
    let _ = js_sys::Reflect::set(&envelope, &JsValue::from_str(MESSAGE_KEY), &value);
    let _ = bridge.worker.post_message(&envelope);
}

/// Sends `Init` with the `OffscreenCanvas` in the transfer list (zero-copy
/// ownership handoff, not a clone).
fn send_init(bridge: &Bridge, canvas: OffscreenCanvas, width: f32, height: f32) {
    let envelope = js_sys::Object::new();
    let value = serde_wasm_bindgen::to_value(&ClientMessage::Init { width, height })
        .unwrap_or(JsValue::NULL);
    let _ = js_sys::Reflect::set(&envelope, &JsValue::from_str(MESSAGE_KEY), &value);
    let _ = js_sys::Reflect::set(&envelope, &JsValue::from_str(CANVAS_KEY), &canvas);
    let transfer = js_sys::Array::of1(&canvas);
    let _ = bridge
        .worker
        .post_message_with_transfer(&envelope, &transfer);
}

/// Sends a stats request and awaits the matching reply.
pub async fn request_stats(bridge: &Bridge) -> Stats {
    let pending_stats = bridge.pending_stats;
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        pending_stats.set_value(Some(resolve));
    });
    send(bridge, &ClientMessage::StatsRequest { id: 0 });
    let value = JsFuture::from(promise).await.unwrap_or(JsValue::NULL);
    serde_wasm_bindgen::from_value(value).unwrap_or(Stats {
        frames: 0.0,
        fps: 0.0,
    })
}
