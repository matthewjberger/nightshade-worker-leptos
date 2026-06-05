use leptos::prelude::*;

use crate::bridge::Bridge;
use crate::components::control_panel::ControlPanel;
use crate::components::github_link::GithubLink;
use crate::components::viewport::Viewport;
use crate::state::{UiState, WorkerState};

/// Application root. Owns the shared state and the bridge slot, and composes the
/// view from the `Viewport`, `ControlPanel`, and `GithubLink` components.
#[component]
pub fn App() -> impl IntoView {
    let worker = WorkerState::new();
    let ui = UiState::new();
    let bridge = StoredValue::new_local(None::<Bridge>);

    view! {
        <Viewport bridge ui worker />
        <ControlPanel bridge ui worker />
        <GithubLink />
    }
}
