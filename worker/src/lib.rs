use std::cell::RefCell;
use std::rc::Rc;

use nightshade::prelude::*;
use nightshade::render::wgpu::create_wgpu_renderer;
use protocol::{AdapterInfo, ClientMessage, PickResult, Stats, WorkerMessage};
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent, OffscreenCanvas};

const ORBIT_SENSITIVITY: f32 = 0.005;
const ZOOM_SENSITIVITY: f32 = 0.01;
const PITCH_LIMIT: f32 = 1.5;
const MIN_RADIUS: f32 = 1.5;
const MAX_RADIUS: f32 = 20.0;

const HELMET_GLB: &[u8] = include_bytes!("../assets/DamagedHelmet.glb");

type AppSlot = Rc<RefCell<Option<App>>>;
type FrameLoop = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

struct App {
    world: World,
    renderer: WgpuRenderer,
    game: CubeGame,
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

struct CubeGame {
    cube: Option<Entity>,
    marker: Option<Entity>,
    helmet: Option<Entity>,
    helmet_meshes: Vec<Entity>,
    helmet_base_rotation: Option<nalgebra_glm::Quat>,
    show_helmet: bool,
    camera: Option<Entity>,
    spin: f32,
    speed: f32,
    pending_yaw: f32,
    pending_pitch: f32,
    pending_zoom: f32,
}

impl CubeGame {
    fn new() -> Self {
        Self {
            cube: None,
            marker: None,
            helmet: None,
            helmet_meshes: Vec::new(),
            helmet_base_rotation: None,
            show_helmet: false,
            camera: None,
            spin: 0.0,
            speed: 1.0,
            pending_yaw: 0.0,
            pending_pitch: 0.0,
            pending_zoom: 0.0,
        }
    }

    fn set_helmet_visible(&mut self, world: &mut World, enabled: bool) {
        if enabled && self.helmet.is_none() {
            self.load_helmet(world);
        }
        self.show_helmet = enabled;

        if let Some(cube) = self.cube {
            world
                .core
                .set_visibility(cube, Visibility { visible: !enabled });
        }
        for &entity in &self.helmet_meshes {
            world
                .core
                .set_visibility(entity, Visibility { visible: enabled });
        }

        let root = if enabled { self.helmet } else { self.cube };
        if let (Some(marker), Some(root)) = (self.marker, root) {
            update_parent(world, marker, Some(Parent(Some(root))));
            if let Some(transform) = world.core.get_local_transform_mut(marker) {
                transform.translation = Vec3::new(0.0, 0.0, 0.0);
            }
            mark_local_transform_dirty(world, marker);
        }
    }

