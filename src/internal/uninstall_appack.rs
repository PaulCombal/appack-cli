use crate::internal::types::AppPackLocalSettings;
use anyhow::{Result, anyhow};
use std::fs;
use std::path::PathBuf;

pub fn uninstall_appack(settings: &AppPackLocalSettings, domain_name: &str) -> Result<()> {
    // let installed = get_installed(settings)?;

    // let entry = installed.installed.iter().find(|e| e.id == domain_name);
    //
    // if entry.is_none() {
    //     println!("AppPack not installed: {}", domain_name);
    //     println!("Installed app packs: {installed:?}");
    //     Err(anyhow!("AppPack not installed"))?
    // }
    //
    // // Ugly code
    // let entry = entry.unwrap();
    // let entry_image = PathBuf::from(settings.images_folder.as_ref()).join(entry.image.as_str());
    //
    // // Race condition?
    // if fs::metadata(&entry_image).is_ok() {
    //     fs::remove_file(entry_image)?;
    // }
    //
    // todo!();
    //
    // installed.installed.retain(|e| e.id != domain_name);
    // save_installed(&installed, settings)?;

    Ok(())
}
