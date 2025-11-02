use crate::internal::types::AppPackLocalSettings;
use anyhow::Result;

pub fn list_installed(settings: AppPackLocalSettings) -> Result<()> {
    let installed_apps = settings.get_installed()?;
    println!("Installed app packs:");
    println!("{:#?}", installed_apps); // Todo impl display or something

    Ok(())
}