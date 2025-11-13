use crate::internal::helpers::get_app_installed;
use crate::internal::types::AppPackLocalSettings;
use anyhow::Result;
use anyhow::{Context, anyhow};
use std::process::Command;

pub fn reset(settings: &AppPackLocalSettings, id: String, version: Option<&str>) -> Result<()> {
    let app_installed =
        get_app_installed(settings, &id, version).context("Failed to get installed AppPack")?;
    let app_installed_home = settings.get_app_home_dir(&app_installed);
    let image_name = app_installed.image.clone();
    let image_path = app_installed_home.join(image_name);

    let result = Command::new("qemu-img")
        .arg("snapshot")
        .arg("-d")
        .arg("appack-onclose")
        .arg(&image_path)
        .status()
        .context("Failed to delete snapshot 'appack-onclose'")?;

    if !result.success() {
        return Err(anyhow!(
            "Failed to reset the AppPack. Make sure the AppPack is NOT running."
        ))
        .context("Failed to delete snapshot 'appack-onclose'");
    }

    Ok(())
}
