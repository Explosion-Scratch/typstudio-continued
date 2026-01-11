use crate::ipc::TypstCompileEvent;
use serde::Serialize;
use tauri::{Runtime, WebviewWindow, Emitter};

#[derive(Serialize, Clone, Debug)]
#[serde(tag = "event", content = "payload")]
pub enum BackendEvent {
    #[serde(rename = "typst_compile")]
    Compile(TypstCompileEvent),
}

pub fn emit_event<R: Runtime>(window: &WebviewWindow<R>, event: BackendEvent) {
    let _ = match &event {
        BackendEvent::Compile(payload) => window.emit("typst_compile", payload),
    };
    // Also emit a generic "backend_event" for single-listener setups if needed
    let _ = window.emit("backend_event", event);
}
