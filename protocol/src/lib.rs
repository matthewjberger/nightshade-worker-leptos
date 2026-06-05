use serde::{Deserialize, Serialize};

/// Envelope field carrying the serialized message in every `postMessage`.
pub const MESSAGE_KEY: &str = "message";
/// Envelope field carrying the transferred `OffscreenCanvas` (on `Init` only).
pub const CANVAS_KEY: &str = "canvas";

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Stats {
    pub frames: f64,
    pub fps: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AdapterInfo {
    pub adapter: String,
    pub backend: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PickResult {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Page to worker. All pixel quantities are physical surface pixels (CSS pixels
/// times the device pixel ratio), with the origin at the canvas top-left.
#[derive(Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Sent once with the `OffscreenCanvas` in the transfer list. Initial
    /// surface size in physical pixels.
    Init { width: f32, height: f32 },
    /// New surface size in physical pixels.
    Resize { width: f32, height: f32 },
    /// Rotation speed multiplier.
    SetSpeed { speed: f32 },
    /// Cube color in linear 0..1 RGB.
    SetColor { red: f32, green: f32, blue: f32 },
    /// Accumulated orbit deltas in raw pointer pixels.
    Orbit { yaw: f32, pitch: f32 },
    /// Accumulated wheel delta.
    Zoom { amount: f32 },
    /// Click position in physical surface pixels.
    Pick { x: f32, y: f32 },
    /// Swap the helmet model in for the cube.
    SetHelmet { enabled: bool },
    /// Request a stats reply correlated by `id`.
    StatsRequest { id: u32 },
}

/// Worker to page.
#[derive(Clone, Serialize, Deserialize)]
pub enum WorkerMessage {
    Ready { info: AdapterInfo, context: String },
    Stats { stats: Stats },
    StatsReply { id: u32, stats: Stats },
    Picked { hit: Option<PickResult> },
}
