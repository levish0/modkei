use std::net::TcpListener;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use anyhow::{Context, Result};
use modkei_core::GraphData;
use rust_embed::RustEmbed;
use tiny_http::{Header, Response, Server};

#[derive(RustEmbed)]
#[folder = "static-report/"]
struct Asset;

pub fn generate_and_serve(graph: &GraphData, no_open: bool) -> Result<()> {
    let port = available_port()?;
    let server = Server::http(format!("127.0.0.1:{port}"))
        .map_err(|e| anyhow::anyhow!("Failed to bind server: {}", e))?;

    let url = format!("http://127.0.0.1:{port}/");

    if !no_open {
        open::that(&url).with_context(|| format!("failed to open {url}"))?;
    }
    eprintln!("Serving report at {url}  (press Ctrl+C to stop)");

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || r.store(false, Ordering::SeqCst))
        .context("failed to set Ctrl+C handler")?;

    let graph_json = serde_json::to_vec_pretty(graph)?;

    while running.load(Ordering::SeqCst) {
        if let Ok(Some(request)) = server.recv_timeout(Duration::from_millis(200)) {
            let path = request.url();

            if path == "/api/graph-data.json" {
                let mut response = Response::from_data(graph_json.clone());
                response.add_header(
                    Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
                );
                let _ = request.respond(response);
                continue;
            }

            let mut asset_path = path.trim_start_matches('/');
            if asset_path.is_empty() {
                asset_path = "index.html";
            }

            match Asset::get(asset_path) {
                Some(content) => {
                    let mut response = Response::from_data(content.data.into_owned());
                    let mime = mime_guess::from_path(asset_path).first_or_octet_stream();
                    response.add_header(
                        Header::from_bytes(&b"Content-Type"[..], mime.as_ref().as_bytes()).unwrap(),
                    );
                    let _ = request.respond(response);
                }
                None => {
                    // Fallback to index.html for SPA routing
                    if let Some(content) = Asset::get("index.html") {
                        let mut response = Response::from_data(content.data.into_owned());
                        response.add_header(
                            Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap(),
                        );
                        let _ = request.respond(response);
                    } else {
                        let _ = request
                            .respond(Response::from_string("Not Found").with_status_code(404));
                    }
                }
            }
        }
    }

    Ok(())
}

fn available_port() -> Result<u16> {
    let listener =
        TcpListener::bind(("127.0.0.1", 0)).context("failed to reserve preview server port")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}
