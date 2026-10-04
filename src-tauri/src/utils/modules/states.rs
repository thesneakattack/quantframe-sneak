use std::sync::Mutex;

use std::sync::Arc;

use crate::{
    app::{AppState, Settings},
    cache::client::CacheState,
    live_scraper::client::LiveScraperState,
    wf_inventory::WFInventoryState,
    APP, APP_ERROR,
};
use tauri::Manager;
use utils::Error;

pub fn app_state() -> Result<AppState, Error> {
    let app = APP.get().expect("APP not initialized");
    let state = app.state::<Mutex<AppState>>();
    let guard = state.lock()?;
    Ok(guard.clone())
}

pub fn get_settings() -> Result<Settings, Error> {
    let app = app_state()?;
    let settings = app.settings.clone();
    Ok(settings)
}

pub fn cache_client() -> Result<CacheState, Error> {
    let app = APP.get().expect("APP not initialized");
    let state = app.state::<Mutex<CacheState>>();
    let guard = state.lock()?;
    Ok(guard.clone())
}
/// The inventory state, for callers outside a Tauri command.
pub fn wf_inventory() -> Result<Arc<WFInventoryState>, Error> {
    let app = APP.get().expect("APP not initialized");
    let state = app.state::<Mutex<Arc<WFInventoryState>>>();
    let guard = state.lock()?;
    Ok(guard.clone())
}

/// Whether the live scraper is currently working.
pub fn live_scraper_is_running() -> bool {
    let Some(app) = APP.get() else {
        return false;
    };
    app.try_state::<LiveScraperState>()
        .map(|state| state.is_running())
        .unwrap_or(false)
}

pub fn get_app_error() -> Option<Error> {
    let app_error = APP_ERROR.get_or_init(|| Mutex::new(None));
    let guard = app_error.lock().expect("Failed to lock APP_ERROR");
    guard.clone()
}
pub fn set_app_error(error: Option<Error>) {
    let app_error = APP_ERROR.get_or_init(|| Mutex::new(None));
    let mut guard = app_error.lock().expect("Failed to lock APP_ERROR");
    *guard = error;
}
