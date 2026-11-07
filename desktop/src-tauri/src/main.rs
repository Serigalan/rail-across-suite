//! The local package of Rail-Across-Suite (desktop/): one window on the
//! project server running beside it on 127.0.0.1 — `serve --local`, without
//! user management — which also serves the built suite (desktop/site), so
//! page and API under /api share one origin, as behind Caddy in production.
//!
//! The server is a child process: this shell starts it on a free port
//! (--port 0), reads the address it prints (RA_DESKTOP_URL=…), opens the
//! window on it and stops the server when the app exits. The database and
//! the point clouds live in the app's own data directory, not in the
//! checkout.
//!
//! Before the main window, a start screen (splash.html) shows the logo and
//! asks for the suite's language, the folder of the working files (database,
//! point clouds) and the folder exported files are saved to; the choices are
//! kept in settings.json in the app data directory. The language is set on
//! every page of the main window, downloads are redirected to the export
//! folder (WebviewWindowBuilder::on_download), the server's database and
//! clouds live in the working-files folder.
//!
//! Environment: RA_DESKTOP_REPO — the repository checkout (default: the one
//! this was built from; in the packaged app the bundled runtime/ is used);
//! RA_DESKTOP_NODE — the Node.js binary (default: the one bundled as the
//! runtime/node/bin/node resource, else node on PATH — Node.js 22 or later);
//! RA_DESKTOP_SITE — the built site (default: <repo>/desktop/site, else
//! the bundled site resource).

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{Builder, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri::webview::DownloadEvent;

/// The server child process, stopped when the app exits.
struct Server(Mutex<Option<Child>>);

/// What the start screen asks: the suite's language, the folder of the working
/// files (database, point clouds) and the folder exported files are saved to.
/// Kept in settings.json in the app data directory; None until chosen — then
/// the defaults apply (app data directory, system download directory).
#[derive(Default, Clone)]
struct Prefs {
    language: Option<String>,
    work_dir: Option<PathBuf>,
    export_dir: Option<PathBuf>,
}

struct PrefsState(Mutex<Prefs>);

fn settings_file(app: &impl Manager<tauri::Wry>) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("settings.json"))
}

fn load_prefs(app: &impl Manager<tauri::Wry>) -> Prefs {
    let mut p = Prefs::default();
    if let Ok(s) = std::fs::read_to_string(settings_file(app).unwrap_or_default()) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
            p.language = v.get("language").and_then(|x| x.as_str()).map(String::from);
            p.work_dir = v.get("workDir").and_then(|x| x.as_str()).map(PathBuf::from);
            p.export_dir = v.get("exportDir").and_then(|x| x.as_str()).map(PathBuf::from);
        }
    }
    p
}

fn save_prefs(app: &impl Manager<tauri::Wry>, p: &Prefs) {
    if let Some(f) = settings_file(app) {
        if let Some(d) = f.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(f, serde_json::json!({
            "language": p.language,
            "workDir": p.work_dir,
            "exportDir": p.export_dir,
        }).to_string());
    }
}

/// What the start screen shows: the remembered choices, plus the defaults
/// (app data directory for the working files, system download folder for exports).
#[tauri::command]
fn current_prefs(app: tauri::AppHandle) -> serde_json::Value {
    let p = app.state::<PrefsState>().0.lock().unwrap().clone();
    serde_json::json!({
        "language": p.language,
        "workDir": p.work_dir,
        "exportDir": p.export_dir,
        "defaults": {
            "workDir": app.path().app_data_dir().ok(),
            "exportDir": app.path().download_dir().ok(),
        },
    })
}

/// Answered on the start screen: keep the choices, start the server with the
/// working files in the chosen folder, open the main window in the chosen
/// language, then close the start screen.
#[tauri::command]
fn start_app(app: tauri::AppHandle, prefs: serde_json::Value) -> Result<(), String> {
    let p = Prefs {
        language: prefs.get("language").and_then(|x| x.as_str()).map(String::from),
        work_dir: prefs.get("workDir").and_then(|x| x.as_str()).filter(|d| !d.is_empty()).map(PathBuf::from),
        export_dir: prefs.get("exportDir").and_then(|x| x.as_str()).filter(|d| !d.is_empty()).map(PathBuf::from),
    };
    save_prefs(&app, &p);
    *app.state::<PrefsState>().0.lock().unwrap() = p;
    start_main(&app)?;
    if let Some(splash) = app.get_webview_window("splash") {
        let _ = splash.close();
    }
    Ok(())
}

