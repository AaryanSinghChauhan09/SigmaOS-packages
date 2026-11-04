//! SigmaOS-packages — Package management component for SigmaOS
#![allow(dead_code)]

pub mod cache;
pub mod config;
pub mod error;
pub mod installer;
pub mod package;
pub mod service;

pub use cache::PackageCache;
pub use config::Config;
pub use error::{ComponentError, Result};
pub use installer::{InstallStep, InstallTransaction, PackageInstaller};
pub use package::{Package, PackageRepository, Version};
pub use service::Service;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
