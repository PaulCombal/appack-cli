use crate::internal::types::{AppPackDesktopEntry, AppPackLocalSettings, InstalledAppPackEntry, InstalledAppPacks};
use anyhow::{Context, Result, anyhow};
use std::collections::HashSet;
use std::fmt::format;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::path::PathBuf;
use zip::ZipArchive;


/// Weirdly enough this doesn't need escaping. Since we're not running a shell command.
/// https://specifications.freedesktop.org/desktop-entry-spec/1.1/value-types.html
fn process_desktop_entry(
    file_entry_contents: &str,
    desktop_entry: &AppPackDesktopEntry,
    app: &InstalledAppPackEntry,
    settings: &AppPackLocalSettings
) -> Result<String> {
    let icon_dir = settings.get_app_home_dir(app).join("desktop");
    let rdp_args = desktop_entry.rdp_args.replace(" ", "\\s");
    let rdp_args = rdp_args.replace("\\", "\\\\");

    let appack_launch_cmd = format!("appack launch {} {} --version={}", app.id, rdp_args, app.version);

    let final_contents = file_entry_contents.replace("$APPACK_LAUNCH_CMD", &appack_launch_cmd);
    let final_contents = final_contents.replace("$ICON_DIR", icon_dir.to_str().unwrap());

    let final_exec_lines: Vec<_> = final_contents
        .lines()
        .filter(|line| line.starts_with("Exec")).collect();

    if final_exec_lines.len() != 1 {
        return Err(anyhow!("Incorrect amount of Exec entries"));
    }

    let final_exec_lines = final_exec_lines[0];
    let mut final_exec_line_split = final_exec_lines.splitn(2, "=");
    if final_exec_lines.len() == 1 {
        return Err(anyhow!("Malformed exec entry: {}", final_exec_lines));
    }

    let exec_line = final_exec_line_split.nth(1).context("Sanitization error, this should never happen")?.to_string();

    if appack_launch_cmd != exec_line {
        println!("====== DANGER =======");
        println!("A desktop entry was added for this app. It will run the following command on activation:");
        println!();
        println!("{exec_line}");
        println!();
        println!("Your typical command should look like this: '{appack_launch_cmd}'");
        println!();
        println!("If you think this could be malicious, uninstall this appack immediately.");
    }

    Ok(final_contents)
}

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
    let desktop_entries = new_app_entry.desktop_entries.clone().unwrap_or_default();

    if new_app_base_dir.exists() {
        return Err(anyhow!(
            "App directory already exists: {}",
            new_app_base_dir.display()
        ));
    }

    for entry in desktop_entries.iter() {
        archive
            .by_name(&format!("desktop/{}", entry.entry))
            .map_err(|_| anyhow!("Desktop entry '{}' not found in archive", entry.entry))?;

        let entry_file_fullpath = local_settings
            .desktop_entries_dir
            .join(format!("{new_app_version}_{}", entry.entry));
        if entry_file_fullpath.exists() {
            println!("Desktop entry already exists: {}", entry_file_fullpath.display());
            return Err(anyhow!("Desktop entry already exists"));
        }
    }

    std::fs::create_dir_all(&new_app_base_dir.join("desktop"))?;

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

    for entry in desktop_entries.iter() {
        {
            let mut entry_file = archive
                .by_name(&format!("desktop/{}", entry.entry))
                .map_err(|_| anyhow!("Desktop entry '{}' not found in archive", entry.entry))?;

            let entry_fullpath = local_settings
                .desktop_entries_dir
                .join(format!("{new_app_version}_{}", entry.entry));
            let mut outfile =
                File::create(&entry_fullpath).context("Unable to create desktop entry file")?;

            let mut file_content = String::new();
            entry_file.read_to_string(&mut file_content).context("Unable to read entry file")?;

            let file_content = process_desktop_entry(&file_content, entry, &new_app_entry, &local_settings).context("Unable to parse desktop entry")?;

            outfile.write_all(file_content.as_bytes())?;
        }

        {
            let mut entry_file = archive
                .by_name(&format!("desktop/{}", entry.icon))
                .map_err(|_| anyhow!("Desktop entry '{}' not found in archive", entry.icon))?;
            let entry_fullpath = new_app_base_dir.join("desktop").join(&entry.icon);
            let mut outfile = File::create(&entry_fullpath).context("Unable to create desktop entry icon")?;

            io::copy(&mut entry_file, &mut outfile)?;
        }
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
    let forbidden_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|', ' ', '&', ';'];

    if new_app_entry
        .version
        .chars()
        .any(|c| forbidden_chars.contains(&c))
    {
        return Err(anyhow!(
            "Invalid character in version: {}",
            new_app_entry.version
        ));
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
            required_files.push(format!("desktop/{}", entry.entry));
            required_files.push(format!("desktop/{}", entry.icon));
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

    Ok(())
}
