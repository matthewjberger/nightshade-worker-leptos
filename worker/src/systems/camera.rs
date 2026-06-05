use crate::ecs::ShowcaseWorld;
use nightshade::prelude::*;

const ORBIT_SENSITIVITY: f32 = 0.005;
const ZOOM_SENSITIVITY: f32 = 0.01;
const PITCH_LIMIT: f32 = 1.5;
const MIN_RADIUS: f32 = 1.5;
const MAX_RADIUS: f32 = 20.0;

/// Accumulates a forwarded orbit delta for the next frame.
pub fn queue_orbit(showcase_world: &mut ShowcaseWorld, yaw: f32, pitch: f32) {
    showcase_world.resources.camera_input.pending_yaw += yaw;
    showcase_world.resources.camera_input.pending_pitch += pitch;
}

/// Accumulates a forwarded zoom delta for the next frame.
pub fn queue_zoom(showcase_world: &mut ShowcaseWorld, amount: f32) {
    showcase_world.resources.camera_input.pending_zoom += amount;
}

/// Applies the forwarded orbit and zoom deltas to the pan-orbit camera, then
/// runs the engine's pan-orbit controller.
pub fn orbit(showcase_world: &mut ShowcaseWorld, world: &mut World) {
    let input = &mut showcase_world.resources.camera_input;
    let yaw = input.pending_yaw;
    let pitch = input.pending_pitch;
    let zoom = input.pending_zoom;
    input.pending_yaw = 0.0;
    input.pending_pitch = 0.0;
    input.pending_zoom = 0.0;

    if let Some(camera) = world.resources.active_camera
        && let Some(orbit) = world.core.get_pan_orbit_camera_mut(camera)
    {
        orbit.target_yaw -= yaw * ORBIT_SENSITIVITY;
        orbit.target_pitch =
            (orbit.target_pitch + pitch * ORBIT_SENSITIVITY).clamp(-PITCH_LIMIT, PITCH_LIMIT);
        orbit.target_radius =
            (orbit.target_radius + zoom * ZOOM_SENSITIVITY).clamp(MIN_RADIUS, MAX_RADIUS);
    }

    pan_orbit_camera_system(world);
}
