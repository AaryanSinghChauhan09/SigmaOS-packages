#[derive(Debug, Clone)]
pub struct PackageMeta {
    pub name: String,
    pub version: String,
    pub sha256: String,
}

#[derive(Debug, Default)]
pub struct PackageManager {
    installed: Vec<PackageMeta>,
}

impl PackageManager {
    pub fn new() -> Self { Self::default() }
    pub fn install(&mut self, meta: PackageMeta) { self.installed.push(meta); }
    pub fn is_installed(&self, name: &str) -> bool { self.installed.iter().any(|p| p.name == name) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_package_manager_install_and_query() {
        let mut pm = PackageManager::new();
        pm.install(PackageMeta { name: "neovim".into(), version: "0.10.0".into(), sha256: "abc".into() });
        assert!(pm.is_installed("neovim"));
        assert!(!pm.is_installed("emacs"));
    }
}
