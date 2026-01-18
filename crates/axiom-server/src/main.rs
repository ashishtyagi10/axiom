//! Axiom Server Binary
//!
//! Standalone entry point - delegates to lib.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    axiom_server::run_server(port).await
}
