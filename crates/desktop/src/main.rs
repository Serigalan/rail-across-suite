// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

// Started from Explorer, no console window beside the app's (release builds).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Rail-Across-Suite on the desktop (decision 0005): the whole suite
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
//! (the local package of the branch `desktop-standalone`). The suite's start
//! page opens the start screen again (`open_settings`); a change restarts the
//! app. `mdb-export` comes bundled where scripts/stage-mdbtools.mjs found it.
//! The templates of templates/ — test data — become templates of the server
//! on the first start on a folder of working files (`seed_templates`).
//!
//! The portable package (Windows, a ZIP): a file `portable` beside the
//! executable puts everything the app keeps into that folder — settings.json,
//! the working files under data/, the exports under export/, the webview's own
//! storage (the working copies, the language) under webview/ — and paths in it
//! are kept relative to it, so it can move, a USB stick under another drive
//! letter included. Started from Explorer, what the server reports goes to
//! rail-across.log beside the working files.
//!
//! Environment: RA_DESKTOP_SITE — the built site (default: the bundled one,
//! else crates/desktop/site, else the repository's dist/); RA_DESKTOP_DATA —
//! the working files, overriding the start screen's choice;
//! RA_DESKTOP_PORTABLE — a folder to run portable in, without the marker file.
//! `--smoke`: start on the defaults, load the suite, ask the API through the
//! window, fetch the Layout-Tool's page and compile the core's WebAssembly
//! module there, try to open a window (`window.open`), print what came back,
//! then quit (CI's check of the package).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use rail_across_server::app::{App, Settings};
use rail_across_server::clouds::{jobs, Storage};
use tauri::http::{Request, Response};
use tauri::webview::{DownloadEvent, NewWindowFeatures, NewWindowResponse};
use tauri::{Builder, Manager, RunEvent, Webview, WebviewUrl, WebviewWindowBuilder, WindowEvent};

/// The server, once the working files are known.
static SERVER: OnceLock<Arc<App>> = OnceLock::new();
/// Tells the cloud jobs' threads to stop.
static STOP: OnceLock<Arc<AtomicBool>> = OnceLock::new();
/// The folder of the portable package, if it is one.
static PORTABLE: OnceLock<Option<PathBuf>> = OnceLock::new();

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// RA_DESKTOP_PORTABLE, else the executable's folder where a file `portable` lies in it.
fn portable() -> Option<&'static PathBuf> {
    PORTABLE
        .get_or_init(|| {
            if let Some(d) = std::env::var_os("RA_DESKTOP_PORTABLE").filter(|d| !d.is_empty()) {
                return Some(PathBuf::from(d));
            }
            let dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
            dir.join("portable").is_file().then_some(dir)
        })
        .as_ref()
}

/// A folder as settings.json keeps it: inside the portable folder, relative to it.
fn kept(p: &Path) -> String {
    match portable().and_then(|base| p.strip_prefix(base).ok()) {
        Some(rel) => rel.to_string_lossy().into_owned(),
        None => p.to_string_lossy().into_owned(),
    }
}

/// A folder from settings.json: a relative one lies in the portable folder.
fn resolved(s: &str) -> PathBuf {
    let p = PathBuf::from(s);
    match portable() {
        Some(base) if p.is_relative() => base.join(p),
        _ => p,
    }
}

/// Where settings.json lies: the portable folder, else the app data directory.
fn settings_dir(app: &impl Manager<tauri::Wry>) -> Option<PathBuf> {
    portable().cloned().or_else(|| app.path().app_data_dir().ok())
}

/// The working files unless chosen: data/ in the portable folder, else the app data directory.
fn default_work_dir(app: &impl Manager<tauri::Wry>) -> Option<PathBuf> {
    match portable() {
        Some(base) => Some(base.join("data")),
        None => app.path().app_data_dir().ok(),
    }
}

/// The exports unless chosen: export/ in the portable folder, else where the webview saves downloads.
fn default_export_dir() -> Option<PathBuf> {
    portable().map(|base| base.join("export"))
}

/// The webview's own storage: webview/ in the portable folder, else the platform's place for it.
fn with_storage<'a, M: Manager<tauri::Wry>>(b: WebviewWindowBuilder<'a, tauri::Wry, M>) -> WebviewWindowBuilder<'a, tauri::Wry, M> {
    match portable() {
        Some(base) => b.data_directory(base.join("webview")),
        None => b,
    }
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
    settings_dir(app).map(|d| d.join("settings.json"))
}

fn load_prefs(app: &impl Manager<tauri::Wry>) -> Prefs {
    let mut p = Prefs::default();
    if let Ok(s) = std::fs::read_to_string(settings_file(app).unwrap_or_default()) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
            let dir = |k: &str| v.get(k).and_then(|x| x.as_str()).filter(|d| !d.is_empty()).map(resolved);
            p.language = v.get("language").and_then(|x| x.as_str()).map(String::from);
            p.work_dir = dir("workDir");
            p.export_dir = dir("exportDir");
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
            serde_json::json!({
                "language": p.language,
                "workDir": p.work_dir.as_deref().map(kept),
                "exportDir": p.export_dir.as_deref().map(kept),
            })
            .to_string(),
        );
    }
}

