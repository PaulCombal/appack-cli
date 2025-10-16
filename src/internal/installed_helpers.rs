use crate::internal::types::{AppPackLocalSettings, InstalledAppPacks};
use anyhow::{Result, anyhow};

pub fn get_installed(settings: &AppPackLocalSettings) -> Result<InstalledAppPacks> {
    let installed_filepath = settings.installed_file.as_ref();

    let installed_app_packs: InstalledAppPacks = if installed_filepath.exists() {
        let content = std::fs::read_to_string(installed_filepath).map_err(|e| {
            anyhow!(
                "Failed to read installed file {}: {}",
                installed_filepath.display(),
                e
            )
        })?;
        serde_yaml::from_str(&content).map_err(|e| {
            anyhow!(
                "Failed to parse installed file {}: {}",
                installed_filepath.display(),
                e
            )
        })?
    } else {
        InstalledAppPacks {
            installed: Vec::new(),
        }
    };

    Ok(installed_app_packs)
}

pub fn save_installed(
    installed_app_packs: &InstalledAppPacks,
    settings: &AppPackLocalSettings,
) -> Result<()> {
    let installed_filepath = settings.installed_file.as_ref();
    let content = serde_yaml::to_string(&installed_app_packs)
        .map_err(|e| anyhow!("Failed to serialize installed app packs: {}", e))?;
    std::fs::write(installed_filepath, content).map_err(|e| {
        anyhow!(
            "Failed to write installed file {}: {}",
            installed_filepath.display(),
            e
        )
    })?;
    Ok(())
}
