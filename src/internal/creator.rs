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

use crate::internal::helpers::get_os_assigned_port;
use crate::types::app_build_config::AppBuildConfig;
use crate::types::app_installed::InstalledAppPackEntry;
use crate::types::{AppDesktopEntry, AppSnapshotTriggerMode};
use crate::utils::qmp::{delete_snapshot_blocking, has_snapshot_qmp, take_snapshot_blocking};
use crate::utils::zip_dir::zip_dir;
use anyhow::{Context, Result, anyhow};
use qapi::{Qmp, qmp};
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};
use crate::utils::xdg_session_type_detector::get_freerdp_executable;

fn create_image(path: &Path) -> Result<()> {
    Command::new("qemu-img")
        .arg("create")
        .arg("-f")
        .arg("qcow2")
        .arg(path)
        .arg("32G")
        .status()
        .context("Failed to create disk image")?;

    Ok(())
}

// TODO: rewrite the logic, we shouldn't ever run that, we're in a snap though
fn get_xfreerdp3_pids() -> Result<String> {
    let freerdp_exec = get_freerdp_executable();
    let shell_cmd = format!(
        "ps aux | grep {} | grep -v grep | awk '{{print $2}}'",
        freerdp_exec
    );
    let output = Command::new("sh")
        .arg("-c")
        .arg(shell_cmd)
        .output()
        .context("Failed to get FreeRDP pids")?;

    if !output.status.success() {
        return Err(anyhow!("PID finding command failed"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn terminate_xfreerdp3() -> Result<()> {
    // Return the PIDs as a single string (e.g., "1234 5678 9012")
    let pids_string = get_xfreerdp3_pids()?;

    if pids_string.is_empty() {
        return Ok(());
    }

    let pids: Vec<&str> = pids_string.split_whitespace().collect();

    let mut command = Command::new("kill");
    command.arg("-TERM");
    command.args(pids);
    println!("Executing: kill -TERM {}", pids_string);

    match command.status() {
        Ok(status) => {
            if !status.success() {
                eprintln!("'kill' command failed with status: {}", status);
            }
        }
        Err(e) => return Err(anyhow!("Failed to execute 'kill' command: {}", e)),
    }

    while let Ok(pids) = get_xfreerdp3_pids() {
        if pids.is_empty() {
            break;
        }

        thread::sleep(Duration::from_secs(1));
    }

    Ok(())
}

fn zip_appack(config: &AppBuildConfig) -> Result<()> {
    let zip_name = format!("{}_{}.zip", config.id, config.version);
    let zip_file = std::fs::File::create(zip_name).context("Failed to create zip file")?;
    let mut zip = ZipWriter::new(zip_file);

    let zip_options = SimpleFileOptions::default()
        .large_file(true)
        .compression_method(CompressionMethod::Zstd)
        // .compression_level(Some(9))
        .unix_permissions(0o755);

    // Add readme folder
    zip_dir(&mut zip, &zip_options, Path::new(&config.readme.folder))?;

    // Does not copy the desktop entries
    let mut installed_appack_entry = InstalledAppPackEntry::from(config.clone());

    // Add desktop entries
    if let Some(entries) = &config.desktop_entries {
        installed_appack_entry.desktop_entries = Some(Vec::new());

        for entry in entries {
            let entry_path = Path::new(&entry.entry);
            let entry_file_name = entry_path.file_name().ok_or_else(|| {
                anyhow!("Could not get file name of desktop entry {entry_path:?}")
            })?;

            let entry_icon_path = Path::new(&entry.icon);
            let entry_icon_name = entry_icon_path.file_name().ok_or_else(|| {
                anyhow!("Could not get icon name of desktop entry {entry_icon_path:?}")
            })?;

            let mut f1 = std::fs::File::open(&entry.entry)
                .context(format!("Failed to open entry {entry_path:?}"))?;
            let file_in_zip = format!("desktop/{}", entry_file_name.display());
            zip.start_file(&file_in_zip, zip_options)
                .context(format!("Failed to start zip entry {file_in_zip}"))?;
            std::io::copy(&mut f1, &mut zip)
                .context(format!("Failed to copy to archive {file_in_zip}"))?;

            let mut f1 = std::fs::File::open(&entry.icon)
                .context(format!("Failed to open icon {entry_icon_path:?}"))?;
            let file_in_zip = format!("desktop/{}", entry_icon_name.display());

            // Same icon may be reused for multiple entries
            match zip.start_file(&file_in_zip, zip_options) {
                Ok(_) => {
                    std::io::copy(&mut f1, &mut zip)
                        .context(format!("Failed to copy to archive {file_in_zip}"))?;
                }
                Err(e) => {
                    println!("Failed to start icon zip entry {file_in_zip}: {}", e);
                    println!("This can be intentional, skipping.")
                }
            };

            let installed_desktop_entry = AppDesktopEntry {
                entry: entry_file_name.to_string_lossy().to_string(),
                icon: entry_icon_name.to_string_lossy().to_string(),
                rdp_args: entry.rdp_args.clone(),
            };

            installed_appack_entry
                .desktop_entries
                .as_mut()
                .unwrap()
                .push(installed_desktop_entry);
            println!("Added {entry_file_name:?} to package");
        }
    }

    let installed_entry_str = serde_yaml::to_string(&installed_appack_entry)?;
    zip.start_file("AppPack.yaml", zip_options)
        .context("Failed to start file AppPack")?;
    zip.write_all(installed_entry_str.as_bytes())
        .context("Failed to write AppPack.yaml to zip")?;

    // Add image
    println!("Adding image file to package. This will take a while.");
    zip.start_file("image.qcow2", zip_options)
        .context("Failed to start image.qcow2".to_string())?;
    let mut f1 = std::fs::File::open(&config.image)
        .context(format!("Failed to open image file {}", config.image))?;
    std::io::copy(&mut f1, &mut zip)
        .context(format!("Failed to copy to archive file {}", config.image))?;
    println!("Added \"image.qcow2\" to package");

    zip.finish().context("Failed to finish zip")?;

    Ok(())
}

pub fn creator_new() -> Result<()> {
    let assets_path_str = std::env::var("SNAP").context("Failed to get assets path")?;
    let assets_path = Path::new(&assets_path_str).join("assets");
    std::fs::create_dir("AppPack").context("Failed to create AppPack directory")?;
    std::fs::create_dir("AppPack/readme").context("Failed to create readme directory")?;
    std::fs::create_dir("AppPack/desktop").context("Failed to create desktop directory")?;

    std::fs::copy(
        assets_path.join("creator").join("README.md"),
        "AppPack/readme/README.md",
    )?;
    std::fs::copy(
        assets_path.join("creator").join("AppPackBuildConfig.yaml"),
        "AppPack/AppPackBuildConfig.yaml",
    )?;
    std::fs::copy(
        assets_path.join("creator").join("ms-cmd.desktop"),
        "AppPack/desktop/ms-cmd.desktop",
    )?;
    std::fs::copy(
        assets_path.join("creator").join("plain-rdp.desktop"),
        "AppPack/desktop/plain-rdp.desktop",
    )?;
    std::fs::copy(
        assets_path.join("creator").join("ms-cmd.svg"),
        "AppPack/desktop/ms-cmd.svg",
    )?;

    create_image(Path::new("AppPack/image.qcow2"))?;

    Ok(())
}

pub fn creator_boot_install() -> Result<()> {
    let config = AppBuildConfig::new(Path::new("AppPackBuildConfig.yaml"))?;

    let mut command = config.get_boot_install_command();

    command.status()?;

    Ok(())
}

pub fn creator_boot() -> Result<()> {
    let config = AppBuildConfig::new(Path::new("AppPackBuildConfig.yaml"))?;
    let free_port = get_os_assigned_port()?;

    let mut qemu_command = config.get_boot_configure_command(free_port);
    let mut qemu_child = qemu_command.spawn()?;

    // Wait for qmp socket to be available
    let qmp_socket_path = Path::new("qmp-appack.sock");
    loop {
        match qemu_child.try_wait() {
            // 1. Ok(None): Child is STILL RUNNING
            Ok(None) => {
                match UnixStream::connect(&qmp_socket_path) {
                    Ok(_) => {
                        break;
                    }
                    Err(e) => {
                        println!("Waiting for QMP socket connection: {}", e);
                        thread::sleep(Duration::from_millis(200));
                    }
                };
            }

            // 2. Ok(Some(status)): Child has EXITED
            Ok(Some(status)) => {
                eprintln!("QEMU process unexpectedly exited with status: {}", status);
                return Err(anyhow!("QEMU process died before QMP socket was ready.")
                    .context("Qemu failed to start. Make sure you installed AppPack with the command on the Readme (with the appropriate connections)."));
            }

            // 3. Err(e): An error occurred while trying to check the status
            Err(e) => {
                return Err(anyhow!(e).context("Error while checking QEMU status"));
            }
        }
    }

    println!("QMP socket is ready! Continuing.");

    let mut rdp_command = config.get_rdp_configure_command(free_port);

    match rdp_command.status() {
        Ok(status) => {
            if status.success() {
                println!("RDP was successful");
            } else {
                return Err(anyhow!("RDP failed with status: {status:?}"));
            }
        }
        Err(e) => {
            return Err(anyhow!("RDP process failed with error: {e:?}"));
        }
    }

    qemu_child.wait()?;
    println!("Qemu exited");

    Ok(())
}

// For now we will take a snapshot of the disk and memory and this is what will be shipped.
// It is probably possible to optimize this further.
pub fn creator_snapshot() -> Result<()> {
    // We read the config first to validate its contents before proceeding with the snapshot
    let config = AppBuildConfig::new(Path::new("AppPackBuildConfig.yaml"))?;
    let socket_addr = "./qmp-appack.sock";
    let stream = UnixStream::connect(socket_addr).context("Failed to connect to QMP socket")?;
    let mut qmp = Qmp::from_stream(&stream);

    qmp.handshake().context("Failed to handshake with QMP")?;

    match has_snapshot_qmp(&mut qmp, "appack-init") {
        Ok(true) => {
            return Err(anyhow!(
                "Snapshot 'appack-init' already exists. Please delete it first."
            ));
        }
        Err(e) => return Err(e),
        _ => {}
    }

    match has_snapshot_qmp(&mut qmp, "appack-onclose") {
        Ok(true) => {
            return Err(anyhow!(
                "Snapshot 'appack-onclose' already exists. Please delete it first."
            ));
        }
        Err(e) => return Err(e),
        _ => {}
    }

    // 1. Close RDP connections (ctrl+c on xfreerdp?)
    terminate_xfreerdp3()?;

    // 2. Pause VM
    qmp.execute(&qmp::stop {}).context("Failed to stop VM")?;

    // 3. Take a snapshot (internal)
    match config.snapshot {
        AppSnapshotTriggerMode::OnClose => {
            take_snapshot_blocking(&mut qmp, "appack-init")?;
        }
        AppSnapshotTriggerMode::Never => {
            take_snapshot_blocking(&mut qmp, "appack-init")?;
        }
        AppSnapshotTriggerMode::NeverLoad => {}
    }

    // 4. Destroy the VM. Why do this gracefully?
    qmp.execute(&qmp::quit {}).context("Failed to quit QMP")?;

    // 5. Zip files
    match zip_appack(&config) {
        Ok(_) => println!("AppPack created successfully"),
        Err(e) => {
            delete_snapshot_blocking(&mut qmp, "appack-init")?;
            println!("Snapshot deleted. You can safely retry.");

            let zip_name = format!("{}_{}.zip", config.id, config.version);
            let _ = std::fs::remove_file(zip_name); // Ignore error

            return Err(e);
        }
    }

    Ok(())
}

pub fn creator_pack() -> Result<()> {
    let config = AppBuildConfig::new(Path::new("AppPackBuildConfig.yaml"))?;
    match zip_appack(&config) {
        Ok(_) => Ok(()),
        Err(e) => {
            let zip_name = format!("{}_{}.zip", config.id, config.version);
            let _ = std::fs::remove_file(zip_name); // Ignore error

            Err(e)
        }
    }
}
