use anyhow::Result;
use sigmaos_package_component::{
    Config, Service, Package, PackageRepository, Version, PackageInstaller, PackageCache
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_lifecycle_aaa() -> Result<()> {
        let config = Config::default();
        let mut service = Service::new(config);
        assert!(!service.is_running());
        service.start()?;
        assert!(service.is_running());
        service.stop()?;
        assert!(!service.is_running());
        Ok(())
    }

    #[test]
    fn test_package_repository_aaa() {
        let mut repo = PackageRepository::new();
        let pkg = Package::new("sigma-core".to_string(), Version::new(1, 0, 0), "Core utilities".to_string());
        repo.add_package(pkg);
        assert_eq!(repo.list_packages().len(), 1);
        assert!(repo.get_package("sigma-core").is_some());
    }

    #[test]
    fn test_package_cache_aaa() {
        let mut cache = PackageCache::default();
        assert!(cache.get("non-existent").is_none());
    }

    #[test]
    fn test_package_installer_aaa() -> Result<(), &'static str> {
        let mut installer = PackageInstaller::default();
        let mut transaction = installer.create_transaction("sigma-core".to_string());
        installer.install(&mut transaction)
    }
}
