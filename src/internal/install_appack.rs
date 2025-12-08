// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2025 Paul <abonnementspaul (at) gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::types::AppDesktopEntry;
use crate::types::app_build_config::AppBuildConfig;
use crate::types::app_installed::{InstalledAppPackEntry, InstalledAppPacks};
use crate::types::local_settings::AppPackLocalSettings;
use anyhow::{Context, Result, anyhow};
use std::collections::HashSet;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Duration;
use zip::ZipArchive;

/// Weirdly enough this doesn't need escaping. To confirm, I escape anyway.
/// https://specifications.freedesktop.org/desktop-entry-spec/1.1/value-types.html
fn process_desktop_entry(
    file_entry_contents: &str,
    desktop_entry: &AppDesktopEntry,
    app: &InstalledAppPackEntry,
    settings: &AppPackLocalSettings,
) -> Result<String> {
    let icon_dir = settings.get_app_home_dir(app).join("desktop");

    let appack_launch_cmd = if desktop_entry.rdp_args.is_empty() {
        format!("appack launch {} --version={}", app.id, app.version)
    } else {
        let escaped_rdp_args = desktop_entry
            .rdp_args
            .replace('\\', "\\\\")
            .replace('"', "\\\"");

        format!(
            "appack launch {} \"{}\" --version={}",
            app.id, escaped_rdp_args, app.version
        )
    };

    let final_contents = file_entry_contents
        .replace("$APPACK_LAUNCH_CMD", &appack_launch_cmd)
        .replace("$ICON_DIR", icon_dir.to_str().unwrap())
        .replace(
            "$ICON_FULL_PATH",
            icon_dir.join(&desktop_entry.icon).to_str().unwrap(),
        );

    println!("Installed desktop entry with supposed exec line: `{appack_launch_cmd}`");

    let final_exec_lines: Vec<_> = final_contents
        .lines()
        .filter(|line| line.starts_with("Exec"))
        .collect();

    if final_exec_lines.len() != 1 {
        return Err(anyhow!("Incorrect amount of Exec entries"));
    }

    let final_exec_lines = final_exec_lines[0];
    let mut final_exec_line_split = final_exec_lines.splitn(2, "=");
    if final_exec_lines.len() == 1 {
        return Err(anyhow!("Malformed exec entry: {}", final_exec_lines));
    }

    let exec_line = final_exec_line_split
        .nth(1)
        .context("Sanitization error, this should never happen")?
        .to_string();

    if appack_launch_cmd != exec_line {
        println!("=============================================");
        println!("  ⚠️ SECURITY ALERT: DESKTOP ENTRY REVIEW ⚠️  ");
        println!("=============================================");

        println!(
            "A desktop entry has been configured for this application. \
            Please **CRITICALLY REVIEW** the command that will be executed upon activation \
            against the expected safe command."
        );
        println!();

        println!("--- COMMAND COMPARISON ---");
        println!("  1. EXPECTED SAFE COMMAND:");
        println!("     > {appack_launch_cmd}");
        println!();

        println!("  2. CONFIGURED EXECUTION COMMAND:");
        println!("     > {exec_line}");
        println!();

        println!("--- IMMEDIATE ACTION REQUIRED ---");
        println!(
            "If **Command 2 (Configured)** does **NOT** exactly match **Command 1 (Expected)**, \
            this indicates a potential security risk where a malicious program may execute instead. \
            In this case, you must **IMMEDIATELY UNINSTALL** this application upon installation completion."
        );
        println!("=============================================");
        print!("Installation will resume in 5 seconds");
        io::stdout().flush()?;

        std::thread::sleep(Duration::from_secs(1));
        print!(".");
        io::stdout().flush()?;

        std::thread::sleep(Duration::from_secs(1));
        print!(".");
        io::stdout().flush()?;

        std::thread::sleep(Duration::from_secs(1));
        print!(".");
        io::stdout().flush()?;

        std::thread::sleep(Duration::from_secs(1));
        print!(".");
        io::stdout().flush()?;

        std::thread::sleep(Duration::from_secs(1));
        println!(".");
    }

    Ok(final_contents)
}

fn extract_config(archive: &mut ZipArchive<File>) -> Result<InstalledAppPackEntry> {
    let mut file = archive
        .by_name("AppPack.yaml")
        .context("File 'AppPack.yaml' not found in archive")?;

    let mut buffer = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut buffer)
        .context("Unable to read config file")?;
    serde_yaml::from_slice(&buffer).context("Invalid YAML file")
}

// Needs improvement:
// If the installation only partially finishes and is interrupted, clean the previous installation
// and try again if the user tries to install the same app again.
// We cannot use a temp dir because Snap hits us with cross-device errors when trying to copy over
// and desktop entries must be copied in a different directory
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
            .context(format!(
                "Desktop entry '{}' not found in archive",
                entry.entry
            ))?;

        let entry_file_fullpath = local_settings
            .desktop_entries_dir
            .join(format!("{new_app_version}_{}", entry.entry));
        if entry_file_fullpath.exists() {
            return Err(anyhow!("Desktop entry already exists: {entry_file_fullpath:?}").context("That app is seems to have been incorrectly uninstalled previously. Please delete the files from the previous installation before proceeding."));
        }
    }

    std::fs::create_dir_all(&new_app_base_dir.join("desktop"))?;

    println!("Extracting app data.. This may take a while.");

    {
        let mut image_file = archive
            .by_name(image_filename)
            .context(format!("Image '{}' not found in archive", image_filename))?;
        let image_fullpath = new_app_base_dir.join(image_filename);

        let mut outfile = File::create(&image_fullpath).context(format!(
            "Unable to create file {}",
            image_fullpath.display()
        ))?;
        io::copy(&mut image_file, &mut outfile)?;
    }

    println!("Extracting desktop entries..");

    for entry in desktop_entries.iter() {
        {
            let mut entry_file = archive
                .by_name(&format!("desktop/{}", entry.entry))
                .context(format!(
                    "Desktop entry '{}' not found in archive",
                    entry.entry
                ))?;

            let entry_fullpath = local_settings
                .desktop_entries_dir
                .join(format!("{new_app_version}_{}", entry.entry));
            let mut outfile =
                File::create(&entry_fullpath).context("Unable to create desktop entry file")?;

            let mut file_content = String::new();
            entry_file
                .read_to_string(&mut file_content)
                .context("Unable to read entry file")?;

            let file_content =
                process_desktop_entry(&file_content, entry, &new_app_entry, &local_settings)
                    .context("Unable to parse desktop entry")?;

            outfile.write_all(file_content.as_bytes())?;
        }

        {
            let mut entry_file = archive
                .by_name(&format!("desktop/{}", entry.icon))
                .context(format!(
                    "Desktop entry '{}' not found in archive",
                    entry.icon
                ))?;
            let entry_fullpath = new_app_base_dir.join("desktop").join(&entry.icon);
            let mut outfile =
                File::create(&entry_fullpath).context("Unable to create desktop entry icon")?;

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
    if !AppBuildConfig::is_valid_version(&new_app_entry.version) {
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

pub fn install_appack(file_path: PathBuf, settings: AppPackLocalSettings) -> Result<()> {
    let file = File::open(&file_path).context(format!("Unable to open file {file_path:?}"))?;
    let mut archive = ZipArchive::new(file).context("Unable to open file as zip archive")?;

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
