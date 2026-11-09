// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! Rail-Across-Suite on the desktop (decision 0020, step 9): the whole suite
//! in one Tauri window, the project server (`crates/server`) in this very
//! process. The window loads the built suite from the URI scheme `rail`
//! (`rail://localhost/`, `http://rail.localhost/` on Windows), and every
//! request on it — the pages, and the API under /api — is answered by the
//! server's router in this process: no port, no child process, no Node.
//!
//! The server runs in its local mode (no user management, one implicit local
//! user) with the cloud jobs as threads. Before the first main window a start
//! screen (ui/splash.html) asks for the suite's language, the folder of the
//! working files (database, point clouds) and the folder exported files are
//! saved to; the choices are kept in settings.json in the app data directory
//! (the local package of the branch `desktop-standalone`). `mdb-export` comes
//! bundled where scripts/stage-mdbtools.mjs found it.
//!
//! Environment: RA_DESKTOP_SITE — the built site (default: the bundled one,
//! else crates/desktop/site, else the repository's dist/); RA_DESKTOP_DATA —
//! the working files, overriding the start screen's choice.
//! `--smoke`: start on the defaults, load the suite, ask the API through the
//! window, fetch the Layout-Tool's page and compile the core's WebAssembly
//! module there, print what came back, then quit (CI's check of the package).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use rail_across_server::app::{App, Settings};
use rail_across_server::clouds::{jobs, Storage};
use tauri::http::{Request, Response};
use tauri::webview::DownloadEvent;
use tauri::{Builder, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent};

/// The server, once the working files are known.
static SERVER: OnceLock<Arc<App>> = OnceLock::new();
/// Tells the cloud jobs' threads to stop.
static STOP: OnceLock<Arc<AtomicBool>> = OnceLock::new();

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// What the start screen asks; None until chosen, then the defaults apply.
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
        let _ = std::fs::write(
            f,
            serde_json::json!({ "language": p.language, "workDir": p.work_dir, "exportDir": p.export_dir }).to_string(),
        );
    }
}

#[tauri::command]
fn current_prefs(app: tauri::AppHandle) -> serde_json::Value {
    let p = app.state::<PrefsState>().0.lock().unwrap().clone();
    serde_json::json!({
        "language": p.language, "workDir": p.work_dir, "exportDir": p.export_dir,
        "defaults": { "workDir": app.path().app_data_dir().ok(), "exportDir": app.path().download_dir().ok() },
    })
}

#[tauri::command]
fn start_app(app: tauri::AppHandle, prefs: serde_json::Value) -> Result<(), String> {
    let dir = |k: &str| prefs.get(k).and_then(|x| x.as_str()).filter(|d| !d.is_empty()).map(PathBuf::from);
    let p = Prefs {
        language: prefs.get("language").and_then(|x| x.as_str()).map(String::from),
        work_dir: dir("workDir"),
        export_dir: dir("exportDir"),
    };
    save_prefs(&app, &p);
    *app.state::<PrefsState>().0.lock().unwrap() = p;
    start_main(&app, false)?;
    if let Some(splash) = app.get_webview_window("splash") {
        let _ = splash.close();
    }
    Ok(())
}

/// The smoke check's report from the window.
#[tauri::command]
fn smoke_report(app: tauri::AppHandle, report: serde_json::Value) {
    println!("RA_DESKTOP_SMOKE={report}");
    stop_jobs();
    app.exit(if report.get("ok") == Some(&serde_json::json!(true)) { 0 } else { 1 });
}

/// The built suite: RA_DESKTOP_SITE, the bundled resource, the crate's site/, the repository's dist/.
fn find_site(app: &impl Manager<tauri::Wry>) -> Option<PathBuf> {
    let has_index = |p: &PathBuf| p.join("index.html").is_file();
    let mut candidates = Vec::new();
    if let Ok(p) = std::env::var("RA_DESKTOP_SITE") {
        candidates.push(PathBuf::from(p));
    }
    if let Ok(res) = app.path().resource_dir() {
        candidates.push(res.join("site"));
    }
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    candidates.push(crate_dir.join("site"));
    candidates.push(crate_dir.join("../../dist"));
    candidates.into_iter().find(has_index)
}

/// mdb-export bundled with its libraries (scripts/stage-mdbtools.mjs), told to the server.
fn use_bundled_mdbtools(app: &impl Manager<tauri::Wry>) {
    if std::env::var("RA_MDB_EXPORT").is_ok() {
        return;
    }
    let mut dirs = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("mdbtools")];
    if let Ok(res) = app.path().resource_dir() {
        dirs.insert(0, res.join("mdbtools"));
    }
    for d in dirs {
        let bin = d.join(if cfg!(windows) { "mdb-export.exe" } else { "mdb-export" });
        if bin.is_file() {
            std::env::set_var("RA_MDB_EXPORT", &bin);
            std::env::set_var("RA_MDB_LIBS", d.join("lib"));
            return;
        }
    }
}

/// The address of the scheme's root on this platform.
fn root_url() -> tauri::Url {
    let s = if cfg!(windows) { "http://rail.localhost/" } else { "rail://localhost/" };
    s.parse().expect("the scheme's address")
}

