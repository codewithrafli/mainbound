use std::collections::HashMap;

use parking_lot::Mutex;

use crate::pty::PtySession;
use crate::store::{self, PersistedState};

pub struct AppState {
    pub sessions: Mutex<HashMap<String, PtySession>>,
    pub store: Mutex<PersistedState>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            store: Mutex::new(store::load()),
        }
    }
}
