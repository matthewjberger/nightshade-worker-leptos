mod ecs;
mod state;
mod systems;

use std::cell::RefCell;
use std::rc::Rc;

use nightshade::prelude::*;
use nightshade::render::wgpu::create_wgpu_renderer;
use protocol::{AdapterInfo, CANVAS_KEY, ClientMessage, MESSAGE_KEY, Stats, WorkerMessage};
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent, OffscreenCanvas};

use crate::ecs::PickOutcome;
use crate::state::Showcase;

type AppSlot = Rc<RefCell<Option<App>>>;

struct App {
    world: World,
    renderer: WgpuRenderer,
    showcase: Showcase,
    frames: f64,
}

impl App {
    fn stats(&self) -> Stats {
        Stats {
            frames: self.frames,
            fps: self.world.resources.window.timing.frames_per_second,
        }
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();

    let scope: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();
    let app_slot: AppSlot = Rc::new(RefCell::new(None));

    let handler_scope = scope.clone();
    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        handle_message(&handler_scope, &app_slot, event);
    });
    scope.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();
}

fn handle_message(scope: &DedicatedWorkerGlobalScope, app_slot: &AppSlot, event: MessageEvent) {
    let data = event.data();
    let Ok(payload) = js_sys::Reflect::get(&data, &JsValue::from_str(MESSAGE_KEY)) else {
        return;
    };
    let Ok(message) = serde_wasm_bindgen::from_value::<ClientMessage>(payload) else {
        return;
    };

    match message {
        ClientMessage::Init { width, height } => {
            let Some(canvas) = canvas_from(&data) else {
                return;
            };
            let scope = scope.clone();
            let app_slot = app_slot.clone();
            spawn_local(async move {
                let app = create_app(canvas, width, height).await;
                post(
                    &scope,
                    &WorkerMessage::Ready {
                        info: AdapterInfo {
                            adapter: "nightshade".to_string(),
                            backend: "WebGPU".to_string(),
                        },
                        context: context(),
                    },
                );
                *app_slot.borrow_mut() = Some(app);
                start_render_loop(scope, app_slot);
            });
        }
        ClientMessage::Resize { width, height } => {
            if let Some(app) = app_slot.borrow_mut().as_mut() {
                resize_offscreen(
                    &mut app.world,
                    &mut app.renderer,
                    (width as u32).max(1),
                    (height as u32).max(1),
                );
            }
        }
        ClientMessage::SetSpeed { speed } => {
            if let Some(app) = app_slot.borrow_mut().as_mut() {
                systems::controls::set_speed(&mut app.showcase.showcase_world, speed);
            }
        }
        ClientMessage::SetColor { red, green, blue } => {
            if let Some(app) = app_slot.borrow_mut().as_mut() {
                systems::controls::set_color(&mut app.world, red, green, blue);
            }
        }
        ClientMessage::Orbit { yaw, pitch } => {
            if let Some(app) = app_slot.borrow_mut().as_mut() {
                systems::camera::queue_orbit(&mut app.showcase.showcase_world, yaw, pitch);
            }
        }
        ClientMessage::Zoom { amount } => {
            if let Some(app) = app_slot.borrow_mut().as_mut() {
                systems::camera::queue_zoom(&mut app.showcase.showcase_world, amount);
            }
        }
        ClientMessage::Pick { x, y } => {
            if let Some(app) = app_slot.borrow_mut().as_mut() {
                systems::picking::request(&mut app.showcase.showcase_world, &mut app.world, x, y);
            }
        }
        ClientMessage::SetHelmet { enabled } => {
            if let Some(app) = app_slot.borrow_mut().as_mut() {
                systems::controls::set_helmet(
                    &mut app.showcase.showcase_world,
                    &mut app.world,
                    enabled,
                );
            }
        }
        ClientMessage::StatsRequest { id } => {
            let stats = app_slot
                .borrow()
                .as_ref()
                .map(|app| app.stats())
                .unwrap_or(Stats {
                    frames: 0.0,
                    fps: 0.0,
                });
            post(scope, &WorkerMessage::StatsReply { id, stats });
        }
    }
}

async fn create_app(canvas: OffscreenCanvas, width: f32, height: f32) -> App {
    let physical_width = (width as u32).max(1);
    let physical_height = (height as u32).max(1);

    let surface_target = wgpu::SurfaceTarget::OffscreenCanvas(canvas);
    let mut renderer = create_wgpu_renderer(surface_target, physical_width, physical_height)
        .await
        .expect("failed to create renderer from offscreen canvas");

    let mut world = World::default();
    let mut showcase = Showcase::default();
    initialize_offscreen(
        &mut world,
        &mut showcase,
        &mut renderer,
        (physical_width, physical_height),
        1.0,
    );

    App {
        world,
        renderer,
        showcase,
        frames: 0.0,
    }
}

fn start_render_loop(scope: DedicatedWorkerGlobalScope, app_slot: AppSlot) {
    let last_push = Rc::new(RefCell::new(0.0_f64));

    spawn_animation_frame_loop(move || {
        if let Some(app) = app_slot.borrow_mut().as_mut() {
            tick_offscreen(&mut app.world, &mut app.showcase, &mut app.renderer);
            app.frames += 1.0;
            if let Some(outcome) = app.showcase.showcase_world.resources.picking.result.take() {
                let hit = match outcome {
                    PickOutcome::Hit(result) => Some(result),
                    PickOutcome::Miss => None,
                };
                post(&scope, &WorkerMessage::Picked { hit });
            }
            if let Some(performance) = scope.performance() {
                let now = performance.now();
                let mut last = last_push.borrow_mut();
                if now - *last > 250.0 {
                    *last = now;
                    post(&scope, &WorkerMessage::Stats { stats: app.stats() });
                }
            }
        }
    });
}

fn canvas_from(data: &JsValue) -> Option<OffscreenCanvas> {
    js_sys::Reflect::get(data, &JsValue::from_str(CANVAS_KEY))
        .ok()
        .and_then(|value| value.dyn_into::<OffscreenCanvas>().ok())
}

fn post(scope: &DedicatedWorkerGlobalScope, message: &WorkerMessage) {
    if let Ok(value) = serde_wasm_bindgen::to_value(message) {
        let _ = scope.post_message(&value);
    }
}

fn context() -> String {
    let global = js_sys::global();
    js_sys::Reflect::get(&global, &JsValue::from_str("constructor"))
        .ok()
        .and_then(|constructor| js_sys::Reflect::get(&constructor, &JsValue::from_str("name")).ok())
        .and_then(|name| name.as_string())
        .unwrap_or_else(|| "unknown".to_string())
}
