// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! rail-across-server — the project server in Rust (`cli.js`, `clouds/cli.js`):
//!
//!   rail-across-server serve                        run the API (RA_SERVER_HOST, RA_SERVER_PORT);
//!                                                   with RA_SERVER_JOBS=1 the cloud jobs as threads too
//!   rail-across-server create-admin <login> [name]  create an admin (RA_ADMIN_PASSWORD or a printed one)
//!   rail-across-server backup <file>                write a consistent copy of the database
//!   rail-across-server user-count                   print how many users exist
//!   rail-across-server migrate                      bring the database up to date, data steps included
//!   rail-across-server cloudjobs serve              the queues of the cloud jobs, as threads
//!   rail-across-server cloudjobs run <job>          prepare one cloud
//!   rail-across-server cloudjobs exec <run>         one long run
//!
//! The database is RA_SERVER_DB (default ./rail-across.sqlite), the clouds RA_SERVER_CLOUDS, OSRD's nets RA_SERVER_OSRD.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use rail_across_server::app::{App, LogLevel, Settings};
use rail_across_server::auth;
use rail_across_server::clouds::{Storage, jobs};
use rail_across_server::db::{backup, open_database};
use rail_across_server::util::iso;

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

fn db_path() -> String {
    env("RA_SERVER_DB").unwrap_or_else(|| "rail-across.sqlite".into())
}

fn clouds_path(db: &str) -> PathBuf {
    env("RA_SERVER_CLOUDS").map(PathBuf::from).unwrap_or_else(|| {
        let abs = std::fs::canonicalize(Path::new(db).parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new(".")))
            .unwrap_or_else(|_| PathBuf::from("."));
        abs.join("clouds")
    })
}

/// OSRD's nets (`RA_SERVER_OSRD`): by default `osrd/` beside the database.
fn osrd_path(db: &str) -> PathBuf {
    env("RA_SERVER_OSRD").map(PathBuf::from).unwrap_or_else(|| {
        let abs = std::fs::canonicalize(Path::new(db).parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new(".")))
            .unwrap_or_else(|_| PathBuf::from("."));
        abs.join("osrd")
    })
}

fn stamp(m: &str) {
    println!("{} {m}", iso(now()));
}

fn fail(m: impl std::fmt::Display) -> ! {
    eprintln!("{m}");
    std::process::exit(1)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = db_path();
    match args.first().map(String::as_str) {
        Some("serve") => serve(&db, &args[1..]),
        Some("create-admin" | "user-count") if env("RA_SERVER_LOCAL").as_deref() == Some("1") => {
            fail("the local package has no user management (it runs serve --local)")
        }
        Some("create-admin") => {
            let Some(login) = args.get(1) else {
                fail("usage: rail-across-server create-admin <login> [name]")
            };
            let conn = open_database(&db, &iso(now())).unwrap_or_else(|e| fail(e));
            if auth::user_by_login(&conn, login).ok().flatten().is_some() {
                fail(format!("a user \"{login}\" exists already"));
            }
            let given = env("RA_ADMIN_PASSWORD");
            if given.as_ref().is_some_and(|g| g.chars().count() < 12) {
                fail("RA_ADMIN_PASSWORD is shorter than 12 characters");
            }
            let password = given.clone().unwrap_or_else(auth::random_password);
            let name = if args.len() > 2 { args[2..].join(" ") } else { login.clone() };
            auth::create_user(&conn, login, &name, &password, "admin", true, now()).unwrap_or_else(|e| fail(e));
            println!("admin \"{login}\" created.");
            if given.is_none() {
                println!("start password: {password}  (to be changed at the first sign-in)");
            }
        }
        Some("backup") => {
            let Some(file) = args.get(1) else {
                fail("usage: rail-across-server backup <file>")
            };
            let conn = open_database(&db, &iso(now())).unwrap_or_else(|e| fail(e));
            backup(&conn, file).unwrap_or_else(|e| fail(e));
            println!("backup written to {file}");
        }
        Some("user-count") => {
            let conn = open_database(&db, &iso(now())).unwrap_or_else(|e| fail(e));
            let n: i64 = conn.query_row("select count(*) from user", [], |r| r.get(0)).unwrap_or_else(|e| fail(e));
            println!("{n}");
        }
        Some("migrate") => {
            let conn = open_database(&db, &iso(now())).unwrap_or_else(|e| fail(e));
            for (name, note) in rail_across_server::data_migrations::run_data_migrations(&conn, now()).unwrap_or_else(|e| fail(e)) {
                println!("data step {name}: {note}");
            }
        }
        Some("cloudjobs") => cloudjobs(&db, &args[1..]),
        _ => {
            fail("usage: rail-across-server serve | create-admin <login> [name] | backup <file> | user-count | migrate | cloudjobs serve|run <job>|exec <run>")
        }
    }
}

