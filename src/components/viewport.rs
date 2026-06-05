use std::cell::RefCell;
use std::rc::Rc;

use leptos::html;
use leptos::prelude::*;
use protocol::ClientMessage;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, PointerEvent, ResizeObserver, WheelEvent, Window};

use crate::bridge::{Bridge, connect, send};
use crate::state::{UiState, WorkerState};

/// Raw pointer and wheel input accumulated between frames. Drained once per
/// animation frame into a single orbit and zoom message.
#[derive(Clone, Copy, Default)]
struct DragState {
    pending_yaw: f32,
    pending_pitch: f32,
    pending_zoom: f32,
    dragging: bool,
    moved: f32,
    last_x: f32,
    last_y: f32,
}

/// The render surface: transfers the canvas to the worker, forwards pointer and
/// wheel input, and ticks a main-thread heartbeat.
#[component]
pub fn Viewport(
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
    ui: UiState,
    worker: WorkerState,
) -> impl IntoView {
    let canvas_ref = NodeRef::<html::Canvas>::new();
    let drag = StoredValue::new(DragState::default());

    Effect::new(move |_| {
        let Some(canvas) = canvas_ref.get() else {
            return;
        };
        if bridge.with_value(Option::is_some) {
            return;
        }

        let window = web_sys::window().unwrap();
        let dpr = window.device_pixel_ratio() as f32;
        let rect = canvas.get_bounding_client_rect();
        let width = rect.width() as f32 * dpr;
        let height = rect.height() as f32 * dpr;
        canvas.set_width(width as u32);
        canvas.set_height(height as u32);

        let offscreen = canvas
            .transfer_control_to_offscreen()
            .expect("failed to transfer canvas to offscreen");
        let connected = connect(offscreen, width, height, worker);

        spawn_input_loop(window, drag, connected.clone(), ui);
        observe_resize(canvas, connected.clone());
        bridge.set_value(Some(connected));
    });

    let on_pointerdown = move |event: PointerEvent| {
        drag.update_value(|state| {
            state.dragging = true;
            state.moved = 0.0;
            state.last_x = event.client_x() as f32;
            state.last_y = event.client_y() as f32;
        });
        if let Some(canvas) = canvas_ref.get() {
            let _ = canvas.set_pointer_capture(event.pointer_id());
        }
        ui.grabbing.set(true);
    };

    let on_pointermove = move |event: PointerEvent| {
        drag.update_value(|state| {
            if !state.dragging {
                return;
            }
            let x = event.client_x() as f32;
            let y = event.client_y() as f32;
            let delta_x = x - state.last_x;
            let delta_y = y - state.last_y;
            state.last_x = x;
            state.last_y = y;
            state.moved += delta_x.abs() + delta_y.abs();
            state.pending_yaw += delta_x;
            state.pending_pitch += delta_y;
        });
    };

    let on_pointercancel = move |event: PointerEvent| {
        drag.update_value(|state| state.dragging = false);
        ui.grabbing.set(false);
        if let Some(canvas) = canvas_ref.get() {
            let _ = canvas.release_pointer_capture(event.pointer_id());
        }
    };

    let on_pointerup = move |event: PointerEvent| {
        let was_click = drag.with_value(|state| state.dragging && state.moved < 4.0);
        drag.update_value(|state| state.dragging = false);
        ui.grabbing.set(false);
        if let Some(canvas) = canvas_ref.get() {
            let _ = canvas.release_pointer_capture(event.pointer_id());
            if was_click && let Some(bridge) = bridge.get_value() {
                let dpr = web_sys::window().unwrap().device_pixel_ratio();
                let rect = canvas.get_bounding_client_rect();
                let pixel_x = (event.client_x() as f64 - rect.left()) * dpr;
                let pixel_y = (event.client_y() as f64 - rect.top()) * dpr;
                send(
                    &bridge,
                    &ClientMessage::Pick {
                        x: pixel_x as f32,
                        y: pixel_y as f32,
                    },
                );
            }
        }
    };

    let on_wheel = move |event: WheelEvent| {
        event.prevent_default();
        drag.update_value(|state| state.pending_zoom += event.delta_y() as f32);
    };

    let canvas_class = move || {
        let cursor = if ui.grabbing.get() {
            "cursor-grabbing"
        } else {
            "cursor-grab"
        };
        format!("block w-full h-full touch-none {cursor}")
    };

    view! {
        <div class="fixed inset-0">
            <canvas
                id="canvas"
                node_ref=canvas_ref
                class=canvas_class
                on:pointerdown=on_pointerdown
                on:pointermove=on_pointermove
                on:pointerup=on_pointerup
                on:pointercancel=on_pointercancel
                on:wheel=on_wheel
            ></canvas>
        </div>
    }
}

/// Self-rescheduling `requestAnimationFrame` loop: ticks the heartbeat and
/// forwards at most one orbit and one zoom message per frame. The
/// `Rc<RefCell<…>>` here is the closure-self-reference idiom that a rAF loop
/// requires, confined to this helper and never used for app data.
fn spawn_input_loop(window: Window, drag: StoredValue<DragState>, bridge: Bridge, ui: UiState) {
    type FrameLoop = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;
    let frame: FrameLoop = Rc::new(RefCell::new(None));
    let frame_handle = frame.clone();
    let loop_window = window.clone();

    *frame.borrow_mut() = Some(Closure::<dyn FnMut()>::new(move || {
        ui.heartbeat.update(|value| *value += 1);
        drag.update_value(|state| {
            if state.pending_yaw != 0.0 || state.pending_pitch != 0.0 {
                send(
                    &bridge,
                    &ClientMessage::Orbit {
                        yaw: state.pending_yaw,
                        pitch: state.pending_pitch,
                    },
                );
                state.pending_yaw = 0.0;
                state.pending_pitch = 0.0;
            }
            if state.pending_zoom != 0.0 {
                send(
                    &bridge,
                    &ClientMessage::Zoom {
                        amount: state.pending_zoom,
                    },
                );
                state.pending_zoom = 0.0;
            }
        });
        if let Some(callback) = frame_handle.borrow().as_ref() {
            let _ = loop_window.request_animation_frame(callback.as_ref().unchecked_ref());
        }
    }));

    if let Some(callback) = frame.borrow().as_ref() {
        let _ = window.request_animation_frame(callback.as_ref().unchecked_ref());
    }
}

/// Sends a DPR-scaled `Resize` whenever the canvas layout changes.
fn observe_resize(canvas: HtmlCanvasElement, bridge: Bridge) {
    let resize_window = web_sys::window().unwrap();
    let resize_canvas = canvas.clone();
    let on_resize = Closure::<dyn FnMut()>::new(move || {
        let dpr = resize_window.device_pixel_ratio() as f32;
        let rect = resize_canvas.get_bounding_client_rect();
        send(
            &bridge,
            &ClientMessage::Resize {
                width: rect.width() as f32 * dpr,
                height: rect.height() as f32 * dpr,
            },
        );
    });
    let observer = ResizeObserver::new(on_resize.as_ref().unchecked_ref())
        .expect("failed to create resize observer");
    observer.observe(&canvas);
    on_resize.forget();
    std::mem::forget(observer);
}
