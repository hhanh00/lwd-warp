use anyhow::Result;
use figment::{providers::{Format as _, Toml}, Figment};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use lazy_static::lazy_static;
use serde::Deserialize;

pub mod server;
pub mod cli;

#[derive(Deserialize)]
pub struct Config {
    pub db_path: String,
}

lazy_static! {
    pub static ref CONFIG: Config = init_config();
    pub static ref POOL: Pool<SqliteConnectionManager> = init_pool().unwrap();
}

pub fn init_config() -> Config {
    let config: Config = Figment::new()
        .merge(Toml::file("App.toml"))
        .extract()
        .unwrap();
    config
}

pub fn init_pool() -> Result<Pool<SqliteConnectionManager>> {
    let manager = r2d2_sqlite::SqliteConnectionManager::file(&CONFIG.db_path);
    let pool = r2d2::Pool::new(manager)?;
    Ok(pool)
}
