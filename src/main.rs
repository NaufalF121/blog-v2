use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer};
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
        App::new()
            .wrap(middleware::NormalizePath::trim())
            .default_service(web::route().to(handle_request))
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}

async fn handle_request(req: HttpRequest) -> HttpResponse {
    let path = req.path();
    let mut file_path = path.trim_start_matches('/').to_string();

    // If path is empty or just "/", serve index.html
    if file_path.is_empty() {
        file_path = "index.html".to_string();
    } else if !file_path.ends_with(".html") && !file_path.contains('.') {
        // If no extension, try to append .html
        file_path.push_str(".html");
    }

    let full_path = format!("./output/{}", file_path);

    // Try to serve the file
    match std::fs::read(&full_path) {
        Ok(content) => {
            let content_type = if full_path.ends_with(".css") {
                "text/css"
            } else if full_path.ends_with(".js") {
                "application/javascript"
            } else if full_path.ends_with(".png") {
                "image/png"
            } else if full_path.ends_with(".jpg") || full_path.ends_with(".jpeg") {
                "image/jpeg"
            } else if full_path.ends_with(".gif") {
                "image/gif"
            } else if full_path.ends_with(".svg") {
                "image/svg+xml"
            } else {
                "text/html; charset=utf-8"
            };

            HttpResponse::Ok().content_type(content_type).body(content)
        }
        Err(_) => {
            // If file not found, return 404
            HttpResponse::NotFound().body("404 Not Found")
        }
    }
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