    fn load_helmet(&mut self, world: &mut World) {
        let mut result = match import_gltf_from_bytes(HELMET_GLB) {
            Ok(result) => result,
            Err(error) => {
                tracing::error!("failed to import helmet: {error}");
                return;
            }
        };
        nightshade::ecs::loading::queue_gltf_load(world, &mut result);

        let Some(prefab) = result.prefabs.first() else {
            return;
        };
        let root = nightshade::ecs::prefab::spawn_prefab(world, prefab, Vec3::new(0.0, 0.0, 0.0));
        self.helmet_base_rotation = world
            .core
            .get_local_transform(root)
            .map(|transform| transform.rotation);

        let mut entities = nightshade::ecs::transform::queries::query_descendants(world, root);
        entities.push(root);
        for &entity in &entities {
            world.core.add_components(entity, VISIBILITY);
            world
                .core
                .set_visibility(entity, Visibility { visible: false });
        }
        self.helmet = Some(root);
        self.helmet_meshes = entities;
    }
}

impl State for CubeGame {
    fn initialize(&mut self, world: &mut World) {
        world.resources.render_settings.atmosphere = Atmosphere::CloudySky;
        world.resources.render_settings.clear_color = [0.17, 0.17, 0.18, 1.0];
        capture_procedural_atmosphere_ibl(world, Atmosphere::CloudySky, 0.0);
        spawn_sun(world);

        let camera = spawn_pan_orbit_camera(
            world,
            Vec3::new(0.0, 0.0, 0.0),
            3.0,
            0.0,
            0.0,
            "Camera".to_string(),
        );
        world.resources.active_camera = Some(camera);
        self.camera = Some(camera);

        material_registry_insert(
            &mut world.resources.assets.material_registry,
            "cube".to_string(),
            Material {
                base_color: [0.3, 0.5, 0.9, 1.0],
                roughness: 0.5,
                metallic: 0.0,
                ..Default::default()
            },
        );
        if let Some(&index) = world
            .resources
            .assets
            .material_registry
            .registry
            .name_to_index
            .get("cube")
        {
            registry_add_reference(
                &mut world.resources.assets.material_registry.registry,
                index,
            );
        }

        let cube = spawn_mesh(
            world,
            "Cube",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
        );
        world.core.set_material_ref(cube, MaterialRef::new("cube"));
        world.core.add_components(cube, VISIBILITY);
        self.cube = Some(cube);

        let marker = spawn_mesh(
            world,
            "Sphere",
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.07, 0.07, 0.07),
        );
        world.core.add_components(marker, PARENT);
        material_registry_insert(
            &mut world.resources.assets.material_registry,
            "marker".to_string(),
            Material {
                base_color: [1.0, 0.45, 0.1, 1.0],
                emissive_factor: [1.0, 0.45, 0.1],
                emissive_strength: 1.5,
                ..Default::default()
            },
        );
        if let Some(&index) = world
            .resources
            .assets
            .material_registry
            .registry
            .name_to_index
            .get("marker")
        {
            registry_add_reference(
                &mut world.resources.assets.material_registry.registry,
                index,
            );
        }
        world
            .core
            .set_material_ref(marker, MaterialRef::new("marker"));
        update_parent(world, marker, Some(Parent(Some(cube))));
        world.core.add_components(marker, VISIBILITY);
        self.marker = Some(marker);
    }

