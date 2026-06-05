use leptos::prelude::*;
use protocol::ClientMessage;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Event, HtmlInputElement};

use crate::bridge::{Bridge, request_stats, send};
use crate::state::{UiState, WorkerState};

/// The overlay panel: readouts, controls, and the jam test.
#[component]
pub fn ControlPanel(
    bridge: StoredValue<Option<Bridge>, LocalStorage>,
    ui: UiState,
    worker: WorkerState,
) -> impl IntoView {
    let on_speed = move |event: Event| {
        let value = input_value(&event).parse::<f32>().unwrap_or(1.0);
        ui.speed.set(value);
        if let Some(bridge) = bridge.get_value() {
            send(&bridge, &ClientMessage::SetSpeed { speed: value });
        }
    };

    let on_color = move |event: Event| {
        let (red, green, blue) = hex_to_rgb(&input_value(&event));
        if let Some(bridge) = bridge.get_value() {
            send(&bridge, &ClientMessage::SetColor { red, green, blue });
        }
    };

    let on_helmet = move |event: Event| {
        let enabled = event
            .target()
            .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
            .map(|input| input.checked())
            .unwrap_or(false);
        if let Some(bridge) = bridge.get_value() {
            send(&bridge, &ClientMessage::SetHelmet { enabled });
        }
    };

    let on_jam = move |_| {
        let Some(bridge) = bridge.get_value() else {
            return;
        };
        spawn_local(async move {
            let before = request_stats(&bridge).await;
            ui.jam_result.set(String::new());
            ui.jammed.set(true);

            let performance = web_sys::window().unwrap().performance().unwrap();
            let start = performance.now();
            while performance.now() - start < 3000.0 {}
            let blocked = performance.now() - start;

            let after = request_stats(&bridge).await;
            ui.jammed.set(false);
            let advanced = (after.frames - before.frames) as i64;
            ui.jam_result.set(format!(
                "Main thread blocked {} ms. nightshade advanced {} frames meanwhile.",
                blocked.round() as i64,
                format_thousands(advanced)
            ));
        });
    };

    view! {
        <div class="fixed top-4 left-4 w-[340px] px-[18px] py-4 bg-[#0e1018]/80 border border-[#2a2e3f] rounded-[10px] backdrop-blur-sm text-[13px] leading-[1.5]">
            <h1 class="text-[15px] m-0 mb-1.5">"Nightshade, off the main thread"</h1>
            <p class="m-0 mb-3 text-[#9aa0b4]">
                "The full Nightshade engine, GPU pipeline, and render loop run inside a web worker. The page only forwards events: drag to orbit, scroll to zoom, click to mark a spot. Jam the main thread and watch the cube keep spinning."
            </p>

            <div class="mb-3.5 px-2.5 py-2 rounded-md bg-[#14301f] border border-[#1f5c39] text-[#7ee0a3] break-all">
                "engine runs in: "<span>{move || worker.context.get()}</span><br/>
                "renderer: "<span>{move || worker.adapter.get()}</span>
            </div>

            <StatCards ui worker />

            <label class="block mb-3">
                <div class="flex justify-between text-[#9aa0b4] mb-1">
                    <span>"Rotation speed"</span>
                    <span>{move || format!("{:.1}", ui.speed.get())}</span>
                </div>
                <input
                    type="range"
                    min="0"
                    max="5"
                    step="0.1"
                    prop:value=move || ui.speed.get().to_string()
                    class="w-full"
                    on:input=on_speed
                />
            </label>

            <label class="block mb-3">
                <div class="flex justify-between text-[#9aa0b4] mb-1"><span>"Cube color"</span></div>
                <input
                    type="color"
                    value="#4d80e6"
                    class="w-full h-[30px] bg-transparent border border-[#262b3c] rounded-md"
                    on:input=on_color
                />
            </label>

            <label class="flex items-center gap-2 mb-3 cursor-pointer text-[#9aa0b4]">
                <input type="checkbox" on:change=on_helmet />
                <span>"Show damaged helmet"</span>
            </label>

            <button
                class="w-full p-2.5 text-white bg-[#b4452f] hover:bg-[#c84e36] border-0 rounded-md cursor-pointer"
                on:click=on_jam
            >
                "Jam main thread for 3 s"
            </button>
            <div class="mt-2.5 min-h-[34px] text-[#cfd3e0] text-[12px]">{move || ui.jam_result.get()}</div>
            <div class="text-[#9aa0b4] text-[12px]">
                "Picked: "<span class="text-[#cfd3e0]">{move || worker.pick_text.get()}</span>
            </div>
        </div>
    }
}

/// The two stat cards: worker fps/frames and the main-thread heartbeat.
#[component]
fn StatCards(ui: UiState, worker: WorkerState) -> impl IntoView {
    let main_stat_class = move || {
        let base = "px-2.5 py-2 rounded-md border";
        if ui.jammed.get() {
            format!("{base} bg-[#2a1414] border-[#5c1f1f]")
        } else {
            format!("{base} bg-[#161a26] border-[#262b3c]")
        }
    };
    let heartbeat_value_class = move || {
        if ui.jammed.get() {
            "text-[20px] tabular-nums text-[#ff8a8a]"
        } else {
            "text-[20px] tabular-nums"
        }
    };

    view! {
        <div class="grid grid-cols-2 gap-2 mb-3.5">
            <div class="px-2.5 py-2 rounded-md bg-[#161a26] border border-[#262b3c]">
                <div class="text-[#8a90a6] text-[11px] uppercase tracking-[0.04em]">"Worker (nightshade)"</div>
                <div class="text-[20px] tabular-nums">
                    <span>{move || format!("{:.0}", worker.fps.get())}</span>" fps"
                </div>
                <div class="text-[#8a90a6] text-[11px] uppercase tracking-[0.04em]">
                    <span>{move || format_thousands(worker.frames.get() as i64)}</span>" frames"
                </div>
            </div>
            <div class=main_stat_class>
                <div class="text-[#8a90a6] text-[11px] uppercase tracking-[0.04em]">"Main thread"</div>
                <div class=heartbeat_value_class>{move || ui.heartbeat.get().to_string()}</div>
                <div class="text-[#8a90a6] text-[11px] uppercase tracking-[0.04em]">"heartbeat ticks"</div>
            </div>
        </div>
    }
}

fn input_value(event: &Event) -> String {
    event
        .target()
        .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
        .map(|input| input.value())
        .unwrap_or_default()
}

fn hex_to_rgb(hex: &str) -> (f32, f32, f32) {
    let parse = |range: std::ops::Range<usize>| {
        hex.get(range)
            .and_then(|slice| u8::from_str_radix(slice, 16).ok())
            .map(|value| value as f32 / 255.0)
            .unwrap_or(0.0)
    };
    (parse(1..3), parse(3..5), parse(5..7))
}

fn format_thousands(value: i64) -> String {
    let negative = value < 0;
    let digits = value.unsigned_abs().to_string();
    let bytes = digits.as_bytes();
    let mut grouped = String::new();
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 && (bytes.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(*byte as char);
    }
    if negative {
        format!("-{grouped}")
    } else {
        grouped
    }
}
