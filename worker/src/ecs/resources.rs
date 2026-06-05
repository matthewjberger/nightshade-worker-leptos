use nightshade::prelude::{Entity, nalgebra_glm};
use protocol::PickResult;

/// User-facing settings driven by the control panel.
pub struct Controls {
    pub speed: f32,
}

impl Default for Controls {
    fn default() -> Self {
        Self { speed: 1.0 }
    }
}

/// Per-frame orbit and zoom deltas forwarded from the page, applied by the
/// camera system and then cleared.
#[derive(Default)]
pub struct CameraInput {
    pub pending_yaw: f32,
    pub pending_pitch: f32,
    pub pending_zoom: f32,
}

/// Scene playback state.
#[derive(Default)]
pub struct SceneState {
    pub spin: f32,
    pub show_helmet: bool,
    pub helmet_loaded: bool,
    pub helmet_meshes: Vec<Entity>,
    pub helmet_base_rotation: Option<nalgebra_glm::Quat>,
}

/// GPU pick request state. `message` carries a result ready to post back to the
/// page (`Some(None)` is a miss).
#[derive(Default)]
pub struct PickState {
    pub pending: bool,
    pub message: Option<Option<PickResult>>,
}
