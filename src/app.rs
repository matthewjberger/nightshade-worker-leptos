use leptos::prelude::*;
use wasm_bindgen::JsValue;

use crate::bridge::Bridge;
use crate::components::control_panel::ControlPanel;
use crate::components::github_link::GithubLink;
use crate::components::viewport::Viewport;
use crate::state::{UiState, WorkerState};

/// Application root. Owns the shared state and the bridge slot and composes the
/// view from the `Viewport`, `ControlPanel`, and `GithubLink` components. Shows
/// a notice instead when the browser has no WebGPU.
#[component]
pub fn App() -> impl IntoView {
    if !webgpu_supported() {
        return view! {
            <div class="fixed inset-0 flex items-center justify-center p-8 text-center text-[#9aa0b4]">
                <div class="max-w-[420px]">
                    <h1 class="text-[15px] text-[#cfd3e0] mb-2">"WebGPU not available"</h1>
                    <p>
                        "This demo runs the Nightshade engine in a web worker through WebGPU. Open it in a browser with WebGPU and OffscreenCanvas-in-workers support (Chromium 113+, Firefox 141+)."
                    </p>
                </div>
            </div>
        }
        .into_any();
    }

    let worker = WorkerState::new();
    let ui = UiState::new();
    let bridge = StoredValue::new_local(None::<Bridge>);

    view! {
        <Viewport bridge ui worker />
        <ControlPanel bridge ui worker />
        <GithubLink />
    }
    .into_any()
}

/// True when the browser exposes `navigator.gpu`.
fn webgpu_supported() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let Ok(navigator) = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("navigator"))
    else {
        return false;
    };
    js_sys::Reflect::get(&navigator, &JsValue::from_str("gpu"))
        .map(|gpu| !gpu.is_undefined() && !gpu.is_null())
        .unwrap_or(false)
}
