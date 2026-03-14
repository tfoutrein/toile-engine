use std::path::PathBuf;

use toile_mcp::ToileMcpServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let project_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get cwd"));

    eprintln!(
        "Toile MCP Server v{} — project: {}",
        env!("CARGO_PKG_VERSION"),
        project_dir.display()
    );

    let server = ToileMcpServer::new(project_dir);
    let transport = rmcp::transport::io::stdio();
    let service = rmcp::serve_server(server, transport).await?;
    service.waiting().await?;

    Ok(())
}
