use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use lazy_static::lazy_static;

pub mod server;

lazy_static! {
    pub static ref POOL: Pool<SqliteConnectionManager> = init_pool().unwrap();
}

pub fn init_pool() -> Result<Pool<SqliteConnectionManager>> {
    let _ = dotenv::dotenv();
    let db_path = dotenv::var("DB_PATH")?;
    let manager = r2d2_sqlite::SqliteConnectionManager::file(db_path);
    let pool = r2d2::Pool::new(manager)?;
    Ok(pool)
}
