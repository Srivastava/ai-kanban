pub mod api;
pub mod db;
pub mod logging;
pub mod models;

pub type Result<T> = std::result::Result<T, anyhow::Error>;
