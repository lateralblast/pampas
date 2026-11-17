// A release build on Windows is a window app, not a console app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod logic;

use logic::{
    AskEvent, AskRequest, AskResponse, ExportFormat, ExportReport, Locations, ProviderInfo, SettingInfo, UsageInfo,
    export_file_name, keychain, render_export,
};
use serde_json::Value;
use tauri::Emitter;
use tauri_plugin_dialog::DialogExt;

/// The event the front end listens to for the progress of a run.
const ASK_EVENT: &str = "pampas-ask";

/// Errors reach the front end as text.
type CmdResult<T> = Result<T, String>;

fn text(e: anyhow::Error) -> String {
    format!("{e:#}")
}

fn locate() -> CmdResult<Locations> {
    Locations::locate().map_err(text)
}

/// Run `work` on a thread meant for blocking, so a slow keychain (a macOS access prompt waits for the
/// user) or a slow disk never holds up the async runtime, and with it every other command.
async fn blocking<T, F>(work: F) -> CmdResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> CmdResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn locations() -> CmdResult<Locations> {
    locate()
}

#[tauri::command]
async fn list_providers() -> CmdResult<Vec<ProviderInfo>> {
    blocking(move || logic::list_providers(&locate()?, &keychain()).map_err(text)).await
}

#[tauri::command]
async fn get_config() -> CmdResult<Vec<Value>> {
    blocking(move || logic::get_config(&locate()?).map_err(text)).await
}

#[tauri::command]
async fn example_config() -> CmdResult<Vec<Value>> {
    blocking(move || logic::example_config().map_err(text)).await
}

#[tauri::command]
async fn validate_config(entries: Vec<Value>) -> CmdResult<()> {
    blocking(move || logic::validate_config(&entries).map_err(text)).await
}

#[tauri::command]
async fn save_config(entries: Vec<Value>) -> CmdResult<()> {
    blocking(move || logic::save_config(&locate()?, &entries).map_err(text)).await
}

#[tauri::command]
async fn create_config() -> CmdResult<()> {
    blocking(move || logic::create_config(&locate()?).map_err(text)).await
}

#[tauri::command]
async fn set_key(name: String, value: String) -> CmdResult<()> {
    blocking(move || logic::set_key(&keychain(), &name, &value).map_err(text)).await
}

#[tauri::command]
async fn delete_key(name: String) -> CmdResult<bool> {
    blocking(move || logic::delete_key(&keychain(), &name).map_err(text)).await
}

#[tauri::command]
async fn list_settings() -> CmdResult<Vec<SettingInfo>> {
    blocking(move || logic::list_settings(&locate()?).map_err(text)).await
}

#[tauri::command]
async fn set_setting(name: String, value: String) -> CmdResult<()> {
    blocking(move || logic::set_setting(&locate()?, &name, &value).map_err(text)).await
}

#[tauri::command]
async fn delete_setting(name: String) -> CmdResult<bool> {
    blocking(move || logic::delete_setting(&locate()?, &name).map_err(text)).await
}

#[tauri::command]
async fn usage() -> CmdResult<Vec<UsageInfo>> {
    blocking(move || logic::usage(&locate()?).map_err(text)).await
}

#[tauri::command]
async fn list_history() -> CmdResult<Vec<pampas::history::Summary>> {
    blocking(move || logic::list_history(&locate()?).map_err(text)).await
}

#[tauri::command]
async fn get_history(id: String) -> CmdResult<Value> {
    blocking(move || logic::get_history(&locate()?, &id).map_err(text)).await
}

#[tauri::command]
async fn save_history(session: Value) -> CmdResult<()> {
    blocking(move || logic::save_history(&locate()?, session).map_err(text)).await
}

#[tauri::command]
async fn delete_history(id: String) -> CmdResult<bool> {
    blocking(move || logic::delete_history(&locate()?, &id).map_err(text)).await
}

#[tauri::command]
async fn clear_history() -> CmdResult<usize> {
    blocking(move || logic::clear_history(&locate()?).map_err(text)).await
}

#[tauri::command]
async fn import_history(sessions: Vec<Value>) -> CmdResult<usize> {
    blocking(move || logic::import_history(&locate()?, sessions).map_err(text)).await
}

/// The edition as text in one of the command line's formats, for copying.
#[tauri::command]
async fn render_edition(report: ExportReport, format: ExportFormat) -> String {
    render_export(report, format)
}

/// Ask where to save, then write the edition there. The page never supplies a path: the system's
/// save dialog is opened from here and only what the user chose is written. Returns the path, or
/// nothing if the dialog was cancelled.
#[tauri::command]
async fn save_edition(app: tauri::AppHandle, report: ExportReport, format: ExportFormat) -> CmdResult<Option<String>> {
    let name = export_file_name(&report.prompt, format);
    let content = render_export(report, format);
    tauri::async_runtime::spawn_blocking(move || -> CmdResult<Option<String>> {
        let Some(picked) = app
            .dialog()
            .file()
            .set_file_name(name)
            .add_filter(format.label(), &[format.extension()])
            .blocking_save_file()
        else {
            return Ok(None);
        };
        let path = picked.into_path().map_err(|e| e.to_string())?;
        std::fs::write(&path, content).map_err(|e| format!("writing {}: {e}", path.display()))?;
        Ok(Some(path.display().to_string()))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Ask the providers. Progress is emitted as `pampas-ask` events (`started`, then one `answer` per
/// provider as it finishes); the returned value is the complete result.
#[tauri::command]
async fn ask(app: tauri::AppHandle, request: AskRequest) -> CmdResult<AskResponse> {
    let loc = locate()?;
    logic::ask(&loc, &keychain(), request, &|event: AskEvent| {
        let _ = app.emit(ASK_EVENT, event);
    })
    .await
    .map_err(text)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            locations,
            list_providers,
            get_config,
            example_config,
            validate_config,
            save_config,
            create_config,
            set_key,
            delete_key,
            list_settings,
            set_setting,
            delete_setting,
            usage,
            list_history,
            get_history,
            save_history,
            delete_history,
            clear_history,
            import_history,
            render_edition,
            save_edition,
            ask,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the pampas app");
}
