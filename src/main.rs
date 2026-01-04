use actix_files as fs_serve;
use actix_web::{middleware, web, App, HttpServer};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;

mod generator;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initial build
    println!("🚀 Building blog...");
    generator::build_blog()?;
    println!("✅ Blog built successfully!\n");

    // Create a channel for file change notifications
    let (tx, rx) = mpsc::channel();

    // Spawn watcher thread
    std::thread::spawn(move || {
        if let Err(e) = setup_watcher(tx) {
            eprintln!("Failed to setup file watcher: {}", e);
        }
    });

    println!("Starting web server...");
    println!("Visit: http://localhost:8000");


    // Spawn a thread to handle file change events
    std::thread::spawn(move || {
        for _ in rx.iter() {
            println!("\n📝 Changes detected! Rebuilding blog...");
            if let Err(e) = generator::build_blog() {
                println!("❌ Error rebuilding blog: {}", e);
            } else {
                println!("✅ Blog rebuilt successfully!");
            }
        }
    });

    println!("Server started! Ready to serve your blog.\n");
    HttpServer::new(|| {
        App::new().wrap(middleware::NormalizePath::trim()).service(
            fs_serve::Files::new("/", "./output")
                .index_file("index.html")
                .default_handler(
                    web::route()
                        .to(|| async { actix_web::HttpResponse::NotFound().body("404 Not Found") }),
                ),
        )
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}

fn setup_watcher(tx: mpsc::Sender<()>) -> notify::Result<()> {
    let (watch_tx, watch_rx) = mpsc::channel();

    let mut watcher: RecommendedWatcher = Watcher::new(
        move |res: Result<notify::Event, notify::Error>| match res {
            Ok(event) => {
                if matches!(event.kind, notify::EventKind::Modify(_)) {
                    let _ = watch_tx.send(());
                }
            }
            Err(e) => eprintln!("Watch error: {:?}", e),
        },
        notify::Config::default(),
    )?;

    watcher.watch(Path::new("posts"), RecursiveMode::Recursive)?;

    // Keep watcher alive and relay events
    for _ in watch_rx.iter() {
        let _ = tx.send(());
    }

    Ok(())
}
