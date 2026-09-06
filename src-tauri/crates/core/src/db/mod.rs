mod connection;
mod error;
mod migrations;
pub mod queries;

use std::path::Path;

use rusqlite::Connection;

pub use connection::open;
pub use error::DbError;
pub use migrations::{current_version, latest_version, migrate, Migration, MIGRATIONS};

pub fn open_and_migrate(path: &Path) -> Result<Connection, DbError> {
    let mut conn = open(path)?;
    migrate(&mut conn)?;
    Ok(conn)
}
