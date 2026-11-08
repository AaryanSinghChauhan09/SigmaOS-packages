use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageMeta {
    pub name: String,
    pub version: String,
    pub sha256: String,
}

impl PackageMeta {
    /// Validate metadata syntax. This checks digest shape, not artifact bytes.
    pub fn validate(&self) -> Result<(), PackageError> {
        if self.name.trim().is_empty() || self.name.chars().any(char::is_whitespace) {
            return Err(PackageError::InvalidName);
        }
        if self.version.trim().is_empty() || self.version.chars().any(char::is_whitespace) {
            return Err(PackageError::InvalidVersion);
        }
        if self.sha256.len() != 64 || !self.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(PackageError::InvalidSha256);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageError {
    InvalidName,
    InvalidVersion,
    InvalidSha256,
    AlreadyInstalled(String),
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName => write!(f, "package name must be nonempty and contain no whitespace"),
            Self::InvalidVersion => write!(f, "package version must be nonempty and contain no whitespace"),
            Self::InvalidSha256 => write!(f, "SHA-256 must contain exactly 64 hexadecimal characters"),
            Self::AlreadyInstalled(name) => write!(f, "package is already recorded as installed: {name}"),
        }
    }
}

impl std::error::Error for PackageError {}

#[derive(Debug, Default)]
pub struct PackageManager {
    installed: BTreeMap<String, PackageMeta>,
}

impl PackageManager {
    pub fn new() -> Self { Self::default() }

    /// Record metadata for an installed package; this does not install files.
    pub fn record_installed(&mut self, meta: PackageMeta) -> Result<(), PackageError> {
        meta.validate()?;
        if self.installed.contains_key(&meta.name) {
            return Err(PackageError::AlreadyInstalled(meta.name));
        }
        self.installed.insert(meta.name.clone(), meta);
        Ok(())
    }

    /// Compatibility alias; this records inventory metadata and performs no installation.
    #[deprecated(note = "records metadata only; use record_installed")]
    pub fn install(&mut self, meta: PackageMeta) -> Result<(), PackageError> {
        self.record_installed(meta)
    }

    pub fn get(&self, name: &str) -> Option<&PackageMeta> {
        self.installed.get(name)
    }

    pub fn is_installed(&self, name: &str) -> bool {
        self.installed.contains_key(name)
    }

    /// List entries in stable package-name order.
    pub fn list_installed(&self) -> impl Iterator<Item = &PackageMeta> {
        self.installed.values()
    }

    pub fn remove_record(&mut self, name: &str) -> Option<PackageMeta> {
        self.installed.remove(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_package_manager_install_and_query() {
        let mut pm = PackageManager::new();
        pm.record_installed(PackageMeta {
            name: "neovim".into(),
            version: "0.10.0".into(),
            sha256: "a".repeat(64),
        }).unwrap();
        assert!(pm.is_installed("neovim"));
        assert!(!pm.is_installed("emacs"));
    }
}
