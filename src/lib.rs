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
    DuplicateCatalogEntry(String),
    InvalidDependencyName,
    DuplicateDependency(String),
    MissingPackage(String),
    MissingDependency { package: String, dependency: String },
    DependencyCycle(Vec<String>),
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName => write!(f, "package name must be nonempty and contain no whitespace"),
            Self::InvalidVersion => write!(f, "package version must be nonempty and contain no whitespace"),
            Self::InvalidSha256 => write!(f, "SHA-256 must contain exactly 64 hexadecimal characters"),
            Self::AlreadyInstalled(name) => write!(f, "package is already recorded as installed: {name}"),
            Self::DuplicateCatalogEntry(name) => write!(f, "package is already in the catalog: {name}"),
            Self::InvalidDependencyName => write!(f, "dependency name must be nonempty and contain no whitespace"),
            Self::DuplicateDependency(name) => write!(f, "dependency is listed more than once: {name}"),
            Self::MissingPackage(name) => write!(f, "package is not in the catalog: {name}"),
            Self::MissingDependency { package, dependency } => {
                write!(f, "package {package} depends on missing package {dependency}")
            }
            Self::DependencyCycle(path) => write!(f, "dependency cycle: {}", path.join(" -> ")),
        }
    }
}

impl std::error::Error for PackageError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageCatalogEntry {
    pub metadata: PackageMeta,
    pub dependencies: Vec<String>,
}

impl PackageCatalogEntry {
    pub fn new(metadata: PackageMeta, dependencies: Vec<String>) -> Self {
        Self { metadata, dependencies }
    }
}

/// Available package metadata and dependency relationships, separate from installed state.
#[derive(Debug, Default)]
pub struct PackageCatalog {
    entries: BTreeMap<String, PackageCatalogEntry>,
}

impl PackageCatalog {
    pub fn new() -> Self { Self::default() }

    pub fn add(&mut self, entry: PackageCatalogEntry) -> Result<(), PackageError> {
        entry.metadata.validate()?;
        if self.entries.contains_key(&entry.metadata.name) {
            return Err(PackageError::DuplicateCatalogEntry(entry.metadata.name));
        }
        let mut seen = BTreeMap::new();
        for dependency in &entry.dependencies {
            if dependency.trim().is_empty() || dependency.chars().any(char::is_whitespace) {
                return Err(PackageError::InvalidDependencyName);
            }
            if seen.insert(dependency, ()).is_some() {
                return Err(PackageError::DuplicateDependency(dependency.to_string()));
            }
        }
        self.entries.insert(entry.metadata.name.clone(), entry);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&PackageCatalogEntry> {
        self.entries.get(name)
    }

    /// Return dependency-first order, rejecting missing nodes and cycles.
    pub fn resolve_dependency_order(&self, root: &str) -> Result<Vec<String>, PackageError> {
        if !self.entries.contains_key(root) {
            return Err(PackageError::MissingPackage(root.to_string()));
        }

        // 1 = visiting, 2 = fully resolved. Iterative DFS avoids recursion on
        // untrusted repository metadata and preserves declared dependency order.
        let mut states = BTreeMap::<String, u8>::new();
        let mut stack = vec![(root.to_string(), 0usize)];
        let mut order = Vec::new();
        states.insert(root.to_string(), 1);

        while stack.last().is_some() {
            let next = {
                let Some((name, index)) = stack.last_mut() else {
                    break;
                };
                let Some(entry) = self.entries.get(name) else {
                    return Err(PackageError::MissingPackage(name.clone()));
                };
                if *index < entry.dependencies.len() {
                    let dependency = entry.dependencies[*index].clone();
                    *index += 1;
                    Some((name.clone(), dependency))
                } else {
                    None
                }
            };

            if let Some((package, dependency)) = next {
                if !self.entries.contains_key(&dependency) {
                    return Err(PackageError::MissingDependency { package, dependency });
                }
                match states.get(&dependency).copied() {
                    Some(1) => {
                        let Some(cycle_start) = stack.iter().position(|(name, _)| name == &dependency) else {
                            return Err(PackageError::DependencyCycle(vec![dependency]));
                        };
                        let mut cycle: Vec<String> = stack[cycle_start..]
                            .iter()
                            .map(|(name, _)| name.clone())
                            .collect();
                        cycle.push(dependency);
                        return Err(PackageError::DependencyCycle(cycle));
                    }
                    Some(2) => {}
                    _ => {
                        states.insert(dependency.clone(), 1);
                        stack.push((dependency, 0));
                    }
                }
            } else {
                if let Some((name, _)) = stack.pop() {
                    states.insert(name.clone(), 2);
                    order.push(name);
                }
            }
        }

        Ok(order)
    }
}

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
