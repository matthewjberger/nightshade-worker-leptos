mod components;
mod resources;

pub use components::*;
pub use resources::*;

use nightshade::prelude::{EngineEntity, Entity, freecs};

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

/// Engine entity of the subject of the given kind, if it exists.
pub fn subject_engine(world: &ShowcaseWorld, kind: SubjectKind) -> Option<Entity> {
    world
        .query_entities(SUBJECT)
        .collect::<Vec<_>>()
        .into_iter()
        .find_map(|entity| {
            if world.get_subject(entity)?.kind == kind {
                world.get_engine_entity(entity).map(|link| link.0)
            } else {
                None
            }
        })
}

/// Engine entity of the subject currently on screen.
pub fn active_subject_engine(world: &ShowcaseWorld, show_helmet: bool) -> Option<Entity> {
    let kind = if show_helmet {
        SubjectKind::Helmet
    } else {
        SubjectKind::Cube
    };
    subject_engine(world, kind)
}

/// Engine entity of the pick marker.
pub fn marker_engine(world: &ShowcaseWorld) -> Option<Entity> {
    world
        .query_entities(PICK_MARKER)
        .collect::<Vec<_>>()
        .into_iter()
        .find_map(|entity| world.get_engine_entity(entity).map(|link| link.0))
}