/// One request on the scheme, answered by the server in this process.
fn answer(request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let Some(app) = SERVER.get() else {
        return Response::builder().status(503).body(b"the project server is not started yet".to_vec()).unwrap();
    };
    let url = request.uri().path_and_query().map(|p| p.as_str().to_string()).unwrap_or_else(|| "/".into());
    let headers: Vec<(String, String)> = request
        .headers()
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), String::from_utf8_lossy(v.as_bytes()).into_owned()))
        .collect();
    let method = request.method().as_str().to_string();
    let (status, headers, body) = app.call(&method, &url, headers, request.into_body());
    let mut b = Response::builder().status(status);
    for (k, v) in headers {
        b = b.header(k, v);
    }
    b.body(body).unwrap_or_else(|_| Response::builder().status(500).body(Vec::new()).unwrap())
}

/// Start the server on the working files and open the main window on it.
fn start_main(app: &tauri::AppHandle, smoke: bool) -> Result<(), String> {
    let site = find_site(app).ok_or("no built site found — run `node scripts/build-site.mjs` in crates/desktop, or set RA_DESKTOP_SITE")?;
    let prefs = app.state::<PrefsState>().0.lock().unwrap().clone();
    let data = std::env::var("RA_DESKTOP_DATA")
        .ok()
        .map(PathBuf::from)
        .or(prefs.work_dir)
        .or_else(|| app.path().app_data_dir().ok())
        .ok_or("no app data directory")?;
    std::fs::create_dir_all(&data).map_err(|e| e.to_string())?;
    let db_path = data.join("rail-across.sqlite");
    let clouds = data.join("clouds");
    use_bundled_mdbtools(app);
    if SERVER.get().is_none() {
        let db = rail_across_server::db::open_database(&db_path.to_string_lossy(), &rail_across_server::util::iso(now())).map_err(|e| e.to_string())?;
        for (name, note) in rail_across_server::data_migrations::run_data_migrations(&db, now()).map_err(|e| e.to_string())? {
            eprintln!("data step {name}: {note}");
        }
        let settings = Settings {
            secure_cookie: false,
            cloud_root: Some(clouds.clone()),
            local: true,
            static_root: Some(site.clone()),
            ..Settings::default()
        };
        let _ = SERVER.set(Arc::new(App::new(db, settings)));
        let stop = STOP.get_or_init(|| Arc::new(AtomicBool::new(false))).clone();
        let path = db_path.to_string_lossy().into_owned();
        std::thread::spawn(move || jobs::run_queues(path, Storage { root: clouds }, (2, 2), stop, Arc::new(|m: &str| eprintln!("{m}"))));
    }
    eprintln!("rail-across-suite on the desktop: site {}, working files {}", site.display(), data.display());
    let lang = prefs.language.unwrap_or_else(|| "de".into());
    let mut script = format!("try{{localStorage.setItem('rail-across-language',{lang:?})}}catch(e){{}}");
    if smoke {
        // Once the start page is there: ask the API as the suite does, and report.
        script.push_str(
            "window.addEventListener('load', async () => { const r = {}; try {
               r.me = await (await fetch('/api/me')).json();
               r.health = await (await fetch('/api/health')).json();
               const made = await fetch('/api/projects', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ title: 'Smoke' }) });
               r.created = made.status;
               r.projects = (await (await fetch('/api/projects')).json()).projects.map(p => p.title);
               r.page = document.title;
               r.layout = (await fetch('/layout/')).status;
               const wasm = await fetch('/core-module/rail_across_core.wasm');
               r.coreModule = wasm.ok ? (await WebAssembly.compile(await wasm.arrayBuffer()), 'compiled') : wasm.status;
               r.ok = r.health.local === true && r.created === 201 && r.projects.includes('Smoke') && r.coreModule !== undefined;
             } catch (e) { r.error = String(e); r.ok = false }
             window.__TAURI__.core.invoke('smoke_report', { report: r }) });",
        );
    }
    WebviewWindowBuilder::new(app, "main", WebviewUrl::CustomProtocol(root_url()))
        .title("Rail-Across-Suite")
        .inner_size(1440.0, 900.0)
        .min_inner_size(1024.0, 640.0)
        .initialization_script(&script)
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

fn stop_jobs() {
    if let Some(s) = STOP.get() {
        s.store(true, Ordering::Relaxed);
    }
}

fn main() {
    let smoke = std::env::args().any(|a| a == "--smoke");
    let app = Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .register_asynchronous_uri_scheme_protocol("rail", |_ctx, request, responder| {
            std::thread::spawn(move || responder.respond(answer(request)));
        })
        .invoke_handler(tauri::generate_handler![current_prefs, start_app, smoke_report])
        .setup(move |app| {
            let handle = app.handle().clone();
            app.manage(PrefsState(Mutex::new(load_prefs(&handle))));
            let has_settings = settings_file(&handle).is_some_and(|f| f.exists());
            if has_settings || smoke {
                start_main(&handle, smoke).map_err(Box::<dyn std::error::Error>::from)?;
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
                    stop_jobs();
                    window.app_handle().exit(0);
                }
            }
        })
        .build(tauri::generate_context!());
    let app = match app {
        Ok(app) => app,
        Err(err) => {
            eprintln!("error: {err}");
            std::process::exit(1);
        }
    };
    app.run(|handle, event| match event {
        RunEvent::ExitRequested { .. } => stop_jobs(),
        RunEvent::WindowEvent {
            event: WindowEvent::Destroyed, ..
        } if handle.webview_windows().is_empty() => {
            stop_jobs();
            handle.exit(0);
        }
        _ => {}
    });
}
