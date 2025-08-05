// This attribute hides the console window on Windows when not in debug mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use tauri::Manager;
use walkdir::WalkDir;
use rusqlite::{Connection, params};
use serde::{Serialize, Deserialize};
use std::process::Command;

// Shared state across the application, 
// wrapping SQLite connection in a Mutex
struct AppState {
    db: Mutex<Connection>,
}

// Struct to represent search results returned to the frontend
#[derive(Serialize, Deserialize)]
struct SearchResult {
    path: String,
    name: String,
    r#type: String,        // file, folder, or app
    score: Option<i64>,    // future use for ranking
}

// Initializes the database and sets up tables for files and cache
async fn initialize_database(app: tauri::AppHandle) -> Result<(), String> {
    let app_dir = app.path().app_data_dir()
        .map_err(|_| "Could not get app directory".to_string())?;
    std::fs::create_dir_all(&app_dir).map_err(|e| e.to_string())?;
    
    let db_path = app_dir.join("speedy_index.db");
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    // Create tables if they don't already exist
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS files (
            id INTEGER PRIMARY KEY,
            path TEXT UNIQUE,
            name TEXT,
            is_file BOOLEAN,
            is_app BOOLEAN,
            last_accessed INTEGER,
            access_count INTEGER DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS search_cache (
            query TEXT PRIMARY KEY,
            results TEXT,
            timestamp INTEGER
        );"
    ).map_err(|e| e.to_string())?;
    
    // Store the database connection in the global app state
    app.manage(AppState { db: Mutex::new(conn) });
    Ok(())
}

// Command: Show or hide the search window
#[tauri::command]
async fn toggle_window(visible: bool, app: tauri::AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main")
        .ok_or("Window not found".to_string())?;

    if visible {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    } else {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

// Command: Index files and folders from a given path -> used at startup
#[tauri::command]
async fn index_files(path: String, app: tauri::AppHandle) -> Result<usize, String> {
    let state = app.state::<AppState>();
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let mut count = 0;

    // Walk through the directory tree with max depth of 5
    for entry in WalkDir::new(path).max_depth(5).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path().to_string_lossy().into_owned();
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_file = entry.file_type().is_file();
        let is_app = is_file && entry.path().extension().map_or(false, |ext| ext == "exe");

        // Insert or update file info in the database
        tx.execute(
            "INSERT OR REPLACE INTO files (path, name, is_file, is_app, last_accessed)
             VALUES (?1, ?2, ?3, ?4, strftime('%s','now'))",
            params![path, name, is_file, is_app],
        ).map_err(|e| e.to_string())?;

        count += 1;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}

// Command: Search for files or apps from the database based on a query
#[tauri::command]
async fn search(query: String, app: tauri::AppHandle) -> Result<Vec<SearchResult>, String> {
    let state = app.state::<AppState>();
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    // Try to retrieve from cache first -> only if result is < 5 mins old
    if let Ok(cached) = conn.query_row(
        "SELECT results FROM search_cache 
         WHERE query = ?1 
         AND timestamp > strftime('%s','now','-5 minutes')",
        params![query],
        |row| {
            let results: String = row.get(0)?;
            Ok(serde_json::from_str::<Vec<SearchResult>>(&results).unwrap_or_default())
        },
    ) {
        if !cached.is_empty() {
            return Ok(cached);
        }
    }

    // If not cached, query fresh results from the files table
    let mut stmt = conn.prepare(
        "SELECT path, name, is_file, is_app 
         FROM files 
         WHERE name LIKE ?1 
         ORDER BY last_accessed DESC, access_count DESC
         LIMIT 20"
    ).map_err(|e| e.to_string())?;

    let results = stmt
        .query_map(params![format!("%{}%", query)], |row| {
            Ok(SearchResult {
                path: row.get(0)?,
                name: row.get(1)?,
                r#type: if row.get(3)? { "app".into() } 
                       else if row.get(2)? { "file".into() } 
                       else { "folder".into() },
                score: None,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Cache the results for quick future access
    if !results.is_empty() {
        conn.execute(
            "INSERT OR REPLACE INTO search_cache (query, results, timestamp)
             VALUES (?1, ?2, strftime('%s','now'))",
            params![query, serde_json::to_string(&results).map_err(|e| e.to_string())?],
        ).map_err(|e| e.to_string())?;
    }

    Ok(results)
}

// Command: Open a file/folder/app using system shell
#[tauri::command]
async fn open_path(path: String, app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(windows)]
    {
        // Update usage stats in DB for ranking purposes
        let state = app.state::<AppState>();
        let conn = state.db.lock().map_err(|e| e.to_string())?;

        conn.execute(
            "UPDATE files 
             SET access_count = access_count + 1, 
                 last_accessed = strftime('%s','now') 
             WHERE path = ?1",
            params![path],
        ).map_err(|e| e.to_string())?;

        // Launch using Windows shell (cmd)
        Command::new("cmd")
            .args(["/C", "start", "", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// Entry point of the application
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init()) // Allow shell commands
        .setup(|app| {
            // Optional: show or hide the window on launch
            let window = app.get_webview_window("main")
                .ok_or("Failed to get window".to_string())?;

            // window.hide().map_err(|e| e.to_string())?; 
            window.show().map_err(|e| e.to_string())?;
            window.set_focus().map_err(|e| e.to_string())?;

            // Initialize database
            tauri::async_runtime::block_on(initialize_database(app.handle().clone()))?;

            // Start indexing some common folders in background
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let _ = index_files("C:\\Users".to_string(), app_handle.clone()).await;
                let _ = index_files("C:\\Program Files".to_string(), app_handle).await;
            });

            Ok(())
        })
        // Register all commands to be callable from frontend
        .invoke_handler(tauri::generate_handler![
            toggle_window,
            search,
            index_files,
            open_path
        ])
        // Start Tauri app
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
