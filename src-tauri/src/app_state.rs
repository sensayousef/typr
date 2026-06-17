use std::path::PathBuf;
use std::sync::Mutex;

use typr_lib::recording::Recorder;
use typr_lib::settings::Settings;

use crate::history::History;

pub struct AppState {
    pub recorder: Recorder,
    pub settings: Mutex<Settings>,
    pub app_dir: PathBuf,
    pub history: Mutex<History>,
}
