//! Dumps the bundled content pack as idempotent upsert SQL, for hand-applying or reviewing.
//!
//! Run from `backend/`:
//!
//! ```text
//! cargo run --bin gen-seed                 # print to stdout
//! cargo run --bin gen-seed content/starter_pack.sql
//! ```
//!
//! This is **not** a migration and must never be written into `migrations/`. The server applies
//! the same SQL itself on every boot (`db::sync_content_pack`), so `content/starter_pack.json`
//! is the only file anyone edits. Adding the output to the migration chain would checksum-freeze
//! the content again — the exact failure this command used to cause.

use std::path::PathBuf;

fn main() -> std::io::Result<()> {
    let sql = coc_phoenix::seed::starter_seed_sql();

    match std::env::args().nth(1) {
        None => {
            print!("{sql}");
            Ok(())
        }
        Some(arg) => {
            let path = PathBuf::from(arg);
            if path.components().any(|c| c.as_os_str() == "migrations") {
                eprintln!(
                    "refusing to write into migrations/: bundled content is data, applied at \
                     boot by db::sync_content_pack. See backend/README.md 'Content'."
                );
                std::process::exit(2);
            }
            std::fs::write(&path, sql.as_bytes())?;
            println!("wrote {}", path.display());
            Ok(())
        }
    }
}
