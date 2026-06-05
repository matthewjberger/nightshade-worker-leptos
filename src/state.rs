use leptos::prelude::*;

/// Readouts driven by messages streamed back from the worker.
#[derive(Clone, Copy)]
pub struct WorkerState {
    pub context: RwSignal<String>,
    pub adapter: RwSignal<String>,
    pub fps: RwSignal<f32>,
    pub frames: RwSignal<f64>,
    pub pick_text: RwSignal<String>,
}

impl WorkerState {
    pub fn new() -> Self {
        Self {
            context: RwSignal::new("checking…".to_string()),
            adapter: RwSignal::new("…".to_string()),
            fps: RwSignal::new(0.0),
            frames: RwSignal::new(0.0),
            pick_text: RwSignal::new("click the cube to mark a spot".to_string()),
        }
    }
}

impl Default for WorkerState {
    fn default() -> Self {
        Self::new()
    }
}

/// State owned by the page: control values and main-thread liveness.
#[derive(Clone, Copy)]
pub struct UiState {
    pub speed: RwSignal<f32>,
    pub grabbing: RwSignal<bool>,
    pub jammed: RwSignal<bool>,
    pub heartbeat: RwSignal<u64>,
    pub jam_result: RwSignal<String>,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            speed: RwSignal::new(1.0),
            grabbing: RwSignal::new(false),
            jammed: RwSignal::new(false),
            heartbeat: RwSignal::new(0),
            jam_result: RwSignal::new(String::new()),
        }
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}
