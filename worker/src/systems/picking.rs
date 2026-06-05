use crate::ecs::{ShowcaseWorld, active_subject_engine, marker_engine};
use nightshade::prelude::*;
use protocol::PickResult;

/// Requests a GPU pick at a pixel. The picking resource is reset first so the
/// engine returns an unsmoothed, single-click result.
pub fn request(showcase_world: &mut ShowcaseWorld, world: &mut World, x: f32, y: f32) {
    world.resources.gpu_picking = GpuPicking::default();
    world
        .resources
        .gpu_picking
        .request_pick(x.max(0.0) as u32, y.max(0.0) as u32);
    showcase_world.resources.picking.pending = true;
}

/// Polls the pending GPU pick. When the readback lands, moves the marker to the
/// surface hit (in the active subject's local space) and queues a result for
/// the page. Misses (background depth) are reported without moving the marker.
pub fn apply(showcase_world: &mut ShowcaseWorld, world: &mut World) {
    if !showcase_world.resources.picking.pending {
        return;
    }
    let Some(result) = world.resources.gpu_picking.take_result() else {
        return;
    };
    showcase_world.resources.picking.pending = false;

    if result.depth <= 0.0 {
        showcase_world.resources.picking.message = Some(None);
        return;
    }

    let hit = result.world_position;
    let show_helmet = showcase_world.resources.scene.show_helmet;
    let root = active_subject_engine(showcase_world, show_helmet);
    let marker = marker_engine(showcase_world);
    if let (Some(marker), Some(root)) = (marker, root) {
        let local = world
            .core
            .get_global_transform(root)
            .and_then(|global| global.0.try_inverse())
            .map(|inverse| {
                let point = inverse * nalgebra_glm::vec4(hit.x, hit.y, hit.z, 1.0);
                Vec3::new(point.x / point.w, point.y / point.w, point.z / point.w)
            })
            .unwrap_or(hit);
        if let Some(transform) = world.core.get_local_transform_mut(marker) {
            transform.translation = local;
        }
        mark_local_transform_dirty(world, marker);
    }

    let name = if show_helmet { "Helmet" } else { "Cube" };
    showcase_world.resources.picking.message = Some(Some(PickResult {
        name: name.to_string(),
        x: hit.x,
        y: hit.y,
        z: hit.z,
    }));
}
