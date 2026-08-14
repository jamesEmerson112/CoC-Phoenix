//! CoC Phoenix - character signup backend.
//!
//! Architecture (from `docs/design/01-signup-wizard-spec.md`):
//!
//! * **Answers as inputs.** Postgres stores the player's answers ([`model::Answers`]) and
//!   nothing else while a character is a draft.
//! * **The sheet is a projection.** [`projection::project`] recomputes it on every read from
//!   the answers plus the content pack. Pure, deterministic, database-free.
//! * **Finalize freezes.** The projected sheet is snapshotted into the row and answers stop
//!   accepting mutations.
//!
//! The rules modules ([`allocator`], [`derived`], [`dice`]) never touch I/O, so they are unit
//! tested without a database.

pub mod allocator;
pub mod config;
pub mod content;
pub mod db;
pub mod derived;
pub mod dice;
pub mod error;
pub mod extract;
pub mod model;
pub mod projection;
pub mod routes;
pub mod seed;

use content::Catalog;
use sqlx::PgPool;
use std::sync::Arc;

/// Shared handler state: the connection pool and the immutable content catalogue.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub catalog: Arc<Catalog>,
}

impl AppState {
    pub fn new(pool: PgPool, catalog: Catalog) -> AppState {
        AppState {
            pool,
            catalog: Arc::new(catalog),
        }
    }
}