fn serve(db: &str, args: &[String]) {
    let (mut local, mut static_root, mut host, mut port) = (
        env("RA_SERVER_LOCAL").as_deref() == Some("1"),
        env("RA_SERVER_STATIC"),
        env("RA_SERVER_HOST"),
        env("RA_SERVER_PORT"),
    );
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--local" => local = true,
            "--static" => static_root = it.next().cloned(),
            "--host" => host = it.next().cloned(),
            "--port" => port = it.next().cloned(),
            other => fail(format!("unknown option for serve: {other}")),
        }
    }
    let conn = open_database(db, &iso(now())).unwrap_or_else(|e| fail(e));
    for (name, note) in rail_across_server::data_migrations::run_data_migrations(&conn, now()).unwrap_or_else(|e| fail(e)) {
        println!("data step {name}: {note}");
    }
    let clouds = (env("RA_SERVER_NO_CLOUDS").as_deref() != Some("1")).then(|| clouds_path(db));
    let mut settings = Settings {
        secure_cookie: env("RA_SERVER_INSECURE_COOKIE").as_deref() != Some("1"),
        cloud_root: clouds.clone(),
        osrd_root: Some(osrd_path(db)),
        test_mode: env("RA_SERVER_TEST").as_deref() == Some("1"),
        ..Settings::default()
    };
    // Every request in the log by default; in test mode only when asked for.
    settings.log = match env("RA_SERVER_LOG") {
        Some(l) => LogLevel::parse(&l).unwrap_or_else(|| fail(format!("RA_SERVER_LOG={l}: one of silent, fatal, error, warn, info, debug, trace"))),
        None if settings.test_mode || local => LogLevel::Silent,
        None => LogLevel::Info,
    };
    settings.local = local;
    settings.static_root = static_root.map(PathBuf::from);
    if let Some(t) = env("RA_SERVER_TRUST_PROXY") {
        settings.trust_proxy = t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    }
    let storage = Storage {
        root: clouds.unwrap_or_else(|| clouds_path(db)),
    };
    let app = Arc::new(App::new(conn, settings));
    let stop = Arc::new(AtomicBool::new(false));
    let jobs_thread = (env("RA_SERVER_JOBS").as_deref() == Some("1")).then(|| {
        let (db, stop) = (db.to_string(), stop.clone());
        let parallel = queue_parallel();
        std::thread::spawn(move || jobs::run_queues(db, storage, parallel, stop, Arc::new(stamp)))
    });
    let sweeper = app.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
            let db = sweeper.db.lock().unwrap();
            let _ = auth::sweep(&sweeper, &db);
        }
    });
    let host = host.unwrap_or_else(|| "127.0.0.1".into());
    let addr = format!("{host}:{}", port.unwrap_or_else(|| "8787".into()));
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| fail(e));
    rt.block_on(rail_across_server::serve::serve(app, &addr, |port| {
        println!("listening on port {port}");
        // The local package's shell reads this line (as from the Node server).
        if local {
            println!("RA_DESKTOP_URL=http://{host}:{port}");
        }
    }))
    .unwrap_or_else(|e| fail(e));
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    if let Some(t) = jobs_thread {
        let _ = t.join();
    }
}

/// How many preparations and how many long runs at once (`RA_CLOUDJOBS_PARALLEL`, `RA_CLOUDRUNS_PARALLEL`).
fn queue_parallel() -> (usize, usize) {
    let n = |name| env(name).and_then(|p| p.parse().ok()).unwrap_or(2);
    (n("RA_CLOUDJOBS_PARALLEL"), n("RA_CLOUDRUNS_PARALLEL"))
}

fn cloudjobs(db: &str, args: &[String]) {
    let storage = Storage { root: clouds_path(db) };
    match args.first().map(String::as_str) {
        Some("serve") => {
            let stop = Arc::new(AtomicBool::new(false));
            let parallel = queue_parallel();
            stamp(&format!("rail-across-server cloudjobs: queue in {db}, clouds in {}", storage.root.display()));
            let s = stop.clone();
            let _ = ctrl_stop(s);
            jobs::run_queues(db.to_string(), storage, parallel, stop, Arc::new(stamp));
        }
        Some("run") => {
            let conn = jobs::open(db).unwrap_or_else(|e| fail(e));
            let id: i64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(0);
            let cloud = rail_across_server::clouds::store::CloudStore::new(&conn, now())
                .job(id)
                .ok()
                .flatten()
                .unwrap_or_else(|| fail(format!("no job {id}")));
            jobs::prepare_cloud(&conn, &storage, &cloud, &stamp)
                .unwrap_or_else(|e| fail(if e == "aborted" { "the cloud was deleted meanwhile".into() } else { e }));
        }
        Some("exec") => {
            let conn = jobs::open(db).unwrap_or_else(|e| fail(e));
            let id: i64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(0);
            let run = rail_across_server::clouds::store::RunStore::new(&conn, now())
                .get(id)
                .ok()
                .flatten()
                .unwrap_or_else(|| fail("run gone"));
            jobs::execute_run(&conn, &storage, &run, &stamp).unwrap_or_else(|e| fail(e));
        }
        _ => {
            eprintln!("usage: rail-across-server cloudjobs serve | run <job> | exec <run>");
            std::process::exit(2);
        }
    }
}

fn ctrl_stop(stop: Arc<AtomicBool>) -> std::io::Result<()> {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            let _ = tokio::signal::ctrl_c().await;
        });
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
    });
    Ok(())
}
