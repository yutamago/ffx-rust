//! Development server for the app's UI: `POST /api` with `{"cmd", "args"}`
//! runs a command (answer `{"ok": value}` or `{"error": message}`), other
//! requests serve files from the built UI.
//!
//! `cargo run -p ffx-studio --example serve -- [port] [ui dir]`
//! (defaults 8787, `app/ui/dist`); `pnpm dev` in `app/ui` proxies `/api`
//! here.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;

use ffx_studio::Studio;
use serde_json::{Value, json};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let port = args.get(1).map_or("8787", String::as_str);
    let ui = PathBuf::from(args.get(2).map_or("app/ui/dist", String::as_str));
    let config = std::env::var_os("HOME")
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join(".config/ffx-studio/config.json");
    let mut studio = Studio::new(config, None);
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).expect("bind");
    eprintln!("FFX Studio dev server on http://127.0.0.1:{port}");
    for stream in listener.incoming().flatten() {
        let mut reader = BufReader::new(stream.try_clone().expect("clone"));
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let (method, target) = (
            parts.next().unwrap_or(""),
            parts.next().unwrap_or("/").to_owned(),
        );
        let mut length = 0;
        loop {
            let mut h = String::new();
            if reader.read_line(&mut h).is_err() || h.trim().is_empty() {
                break;
            }
            if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                length = v.trim().parse().unwrap_or(0);
            }
        }
        let mut body = vec![0; length];
        let _ = reader.read_exact(&mut body);
        let (status, kind, out) = if method == "POST" && target == "/api" {
            let req: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
            let cmd = req["cmd"].as_str().unwrap_or("");
            let answer = match studio.call(cmd, &req["args"]) {
                Ok(v) => json!({"ok": v}),
                Err(e) => json!({"error": format!("{e:#}")}),
            };
            (
                "200 OK",
                "application/json",
                serde_json::to_vec(&answer).unwrap_or_default(),
            )
        } else {
            let rel = target
                .split('?')
                .next()
                .unwrap_or("/")
                .trim_start_matches('/');
            let rel = if rel.is_empty() { "index.html" } else { rel };
            let file = ui.join(rel);
            match (!rel.contains(".."))
                .then(|| std::fs::read(&file).ok())
                .flatten()
            {
                Some(b) => {
                    let kind = match file.extension().and_then(|e| e.to_str()) {
                        Some("html") => "text/html",
                        Some("js") => "text/javascript",
                        Some("css") => "text/css",
                        Some("svg") => "image/svg+xml",
                        Some("png") => "image/png",
                        _ => "application/octet-stream",
                    };
                    ("200 OK", kind, b)
                }
                None => ("404 Not Found", "text/plain", b"not found".to_vec()),
            }
        };
        let mut stream = stream;
        let _ = write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            out.len()
        );
        let _ = stream.write_all(&out);
    }
}
