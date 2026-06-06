use std::collections::HashMap;

use parking_lot::Mutex;

use crate::pty::PtySession;
use crate::store::{self, PersistedState};

pub struct AppState {
    pub sessions: Mutex<HashMap<String, PtySession>>,
    pub store: Mutex<PersistedState>,
    /// webview zoom factor (VS Code-style ⌘+/⌘-/⌘0)
    pub zoom: Mutex<f64>,
}

impl AppState {
    pub fn new() -> Self {
        let persisted = store::load();
        let zoom = persisted.zoom;
        Self {
            sessions: Mutex::new(HashMap::new()),
            store: Mutex::new(persisted),
            zoom: Mutex::new(zoom),
        }
    }
}
