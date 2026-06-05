use crate::ecs::{PICK_MARKER, SUBJECT, ShowcaseWorld, SubjectKind};
use nightshade::prelude::Entity;

/// Engine entity of the subject of the given kind, if it exists.
pub fn subject_engine(showcase_world: &ShowcaseWorld, kind: SubjectKind) -> Option<Entity> {
    showcase_world.query_entities(SUBJECT).find_map(|entity| {
        if showcase_world.get_subject(entity)?.kind == kind {
            showcase_world.get_engine_entity(entity).map(|link| link.0)
        } else {
            None
        }
    })
}

/// Engine entity of the subject currently on screen.
pub fn active_subject_engine(showcase_world: &ShowcaseWorld, show_helmet: bool) -> Option<Entity> {
    let kind = if show_helmet {
        SubjectKind::Helmet
    } else {
        SubjectKind::Cube
    };
    subject_engine(showcase_world, kind)
}

/// Engine entity of the pick marker.
pub fn marker_engine(showcase_world: &ShowcaseWorld) -> Option<Entity> {
    showcase_world
        .query_entities(PICK_MARKER)
        .find_map(|entity| showcase_world.get_engine_entity(entity).map(|link| link.0))
}