    fn run_systems(&mut self, world: &mut World) {
        let delta_time = world.resources.window.timing.delta_time;
        self.spin += self.speed * delta_time;

        if self.show_helmet {
            if let (Some(helmet), Some(base)) = (self.helmet, self.helmet_base_rotation) {
                if let Some(transform) = world.core.get_local_transform_mut(helmet) {
                    transform.rotation =
                        nalgebra_glm::quat_angle_axis(self.spin, &Vec3::new(0.0, 1.0, 0.0)) * base;
                }
                mark_local_transform_dirty(world, helmet);
            }
        } else if let Some(cube) = self.cube {
            if let Some(transform) = world.core.get_local_transform_mut(cube) {
                transform.rotation =
                    nalgebra_glm::quat_angle_axis(self.spin, &Vec3::new(0.0, 1.0, 0.0))
                        * nalgebra_glm::quat_angle_axis(self.spin * 0.4, &Vec3::new(1.0, 0.0, 0.0));
            }
            mark_local_transform_dirty(world, cube);
        }

        if let Some(camera) = self.camera
            && let Some(orbit) = world.core.get_pan_orbit_camera_mut(camera)
        {
            orbit.target_yaw -= self.pending_yaw * ORBIT_SENSITIVITY;
            orbit.target_pitch = (orbit.target_pitch + self.pending_pitch * ORBIT_SENSITIVITY)
                .clamp(-PITCH_LIMIT, PITCH_LIMIT);
            orbit.target_radius = (orbit.target_radius + self.pending_zoom * ZOOM_SENSITIVITY)
                .clamp(MIN_RADIUS, MAX_RADIUS);
        }
        self.pending_yaw = 0.0;
        self.pending_pitch = 0.0;
        self.pending_zoom = 0.0;

        pan_orbit_camera_system(world);
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
    let Ok(payload) = js_sys::Reflect::get(&data, &JsValue::from_str("message")) else {
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
                app.game.speed = speed;
            }
        }
        ClientMessage::SetColor { red, green, blue } => {
            if let Some(app) = app_slot.borrow_mut().as_mut() {
                queue_ecs_command(
                    &mut app.world,
                    EcsCommand::ReloadMaterial {
                        name: "cube".to_string(),
                        material: Box::new(Material {
                            base_color: [red, green, blue, 1.0],
                            roughness: 0.5,
                            metallic: 0.0,
                            ..Default::default()
                        }),
                    },
                );
            }
        }
        ClientMessage::Orbit { yaw, pitch } => {
            if let Some(app) = app_slot.borrow_mut().as_mut() {
                app.game.pending_yaw += yaw;
                app.game.pending_pitch += pitch;
            }
        }
        ClientMessage::Zoom { amount } => {
            if let Some(app) = app_slot.borrow_mut().as_mut() {
                app.game.pending_zoom += amount;
            }
        }
        ClientMessage::Pick { x, y } => {
            let hit = app_slot
                .borrow_mut()
                .as_mut()
                .and_then(|app| pick(app, x, y));
            post(scope, &WorkerMessage::Picked { hit });
        }
        ClientMessage::SetHelmet { enabled } => {
            if let Some(app) = app_slot.borrow_mut().as_mut() {
                app.game.set_helmet_visible(&mut app.world, enabled);
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
    let mut game = CubeGame::new();
    initialize_offscreen(
        &mut world,
        &mut game,
        &mut renderer,
        (physical_width, physical_height),
        1.0,
    );

    App {
        world,
        renderer,
        game,
        frames: 0.0,
    }
}

fn start_render_loop(scope: DedicatedWorkerGlobalScope, app_slot: AppSlot) {
    let frame: FrameLoop = Rc::new(RefCell::new(None));
    let frame_handle = frame.clone();
    let loop_scope = scope.clone();
    let last_push = Rc::new(RefCell::new(0.0_f64));

    *frame.borrow_mut() = Some(Closure::<dyn FnMut()>::new(move || {
        if let Some(app) = app_slot.borrow_mut().as_mut() {
            tick_offscreen(&mut app.world, &mut app.game, &mut app.renderer);
            app.frames += 1.0;
            if let Some(performance) = loop_scope.performance() {
                let now = performance.now();
                let mut last = last_push.borrow_mut();
                if now - *last > 250.0 {
                    *last = now;
                    post(&loop_scope, &WorkerMessage::Stats { stats: app.stats() });
                }
            }
        }
        if let Some(callback) = frame_handle.borrow().as_ref() {
            let _ = loop_scope.request_animation_frame(callback.as_ref().unchecked_ref());
        }
    }));

    if let Some(callback) = frame.borrow().as_ref() {
        let _ = scope.request_animation_frame(callback.as_ref().unchecked_ref());
    }
}

fn pick(app: &mut App, x: f32, y: f32) -> Option<PickResult> {
    let results = pick_entities(&app.world, Vec2::new(x, y), PickingOptions::default());

    let (root, result) = if app.game.show_helmet {
        let helmet = app.game.helmet?;
        let result = results
            .into_iter()
            .find(|hit| app.game.helmet_meshes.contains(&hit.entity))?;
        (helmet, result)
    } else {
        let cube = app.game.cube?;
        let result = results.into_iter().find(|hit| hit.entity == cube)?;
        (cube, result)
    };
    let hit = result.world_position;

    if let Some(marker) = app.game.marker {
        let local = app
            .world
            .core
            .get_global_transform(root)
            .and_then(|global| global.0.try_inverse())
            .map(|inverse| {
                let point = inverse * nalgebra_glm::vec4(hit.x, hit.y, hit.z, 1.0);
                Vec3::new(point.x / point.w, point.y / point.w, point.z / point.w)
            })
            .unwrap_or(hit);
        if let Some(transform) = app.world.core.get_local_transform_mut(marker) {
            transform.translation = local;
        }
        mark_local_transform_dirty(&mut app.world, marker);
    }

    let name = app
        .world
        .core
        .get_name(result.entity)
        .map(|name| name.0.clone())
        .unwrap_or_else(|| "entity".to_string());
    Some(PickResult {
        name,
        x: hit.x,
        y: hit.y,
        z: hit.z,
    })
}

fn canvas_from(data: &JsValue) -> Option<OffscreenCanvas> {
    js_sys::Reflect::get(data, &JsValue::from_str("canvas"))
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
