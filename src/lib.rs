#[macro_use]
extern crate diesel;
extern crate dotenv;

pub mod schema;

use std::env;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dotenv::dotenv;

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();
    let database_url: String = env::var("DATABASE_URL")
        .expect("DATABASE_URL ");
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|| panic!("Error connecting to {}", database_url))
}

#[derive(Queryable)]
pub struct KnownVersion {
    pub value: u32,
    pub snapshot: bool
}
struct VersionedName {
    version: u32,
    remapped_name: String,
}

pub struct DatabaseConnection(SqliteConnection);
impl DatabaseConnection {
    pub fn known_versions(&self) -> QueryResult<Vec<KnownVersion>> {
        use schema::known_versions::dsl::*;
        known_versions.select((value, snapshot)).load(&self.0)
    }
    pub fn is_known_version(&self, version: KnownVersion) -> QueryResult<bool> {
        use schema::known_versions::dsl::*;
        Ok(known_versions
            .filter(value.eq(version.value))
            .filter(snapshot.eq(version.snapshot)).limit(1)
            .count().get_result(&self.0)? > 0)
    }
    pub fn load_all_renamed(&self, original: &str) -> QueryResult<bool> {
        use schema::{snapshot_names, versio}
    }
}