#[tauri::command]
fn current_prefs(app: tauri::AppHandle) -> serde_json::Value {
    let p = app.state::<PrefsState>().0.lock().unwrap().clone();
    serde_json::json!({
        "language": p.language, "workDir": p.work_dir, "exportDir": p.export_dir,
        "defaults": {
            "workDir": default_work_dir(&app),
            "exportDir": default_export_dir().or_else(|| app.path().download_dir().ok()),
        },
        "portable": portable().is_some(),
        "running": SERVER.get().is_some(),
    })
}

/// The start screen's choices: kept, then the suite starts — or, opened again
/// from the suite, starts anew on them (the server's working files are fixed
/// for its lifetime). Async: a window built in a synchronous command
/// deadlocks on Windows (WebView2).
#[tauri::command]
async fn start_app(app: tauri::AppHandle, prefs: serde_json::Value) -> Result<(), String> {
    let dir = |k: &str| prefs.get(k).and_then(|x| x.as_str()).filter(|d| !d.is_empty()).map(PathBuf::from);
    let p = Prefs {
        language: prefs.get("language").and_then(|x| x.as_str()).map(String::from),
        work_dir: dir("workDir"),
        export_dir: dir("exportDir"),
    };
    save_prefs(&app, &p);
    *app.state::<PrefsState>().0.lock().unwrap() = p;
    if SERVER.get().is_some() {
        stop_jobs();
        app.restart();
    }
    start_main(&app, false)?;
    if let Some(splash) = app.get_webview_window("splash") {
        let _ = splash.close();
    }
    Ok(())
}

/// The start screen again, from the suite's start page.
#[tauri::command]
async fn open_settings(app: tauri::AppHandle) -> Result<(), String> {
    match app.get_webview_window("splash") {
        Some(w) => w.set_focus().map_err(|e| e.to_string()),
        None => open_splash(&app).map_err(|e| e.to_string()),
    }
}

/// The language chosen in the suite, for the next start.
#[tauri::command]
fn set_language(app: tauri::AppHandle, language: String) {
    let state = app.state::<PrefsState>();
    let mut p = state.0.lock().unwrap();
    p.language = Some(language);
    save_prefs(&app, &p);
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

/// The templates the app brings (templates/): test data, to try the suite on —
/// the line Halle–Könnern of a PEK, its timetables beside it.
const TEMPLATES: [(&str, &str, &str); 1] = [(
    "PEK Halle-Könnern (Testdaten)",
    include_str!("../templates/pek-halle-koennern.railacross.json"),
    include_str!("../templates/pek-halle-koennern.timetables.json"),
)];

const TEMPLATE_NOTE: &str = "Testdaten zum Ausprobieren der Suite – keine verbindlichen Planungsdaten. \
                             Test data to try the suite on – not binding planning data.";

/// The bundled templates made templates of the server, once per folder of working
/// files (a file `templates-seeded` there): one deleted stays deleted.
fn seed_templates(server: &App, data: &Path) {
    let marker = data.join("templates-seeded");
    if marker.exists() {
        return;
    }
    for (title, document, timetables) in TEMPLATES {
        let (Ok(payload), Ok(timetables)) = (
            serde_json::from_str::<serde_json::Value>(document),
            serde_json::from_str::<serde_json::Value>(timetables),
        ) else {
            eprintln!("template {title}: not readable");
            continue;
        };
        let body = serde_json::json!({
            "title": title, "description": TEMPLATE_NOTE, "template": true, "variantName": "Bestand",
            "payload": payload, "timetables": timetables["timetables"],
        });
        let headers = vec![("content-type".to_string(), "application/json".to_string())];
        let (status, _, answer) = server.call("POST", "/api/projects", headers, body.to_string().into_bytes());
        if status != 201 {
            eprintln!("template {title}: {status} {}", String::from_utf8_lossy(&answer));
            return;
        }
    }
    let _ = std::fs::write(marker, "");
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
        .or_else(|| default_work_dir(app))
        .ok_or("no app data directory")?;
    std::fs::create_dir_all(&data).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    stderr_to_log(&data);
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
        let server = Arc::new(App::new(db, settings));
        seed_templates(&server, &data);
        let _ = SERVER.set(server);
        let stop = STOP.get_or_init(|| Arc::new(AtomicBool::new(false))).clone();
        let path = db_path.to_string_lossy().into_owned();
        std::thread::spawn(move || jobs::run_queues(path, Storage { root: clouds }, (2, 2), stop, Arc::new(|m: &str| eprintln!("{m}"))));
    }
    eprintln!("rail-across-suite on the desktop: site {}, working files {}", site.display(), data.display());
    let lang = prefs.language.unwrap_or_else(|| "de".into());
    let mut script = format!("try{{localStorage.setItem('rail-across-language',{lang:?})}}catch(e){{}}");
    if smoke {
        // Once the start page is there: ask the API as the suite does, and report.
        script.push_str(&format!(";const TEMPLATE = {:?};", TEMPLATES[0].0));
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
               // Reported, not required: WebKitGTK opens no window without a click.
               const popup = window.open('/layout/', 'smoke-window');
               r.window = popup ? 'opened' : null;
               popup?.close();
               r.ok = r.health.local === true && r.created === 201 && r.projects.includes('Smoke') && r.projects.includes(TEMPLATE) && r.coreModule !== undefined;
             } catch (e) { r.error = String(e); r.ok = false }
             window.__TAURI__.core.invoke('smoke_report', { report: r }) });",
        );
    }
    with_storage(WebviewWindowBuilder::new(app, "main", WebviewUrl::CustomProtocol(root_url())))
        .title("Rail-Across-Suite")
        .inner_size(1440.0, 900.0)
        .min_inner_size(1024.0, 640.0)
        .initialization_script(&script)
        .on_download(save_to_exports)
        .on_new_window({
            let app = app.clone();
            move |url, features| open_window(&app, url, features)
        })
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// A download goes to the export folder.
fn save_to_exports(webview: Webview, event: DownloadEvent) -> bool {
    if let DownloadEvent::Requested { destination, .. } = event {
        let dir = webview.state::<PrefsState>().0.lock().unwrap().export_dir.clone().or_else(default_export_dir);
        if let (Some(dir), Some(name)) = (dir, destination.file_name()) {
            let _ = std::fs::create_dir_all(&dir);
            *destination = dir.join(name);
        }
    }
    true
}

