mod components;
mod resources;

pub use components::*;
pub use resources::*;

use nightshade::prelude::{EngineEntity, freecs};

freecs::ecs! {
    ShowcaseWorld {
        subject: Subject => SUBJECT,
        pick_marker: PickMarker => PICK_MARKER,
        engine_entity: EngineEntity => ENGINE_ENTITY,
    }
    Tags {
    }
    Events {
    }
    Resources {
        controls: Controls,
        camera_input: CameraInput,
        scene: SceneState,
        picking: PickState,
    }
}
