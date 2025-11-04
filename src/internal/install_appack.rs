use crate::internal::types::{AppPackIndexFile, AppPackLocalSettings, InstalledAppPackEntry, InstalledAppPacks};
use anyhow::{Result, anyhow, Context};
use std::collections::HashSet;
use std::fmt::format;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use zip::ZipArchive;

fn extract_config(archive: &mut ZipArchive<File>) -> Result<InstalledAppPackEntry> {
    let mut file = archive
        .by_name("AppPack.yaml")
        .map_err(|_| anyhow!("File 'AppPack.yaml' not found in archive"))?;

    let mut buffer = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut buffer)
        .map_err(|e| anyhow!("Unable to read config file: {e}"))?;
    serde_yaml::from_slice(&buffer).map_err(|e| anyhow!("Invalid YAML file: {e:?}"))
}

fn extract_files(
    archive: &mut ZipArchive<File>,
    new_app_entry: &InstalledAppPackEntry,
    local_settings: &AppPackLocalSettings,
) -> Result<()> {
    let image_filename = new_app_entry.image.as_str();
    let new_app_version = new_app_entry.version.as_str();
    let new_app_base_dir = local_settings.get_app_home_dir(new_app_entry);

    if new_app_base_dir.exists() {
        return Err(anyhow!(
            "App directory already exists: {}",
            new_app_base_dir.display()
        ));
    }


    for entry in new_app_entry.desktop_entries.clone().unwrap() {
        archive
            .by_name(&format!("desktop/{entry}"))
            .map_err(|_| anyhow!("Desktop entry '{}' not found in archive", entry))?;

        let entry_fullpath = local_settings.desktop_entries_dir.join(format!("{new_app_version}_{entry}"));
        if entry_fullpath.exists() {
            println!("Desktop entry already exists: {}", entry_fullpath.display());
            return Err(anyhow!("Desktop entry already exists"));
        }
    }

    std::fs::create_dir_all(&new_app_base_dir)?;

    println!("Extracting app data.. This may take a while.");

    {
        let mut image_file = archive
            .by_name(image_filename)
            .map_err(|_| anyhow!("Image '{}' not found in archive", image_filename))?;
        let image_fullpath = new_app_base_dir.join(image_filename);

        let mut outfile = File::create(&image_fullpath)
            .map_err(|e| anyhow!("Unable to create file {}: {e}", image_fullpath.display()))?;
        io::copy(&mut image_file, &mut outfile)?;
    }

    println!("Extracting desktop entries..");

    {
        for entry in new_app_entry.desktop_entries.clone().unwrap() {
            let mut entry_file = archive
                .by_name(&format!("desktop/{entry}"))
                .map_err(|_| anyhow!("Desktop entry '{}' not found in archive", entry))?;

            let entry_fullpath = local_settings.desktop_entries_dir.join(format!("{new_app_version}_{entry}"));
            let mut outfile = File::create(&entry_fullpath)
                .context("Unable to create desktop entry file")?;

            io::copy(&mut entry_file, &mut outfile)?;
        }

        // TODO: extract icons
    }

    Ok(())
}

/// Checks that the following files are present:
/// * image file
/// * desktop entries
fn check_valid_app_pack(
    archive: &mut ZipArchive<File>,
    new_app_entry: &InstalledAppPackEntry,
    installed: &InstalledAppPacks,
) -> Result<()> {
    // TODO: deduplicate with AppPackIndexFile::new
    let forbidden_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|', ' '];

    if new_app_entry.version.chars().any(|c| forbidden_chars.contains(&c)) {
        return Err(anyhow!("Invalid character in version: {}", new_app_entry.version));
    }

    for entry in installed.installed.iter() {
        if entry.id == new_app_entry.id {
            println!("AppPack already installed: {}", entry.id);
            println!("Installed version: {}", entry.version);
            println!("File version: {}", new_app_entry.version);
            return Err(anyhow!("AppPack already installed"));
        }
    }

    let mut required_files = [new_app_entry.image.clone()].to_vec();
    if let Some(entries) = new_app_entry.desktop_entries.clone() {
        for entry in entries {
            required_files.push(format!("desktop/{entry}"));
        }
    }

    // Collect all file names present in the archive into a HashSet
    let mut present_files = HashSet::new();
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            present_files.insert(file.name().to_string());
        }
    }

    // Check which required files are missing
    let missing_files: Vec<_> = required_files
        .iter()
        .filter(|&f| !present_files.contains(f))
        .collect();

    if !missing_files.is_empty() {
        return Err(anyhow!("Missing files: {:?}", missing_files));
    }

    Ok(())
}

// Todo: make this atomic as in if it fails then the halfway done files should be removed
pub fn install_appack(file_path: PathBuf, settings: AppPackLocalSettings) -> Result<()> {
    let file =
        File::open(&file_path).map_err(|e| anyhow!("Unable to open file {file_path:?}: {e}"))?;
    let mut archive =
        ZipArchive::new(file).map_err(|e| anyhow!("Unable to file as zip archive: {e}"))?;

    settings.check_ok()?;
    let new_app_entry = extract_config(&mut archive)?;
    let mut installed_apps = settings.get_installed()?;
    check_valid_app_pack(&mut archive, &new_app_entry, &installed_apps)?;
    extract_files(&mut archive, &new_app_entry, &settings)?;

    // 2. Add to installed list
    installed_apps.installed.push(new_app_entry.clone());
    settings.save_installed(installed_apps)?;

    // TODO: Setup desktop integration
    // For now, we can just print the desktop files content and ask user to copy it manually
    // https://forum.snapcraft.io/t/managing-desktop-entries-at-runtime/49149



    Ok(())
}
