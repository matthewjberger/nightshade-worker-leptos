use crate::ecs::{ENGINE_ENTITY, SUBJECT, ShowcaseWorld, Subject, SubjectKind};
use crate::systems::query::{marker_engine, subject_engine};
use nightshade::prelude::*;

const HELMET_GLB: &[u8] = include_bytes!("../../assets/DamagedHelmet.glb");

/// Sets the rotation speed.
pub fn set_speed(showcase_world: &mut ShowcaseWorld, speed: f32) {
    showcase_world.resources.controls.speed = speed;
}

/// Recolors the cube by reloading its material on the GPU.
pub fn set_color(world: &mut World, red: f32, green: f32, blue: f32) {
    queue_ecs_command(
        world,
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

/// Toggles between the cube and the helmet. The helmet loads lazily on first
/// enable. Visibility is set per mesh entity, and the marker is re-parented to
/// whichever subject is now on screen.
pub fn set_helmet(showcase_world: &mut ShowcaseWorld, world: &mut World, enabled: bool) {
    if enabled && !showcase_world.resources.scene.helmet_loaded {
        load_helmet(showcase_world, world);
    }
    showcase_world.resources.scene.show_helmet = enabled;

    if let Some(cube) = subject_engine(showcase_world, SubjectKind::Cube) {
        world
            .core
            .set_visibility(cube, Visibility { visible: !enabled });
    }
    for &entity in &showcase_world.resources.scene.helmet_meshes {
        world
            .core
            .set_visibility(entity, Visibility { visible: enabled });
    }

    let root = if enabled {
        subject_engine(showcase_world, SubjectKind::Helmet)
    } else {
        subject_engine(showcase_world, SubjectKind::Cube)
    };
    if let (Some(marker), Some(root)) = (marker_engine(showcase_world), root) {
        update_parent(world, marker, Some(Parent(Some(root))));
        if let Some(transform) = world.core.get_local_transform_mut(marker) {
            transform.translation = Vec3::new(0.0, 0.0, 0.0);
        }
        mark_local_transform_dirty(world, marker);
    }
}

fn load_helmet(showcase_world: &mut ShowcaseWorld, world: &mut World) {
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
    showcase_world.resources.scene.helmet_base_rotation = world
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
    showcase_world.resources.scene.helmet_meshes = entities;
    showcase_world.resources.scene.helmet_loaded = true;

    let subject = showcase_world.spawn_entities(SUBJECT | ENGINE_ENTITY, 1)[0];
    showcase_world.set_subject(
        subject,
        Subject {
            kind: SubjectKind::Helmet,
        },
    );
    showcase_world.set_engine_entity(subject, EngineEntity(root));
}
