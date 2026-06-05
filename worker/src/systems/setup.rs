use crate::ecs::{
    ENGINE_ENTITY, PICK_MARKER, PickMarker, SUBJECT, ShowcaseWorld, Subject, SubjectKind,
};
use nightshade::prelude::*;

/// Builds the scene: lighting, environment, camera, the cube and its pick
/// marker, and the game entities that link them to the engine.
pub fn spawn(showcase_world: &mut ShowcaseWorld, world: &mut World) {
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

    let cube = spawn_cube(world);
    let marker = spawn_marker(world, cube);

    let cube_subject = showcase_world.spawn_entities(SUBJECT | ENGINE_ENTITY, 1)[0];
    showcase_world.set_subject(
        cube_subject,
        Subject {
            kind: SubjectKind::Cube,
        },
    );
    showcase_world.set_engine_entity(cube_subject, EngineEntity(cube));

    let marker_link = showcase_world.spawn_entities(PICK_MARKER | ENGINE_ENTITY, 1)[0];
    showcase_world.set_pick_marker(marker_link, PickMarker);
    showcase_world.set_engine_entity(marker_link, EngineEntity(marker));
}

fn spawn_cube(world: &mut World) -> Entity {
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
    add_material_reference(world, "cube");

    let cube = spawn_mesh(
        world,
        "Cube",
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 1.0),
    );
    world.core.set_material_ref(cube, MaterialRef::new("cube"));
    world.core.add_components(cube, VISIBILITY);
    cube
}

fn spawn_marker(world: &mut World, cube: Entity) -> Entity {
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
    add_material_reference(world, "marker");

    let marker = spawn_mesh(
        world,
        "Sphere",
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.07, 0.07, 0.07),
    );
    world
        .core
        .set_material_ref(marker, MaterialRef::new("marker"));
    update_parent(world, marker, Some(Parent(Some(cube))));
    world.core.add_components(marker, VISIBILITY);
    marker
}

fn add_material_reference(world: &mut World, name: &str) {
    if let Some(&index) = world
        .resources
        .assets
        .material_registry
        .registry
        .name_to_index
        .get(name)
    {
        registry_add_reference(
            &mut world.resources.assets.material_registry.registry,
            index,
        );
    }
}
