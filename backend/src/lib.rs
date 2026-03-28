pub mod ai;
pub mod api;
pub mod claude;
pub mod db;
pub mod logging;
pub mod models;
pub mod static_files;
pub mod ws;

pub type Result<T> = std::result::Result<T, anyhow::Error>;
