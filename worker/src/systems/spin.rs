use crate::ecs::{ShowcaseWorld, active_subject_engine};
use nightshade::prelude::*;

/// Spins the on-screen subject. The cube tumbles on two axes; the helmet turns
/// around Y while keeping its imported orientation.
pub fn tick(showcase_world: &mut ShowcaseWorld, world: &mut World) {
    let delta_time = world.resources.window.timing.delta_time;
    showcase_world.resources.scene.spin += showcase_world.resources.controls.speed * delta_time;

    let spin = showcase_world.resources.scene.spin;
    let show_helmet = showcase_world.resources.scene.show_helmet;
    let Some(engine) = active_subject_engine(showcase_world, show_helmet) else {
        return;
    };

    let rotation = if show_helmet {
        let base = showcase_world
            .resources
            .scene
            .helmet_base_rotation
            .unwrap_or_else(nalgebra_glm::quat_identity);
        nalgebra_glm::quat_angle_axis(spin, &Vec3::new(0.0, 1.0, 0.0)) * base
    } else {
        nalgebra_glm::quat_angle_axis(spin, &Vec3::new(0.0, 1.0, 0.0))
            * nalgebra_glm::quat_angle_axis(spin * 0.4, &Vec3::new(1.0, 0.0, 0.0))
    };

    if let Some(transform) = world.core.get_local_transform_mut(engine) {
        transform.rotation = rotation;
    }
    mark_local_transform_dirty(world, engine);
}
