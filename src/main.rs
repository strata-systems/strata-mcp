//! MCP server for Strata database.
//!
//! Run with `strata-mcp --db /path/to/data` or `strata-mcp --cache` for in-memory mode.

use clap::Parser;
use stratadb::{AccessMode, OpenOptions, Strata};
use tracing_subscriber::EnvFilter;

mod convert;
mod error;
mod server;
mod session;
mod tools;

use server::McpServer;
use session::McpSession;

/// MCP server for Strata database.
///
/// Exposes Strata database operations as MCP tools for AI agents.
/// Communicates via JSON-RPC 2.0 over stdin/stdout.
#[derive(Parser)]
#[command(name = "strata-mcp")]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the database directory.
    /// Mutually exclusive with --cache.
    #[arg(long, value_name = "PATH")]
    db: Option<String>,

    /// Use an in-memory (cache) database.
    /// Data is not persisted. Mutually exclusive with --db.
    #[arg(long)]
    cache: bool,

    /// Open the database in read-only mode.
    /// Write operations will be rejected.
    #[arg(long)]
    read_only: bool,

    /// Enable automatic text embedding for semantic search.
    /// Model files are downloaded automatically on first use.
    #[arg(long)]
    auto_embed: bool,

    /// Enable debug logging to stderr.
    #[arg(long, short)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();

    // Set up logging
    if args.verbose {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env().add_directive("strata_mcp=debug".parse().unwrap()))
            .with_writer(std::io::stderr)
            .init();
    }

    // Validate arguments
    if args.db.is_some() && args.cache {
        eprintln!("Error: --db and --cache are mutually exclusive");
        std::process::exit(1);
    }

    if args.db.is_none() && !args.cache {
        eprintln!("Error: Must specify either --db <PATH> or --cache");
        std::process::exit(1);
    }

    // Auto-download model files when --auto-embed is requested (best-effort).
    #[cfg(feature = "embed")]
    if args.auto_embed {
        match strata_intelligence::embed::download::ensure_model() {
            Ok(path) => {
                tracing::info!("Model files ready at {}", path.display());
            }
            Err(e) => {
                eprintln!("Warning: failed to download model files: {}", e);
            }
        }
    }

    // Open the database
    let db = if args.cache {
        match Strata::cache() {
            Ok(db) => db,
            Err(e) => {
                eprintln!("Error: Failed to create cache database: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        let path = args.db.as_ref().unwrap();
        let mut opts = OpenOptions::new();
        if args.read_only {
            opts = opts.access_mode(AccessMode::ReadOnly);
        }
        if args.auto_embed {
            opts = opts.auto_embed(true);
        }

        match Strata::open_with(path, opts) {
            Ok(db) => db,
            Err(e) => {
                eprintln!("Error: Failed to open database at '{}': {}", path, e);
                std::process::exit(1);
            }
        }
    };

    // Create session and server
    let session = McpSession::new(db);
    let mut server = McpServer::new(session);

    // Run the server
    if let Err(e) = server.run_sync() {
        eprintln!("Error: Server error: {}", e);
        std::process::exit(1);
    }
}
