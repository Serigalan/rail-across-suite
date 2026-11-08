fn main() {
    // The bundled resources: the built site (scripts/build-site.mjs) and
    // mdb-export with its libraries (scripts/stage-mdbtools.mjs). Built
    // without them, the app finds the site elsewhere (README) and says so.
    for dir in ["site", "mdbtools"] {
        let _ = std::fs::create_dir_all(dir);
    }
    tauri_build::build()
}
