//! Quick test for workspace functionality
//!
//! Run with: cargo run --example test_workspace

use axiom_core::{AxiomConfig, WorkspaceManager};
use std::path::PathBuf;

fn main() -> axiom_core::Result<()> {
    println!("=== Workspace Manager Test ===\n");

    // Create manager with default config
    let config = AxiomConfig::default();
    let manager = WorkspaceManager::new(config)?;

    // List existing workspaces
    println!("Existing workspaces:");
    let workspaces = manager.list_workspaces();
    if workspaces.is_empty() {
        println!("  (none)\n");
    } else {
        for ws in &workspaces {
            println!(
                "  - {} ({}) [{}]",
                ws.name,
                ws.path.display(),
                if ws.is_active { "ACTIVE" } else { "inactive" }
            );
        }
        println!();
    }

    // Create a test workspace pointing to current directory
    let cwd = std::env::current_dir()?;
    println!("Creating workspace 'axiom-dev' at: {}", cwd.display());

    match manager.create_workspace("axiom-dev", cwd.clone()) {
        Ok(ws) => {
            println!("  Created workspace: {} ({})\n", ws.name, ws.id);

            // Activate it
            println!("Activating workspace...");
            let service = manager.activate_workspace(ws.id)?;
            println!("  Service created for: {}\n", service.cwd().display());

            // List workspaces again
            println!("Workspaces after creation:");
            for ws in manager.list_workspaces() {
                println!(
                    "  - {} [{}]",
                    ws.name,
                    if ws.is_active { "ACTIVE" } else { "inactive" }
                );
            }
            println!();

            // Clean up - delete the test workspace
            println!("Cleaning up (deleting test workspace)...");
            manager.delete_workspace(ws.id)?;
            println!("  Deleted.\n");
        }
        Err(e) => {
            println!("  Workspace may already exist: {}\n", e);
        }
    }

    // Show final state
    println!("Final workspace list:");
    let final_list = manager.list_workspaces();
    if final_list.is_empty() {
        println!("  (none)");
    } else {
        for ws in final_list {
            println!("  - {} ({})", ws.name, ws.path.display());
        }
    }

    println!("\n=== Test Complete ===");
    Ok(())
}
