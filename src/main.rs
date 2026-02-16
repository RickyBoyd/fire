use std::env;

#[tokio::main]
async fn main() {
    let raw_args: Vec<String> = env::args().collect();
    if raw_args.get(1).map(|s| s.as_str()) == Some("serve") {
        let port = raw_args
            .get(2)
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(8080);
        if let Err(e) = fire::api::run_http_server(port).await {
            eprintln!("Server error: {e}");
            std::process::exit(1);
        }
        return;
    }

    eprintln!("Usage: cargo run -- serve [port]");
    std::process::exit(1);
}