/// A window the suite opens (`window.open`: the point clouds' 3D view, the
/// map's panels popped out, a vehicle in its editor) — without a handler the
/// webview opens none. It becomes a window of the app in the opener's webview
/// environment, so the two share their storage and their BroadcastChannel and
/// the opener draws into an empty one; an address outside the suite is not opened.
fn open_window(app: &tauri::AppHandle, url: tauri::Url, features: NewWindowFeatures) -> NewWindowResponse<tauri::Wry> {
    static NEXT: AtomicUsize = AtomicUsize::new(1);
    let root = root_url();
    if url.as_str() != "about:blank" && (url.scheme(), url.host_str()) != (root.scheme(), root.host_str()) {
        return NewWindowResponse::Deny;
    }
    let label = format!("window-{}", NEXT.fetch_add(1, Ordering::Relaxed));
    let built = WebviewWindowBuilder::new(app, label, WebviewUrl::External("about:blank".parse().expect("about:blank")))
        .title("Rail-Across-Suite")
        .inner_size(1280.0, 800.0)
        .window_features(features)
        .on_document_title_changed(|window, title| {
            let _ = window.set_title(&title);
        })
        .on_download(save_to_exports)
        .on_new_window({
            let app = app.clone();
            move |url, features| open_window(&app, url, features)
        })
        .build();
    match built {
        Ok(window) => NewWindowResponse::Create { window },
        Err(err) => {
            eprintln!("window for {url} not opened: {err}");
            NewWindowResponse::Deny
        }
    }
}

/// The start screen (ui/splash.html).
fn open_splash(app: &tauri::AppHandle) -> tauri::Result<()> {
    with_storage(WebviewWindowBuilder::new(app, "splash", WebviewUrl::App("splash.html".into())))
        .title("Rail-Across-Suite")
        .inner_size(560.0, 560.0)
        .resizable(false)
        .center()
        .build()?;
    Ok(())
}

/// Started from Explorer, a Windows app has no stderr: what the server and its
/// jobs report goes to rail-across.log beside the working files then, begun
/// anew past 5 MB. Run from a console or with its output caught (CI), it stays there.
#[cfg(windows)]
fn stderr_to_log(dir: &Path) {
    use std::os::windows::io::IntoRawHandle;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetStdHandle(which: u32) -> isize;
        fn SetStdHandle(which: u32, handle: isize) -> i32;
    }
    const STD_ERROR_HANDLE: u32 = 0xFFFF_FFF4;
    // SAFETY: asks for the process's own standard handle; null or -1 when it has none.
    let current = unsafe { GetStdHandle(STD_ERROR_HANDLE) };
    if current != 0 && current != -1 {
        return;
    }
    let path = dir.join("rail-across.log");
    let fresh = std::fs::metadata(&path).map(|m| m.len() > 5 << 20).unwrap_or(false);
    let file = std::fs::OpenOptions::new().create(true).append(!fresh).write(true).truncate(fresh).open(&path);
    if let Ok(file) = file {
        // SAFETY: hands the file's handle, which it gives up, to the process as its stderr.
        unsafe { SetStdHandle(STD_ERROR_HANDLE, file.into_raw_handle() as isize) };
    }
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
        .invoke_handler(tauri::generate_handler![current_prefs, start_app, open_settings, set_language, smoke_report])
        .setup(move |app| {
            let handle = app.handle().clone();
            app.manage(PrefsState(Mutex::new(load_prefs(&handle))));
            let has_settings = settings_file(&handle).is_some_and(|f| f.exists());
            if has_settings || smoke {
                start_main(&handle, smoke).map_err(Box::<dyn std::error::Error>::from)?;
            } else {
                open_splash(&handle)?;
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
