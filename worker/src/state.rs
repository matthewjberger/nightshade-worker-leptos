use crate::ecs::ShowcaseWorld;
use crate::systems;
use nightshade::prelude::*;

/// The application root. Holds the user-side ECS world (`ShowcaseWorld`) and
/// forwards each `State` hook to system functions in `src/systems/`. Worker
/// message handlers in `lib.rs` call the same systems for one-off actions.
#[derive(Default)]
pub struct Showcase {
    pub showcase_world: ShowcaseWorld,
}

impl State for Showcase {
    fn initialize(&mut self, world: &mut World) {
        systems::setup::spawn(&mut self.showcase_world, world);
    }

    fn run_systems(&mut self, world: &mut World) {
        systems::camera::orbit(&mut self.showcase_world, world);
        systems::spin::tick(&mut self.showcase_world, world);
        systems::picking::apply(&mut self.showcase_world, world);
    }
}