/// The project server's package root (the directory with bin/rail-across-server.mjs):
/// from the checkout (RA_DESKTOP_REPO, or the one this was built from, or beside the
/// executable), or from the bundled resource runtime/server of the packaged app. */
fn find_server(app: &impl Manager<tauri::Wry>) -> Option<PathBuf> {
    let bin = |root: &PathBuf| root.join("server/bin/rail-across-server.mjs").is_file();
    if let Ok(p) = std::env::var("RA_DESKTOP_REPO") {
        let p = PathBuf::from(p);
        if bin(&p) {
            return Some(p.join("server"));
        }
    }
    if let Some(manifest) = option_env!("CARGO_MANIFEST_DIR") {
        let p = PathBuf::from(manifest).join("../..");
        if bin(&p) {
            return Some(p.join("server"));
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for p in [dir.to_path_buf(), dir.join("..")] {
                if bin(&p) {
                    return Some(p.join("server"));
                }
            }
        }
    }
    if let Ok(res) = app.path().resource_dir() {
        for p in [res.join("runtime"), res] {
            if bin(&p) {
                return Some(p.join("server"));
            }
        }
    }
    None
}

/// The Node.js binary that runs the server: RA_DESKTOP_NODE, else the binary
/// bundled in the packaged app (runtime/node/bin/node resource) or staged beside
/// the checkout, else plain `node` on PATH (Node.js 22 or later).
fn find_node(app: &impl Manager<tauri::Wry>, server: &PathBuf) -> PathBuf {
    if let Ok(p) = std::env::var("RA_DESKTOP_NODE") {
        return PathBuf::from(p);
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(res) = app.path().resource_dir() {
        for name in ["node", "node.exe"] {
            candidates.push(res.join("runtime/node/bin").join(name));
            candidates.push(res.join("node/bin").join(name));
        }
    }
    if let Some(repo) = server.parent() {
        for name in ["node", "node.exe"] {
            candidates.push(repo.join("desktop/runtime/node/bin").join(name));
        }
    }
    for p in candidates {
        if p.is_file() {
            return p;
        }
    }
    PathBuf::from("node")
}

/// The built site: the start page at the root, each module's app under its path. */
fn find_site(app: &impl Manager<tauri::Wry>, server: &PathBuf) -> Option<PathBuf> {
    if let Ok(p) = std::env::var("RA_DESKTOP_SITE") {
        return Some(PathBuf::from(p));
    }
    // server/ lies in the checkout as <repo>/server — the site beside it as <repo>/desktop/site.
    if let Some(repo) = server.parent() {
        let s = repo.join("desktop/site");
        if s.is_dir() {
            return Some(s);
        }
    }
    if let Ok(res) = app.path().resource_dir() {
        let s = res.join("site");
        if s.is_dir() {
            return Some(s);
        }
    }
    None
}

/// Start the project server as a child process and open the main window on
/// the address it prints; its downloads are saved to the chosen export folder.
fn start_main(app: &tauri::AppHandle) -> Result<(), String> {
    let server = find_server(app).ok_or("no project server found — set RA_DESKTOP_REPO, or build the package (npm run build)")?;
    let site = find_site(app, &server)
        .ok_or("no built site found — run `npm run build:site` (or npm run dev) first")?;
    let node = find_node(app, &server);

    // The working files (database, point clouds) in the folder chosen on the
    // start screen, else in the app data directory.
    let prefs = app.state::<PrefsState>().0.lock().unwrap().clone();
    let data = prefs.work_dir
        .or_else(|| app.path().app_data_dir().ok())
        .ok_or("no app data directory")?;
    std::fs::create_dir_all(&data).map_err(|e| e.to_string())?;
    let db = data.join("rail-across.sqlite");
    let clouds = data.join("clouds");

    let mut child = Command::new(&node)
        .arg(server.join("bin/rail-across-server.mjs"))
        .arg("serve")
        .arg("--local")
        .arg("--static")
        .arg(&site)
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg("0")
        .current_dir(&server)
        .env("RA_SERVER_DB", &db)
        .env("RA_SERVER_CLOUDS", &clouds)
        .env("RA_SERVER_LOCAL", "1")
        .env("RA_SERVER_LOG", "warn")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("cannot start {} (no bundled Node.js and none on PATH, or set RA_DESKTOP_NODE): {e}", node.display()))?;

    // The server prints its bound address as RA_DESKTOP_URL=… once it listens.
    let (tx, rx) = mpsc::channel();
    let mut stdout = child.stdout.take().expect("piped stdout");
    std::thread::spawn(move || {
        for line in BufReader::new(&mut stdout).lines() {
            match line {
                Ok(l) => {
                    if tx.send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut url = None;
    while let Ok(line) = rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        if let Some(u) = line.strip_prefix("RA_DESKTOP_URL=") {
            url = Some(u.trim().to_string());
            break;
        }
    }
    let url = url.ok_or("the project server did not answer with its address (RA_DESKTOP_URL)")?;
    eprintln!("rail-across-suite (local package) on {url}, database at {}", db.display());

    *app.state::<Server>().0.lock().unwrap() = Some(child);
    let url: tauri::Url = url.parse().map_err(|e| format!("bad server address: {e}"))?;
    // The suite's language, chosen on the start screen, kept for every page of
    // this window (the suite reads localStorage['rail-across-language']).
    let lang = prefs.language.clone().unwrap_or_else(|| "de".to_string());
    let lang_script = format!("try{{localStorage.setItem('rail-across-language',{lang:?})}}catch(e){{}}");
    WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url))
        .title("Rail-Across-Suite")
        .inner_size(1440.0, 900.0)
        .min_inner_size(1024.0, 640.0)
        .initialization_script(&lang_script)
        // Exported files (the suite's downloads) land in the folder chosen on
        // the start screen, keeping the suggested file name.
        .on_download(|webview, event| {
            if let DownloadEvent::Requested { destination, .. } = event {
                let dir = webview.state::<PrefsState>().0.lock().unwrap().export_dir.clone();
                if let (Some(dir), Some(name)) = (dir, destination.file_name()) {
                    *destination = dir.join(name);
                }
            }
            true
        })
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    let app = match Builder::default()
        .manage(Server(Mutex::new(None)))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![current_prefs, start_app])
        .setup(|app| {
            let handle = app.handle().clone();
            app.manage(PrefsState(Mutex::new(load_prefs(&handle))));
            // The start screen (logo centered, the questions below it — language,
            // working files, exports) only on the initial start; once settings.json
            // exists, go straight to the main window with the remembered choices.
            let has_settings = settings_file(&handle).map(|f| f.exists()).unwrap_or(false);
            if has_settings {
                start_main(&handle).map_err(Box::<dyn std::error::Error>::from)?;
            } else {
                WebviewWindowBuilder::new(app, "splash", WebviewUrl::App("splash.html".into()))
                    .title("Rail-Across-Suite")
                    .inner_size(560.0, 470.0)
                    .resizable(false)
                    .center()
                    .build()?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { .. } = event {
                if window.label() == "main" {
                    // The main window closed: stop the server and quit the app.
                    let handle = window.app_handle().clone();
                    let child = handle.state::<Server>().0.lock().unwrap().take();
                    if let Some(mut child) = child {
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                    handle.exit(0);
                }
            }
        })
        .build(tauri::generate_context!())
    {
        Ok(app) => app,
        Err(err) => {
            eprintln!("error: {err}");
            std::process::exit(1);
        }
    };

    app.run(|app_handle, event| match event {
        RunEvent::ExitRequested { .. } => {
            let child = app_handle.state::<Server>().0.lock().unwrap().take();
            if let Some(mut child) = child {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        // macOS keeps the app alive when its last window is closed; quit then,
        // so the app cannot linger without windows (and the start screen cannot
        // come back on the next launch of a still-running instance).
        RunEvent::WindowEvent { event: WindowEvent::Destroyed, .. } => {
            if app_handle.webview_windows().is_empty() {
                let child = app_handle.state::<Server>().0.lock().unwrap().take();
                if let Some(mut child) = child {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                app_handle.exit(0);
            }
        }
        _ => {}
    });
}
