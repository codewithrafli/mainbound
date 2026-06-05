use std::collections::HashMap;

use parking_lot::Mutex;

use crate::pty::PtySession;

#[derive(Default)]
pub struct AppState {
    pub sessions: Mutex<HashMap<String, PtySession>>,
}
