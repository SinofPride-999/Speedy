#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use tauri::Manager;
use walkdir::WalkDir;
use rusqlite::{Connection, params};
use serde::{Serialize, Deserialize};
use std::process::Command;
use std::path::PathBuf;
use std::ffi::OsStr;
use std::env;
// use windows_shortcuts::Shortcut;

struct AppState {
    db: Mutex<Connection>,
}

#[derive(Serialize, Deserialize)]
struct SearchResult {
    path: String,
    name: String,
    #[serde(rename = "type")]
    r#type: String,
    score: Option<f64>,
}

async fn initialize_database(app: tauri::AppHandle) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&app_dir).map_err(|e| e.to_string())?;
    
    let db_path = app_dir.join("speedy_index.db");
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
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
        );
        CREATE TABLE IF NOT EXISTS applications (
            id INTEGER PRIMARY KEY,
            path TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            icon_path TEXT,
            last_used TIMESTAMP,
            times_used INTEGER DEFAULT 0
        );"
    ).map_err(|e| e.to_string())?;
    
    app.manage(AppState { db: Mutex::new(conn) });
    Ok(())
}

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

#[tauri::command]
async fn index_files(path: String, app: tauri::AppHandle) -> Result<usize, String> {
    let state = app.state::<AppState>();
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let mut count = 0;

    for entry in WalkDir::new(path).max_depth(5).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path().to_string_lossy().into_owned();
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_file = entry.file_type().is_file();
        let is_app = is_file && entry.path().extension().map_or(false, |ext| ext == "exe");

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

#[tauri::command]
async fn index_applications(app: tauri::AppHandle) -> Result<usize, String> {
    let state = app.state::<AppState>();
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let mut count = 0;

    #[cfg(target_os = "windows")]
    {
        // Standard Windows application locations
        let app_paths = vec![
            PathBuf::from(r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs"),
            PathBuf::from(r"C:\Users\All Users\Microsoft\Windows\Start Menu\Programs"),
            PathBuf::from(r"C:\Users\*\AppData\Roaming\Microsoft\Windows\Start Menu\Programs"),
            PathBuf::from(r"C:\Program Files"),
            PathBuf::from(r"C:\Program Files (x86)"),
            PathBuf::from(r"C:\Windows\System32"),
        ];

        // Predefined system applications
        let system_apps = vec![
            ("cmd", "Command Prompt", r"C:\Windows\System32\cmd.exe"),
            ("powershell", "PowerShell", r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"),
            ("notepad", "Notepad", r"C:\Windows\System32\notepad.exe"),
            ("calc", "Calculator", r"C:\Windows\System32\calc.exe"),
            ("explorer", "File Explorer", "explorer.exe"),
            ("mspaint", "Paint", r"C:\Windows\System32\mspaint.exe"),
            ("wordpad", "WordPad", r"C:\Program Files\Windows NT\Accessories\wordpad.exe"),

            ("vscode", "Visual Studio Code", r"C:\Users\user\AppData\Local\Programs\Microsoft VS Code\Code.exe"),
            ("gitbash", "Git Bash", r"C:\Program Files\Git\git-bash.exe"),
            ("chrome", "Google Chrome", r"C:\Program Files\Google\Chrome\Application\chrome.exe"),
            ("snip", "Snip & Sketch", r"ms-screenclip:"), // this is a URI protocol
            ("settings", "Settings", r"ms-settings:"), // opens Windows settings
            ("whatsapp", "WhatsApp", r"C:\Users\user\AppData\Local\WhatsApp\WhatsApp.exe"),

            ("edge", "Microsoft Edge", r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"),
            ("teams", "Microsoft Teams", r"C:\Users\user\AppData\Local\Microsoft\Teams\Update.exe"),
            ("onenote", "OneNote", r"C:\Program Files\Microsoft Office\root\Office16\ONENOTE.EXE"),
            ("excel", "Microsoft Excel", r"C:\Program Files\Microsoft Office\root\Office16\EXCEL.EXE"),
            ("word", "Microsoft Word", r"C:\Program Files\Microsoft Office\root\Office16\WINWORD.EXE"),
        ];

        // Add system apps to database
        for (_, display_name, path) in system_apps {
            tx.execute(
                "INSERT OR REPLACE INTO applications 
                (path, name, last_used, times_used) 
                VALUES (?1, ?2, strftime('%s','now'), 0)",
                params![path, display_name],
            ).map_err(|e| e.to_string())?;
            count += 1;
        }

        // Index applications from standard locations
        for base_path in app_paths {
            for entry in WalkDir::new(base_path).max_depth(5).into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(OsStr::to_str) {
                        if ext == "exe" { // Just look for .exe files for now
                            let name = path.file_stem()
                                .and_then(OsStr::to_str)
                                .unwrap_or_default()
                                .to_string();

                            tx.execute(
                                "INSERT OR REPLACE INTO applications 
                                (path, name, last_used, times_used) 
                                VALUES (?1, ?2, strftime('%s','now'), 0)",
                                params![path.to_string_lossy().into_owned(), name],
                            ).map_err(|e| e.to_string())?;
                            
                            count += 1;
                        }
                    }
                }
            }
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}

#[tauri::command]
async fn search(query: String, app: tauri::AppHandle) -> Result<Vec<SearchResult>, String> {

    let state = app.state::<AppState>();
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    // Try to retrieve from cache first
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

    // Search files from database
    let mut stmt = conn.prepare(
        "SELECT path, name, is_file, is_app 
         FROM files 
         WHERE name LIKE ?1 
         ORDER BY last_accessed DESC, access_count DESC
         LIMIT 20"
    ).map_err(|e| e.to_string())?;

    let mut results = stmt
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

    // Search applications
    let app_results = search_apps(&query)?;
    results.extend(app_results);

    // Sort all results by score (if available) or by type
    results.sort_by(|a, b| {
        b.score.partial_cmp(&a.score)
            .unwrap_or_else(|| a.r#type.cmp(&b.r#type))
    });

    // Cache the results
    if !results.is_empty() {
        conn.execute(
            "INSERT OR REPLACE INTO search_cache (query, results, timestamp)
             VALUES (?1, ?2, strftime('%s','now'))",
            params![query, serde_json::to_string(&results).map_err(|e| e.to_string())?],
        ).map_err(|e| e.to_string())?;
    }

    Ok(results)
}

fn search_apps(query: &str) -> Result<Vec<SearchResult>, String> {
    let mut results = Vec::new();
    
    #[cfg(target_os = "windows")]
    {
        let start_menu_paths = vec![
            PathBuf::from(r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs"),
            PathBuf::from(r"C:\Users\All Users\Microsoft\Windows\Start Menu\Programs"),
        ];
        
        for path in start_menu_paths {
            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.filter_map(|e| e.ok()) {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_file() {
                            if let Some(ext) = entry.path().extension().and_then(OsStr::to_str) {
                                if ext == "lnk" {
                                    if let Some(name) = entry.file_name().to_str() {
                                        if name.to_lowercase().contains(&query.to_lowercase()) {
                                            results.push(SearchResult {
                                                path: entry.path().to_string_lossy().into_owned(),
                                                name: name.to_string(),
                                                r#type: "app".to_string(),
                                                score: Some(1.0),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        let app_dirs = vec![
            PathBuf::from("/Applications"),
            PathBuf::from("/System/Applications"),
            PathBuf::from(format!("{}/Applications", env::var("HOME").unwrap())),
        ];
        
        for dir in app_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            if let Some(ext) = entry.path().extension().and_then(OsStr::to_str) {
                                if ext == "app" {
                                    if let Some(name) = entry.file_name().to_str() {
                                        if name.to_lowercase().contains(&query.to_lowercase()) {
                                            results.push(SearchResult {
                                                path: entry.path().to_string_lossy().into_owned(),
                                                name: name.to_string(),
                                                r#type: "app".to_string(),
                                                score: Some(1.0),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(results)
}

#[tauri::command]
async fn open_path(path: String, app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE files 
         SET access_count = access_count + 1, 
             last_accessed = strftime('%s','now') 
         WHERE path = ?1",
        params![path],
    ).map_err(|e| e.to_string())?;

    launch_app(path)?;
    Ok(())
}

#[tauri::command]
fn launch_app(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // Simple launch command that works for both .exe and system commands
        Command::new("cmd")
            .args(&["/C", "start", "", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let window = app.get_webview_window("main")
                .ok_or("Failed to get window".to_string())?;

            window.show().map_err(|e| e.to_string())?;
            window.set_focus().map_err(|e| e.to_string())?;

            tauri::async_runtime::block_on(initialize_database(app.handle().clone()))?;

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let _ = index_files("C:\\Users".to_string(), app_handle.clone()).await;
                let _ = index_files("C:\\Program Files".to_string(), app_handle.clone()).await;
                let _ = index_applications(app_handle.clone()).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            toggle_window,
            search,
            index_files,
            index_applications,
            open_path,
            launch_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}