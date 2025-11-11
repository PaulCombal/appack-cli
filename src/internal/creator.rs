use crate::internal::helpers::{delete_snapshot_blocking, get_os_assigned_port, take_snapshot_blocking, zip_dir};
use crate::internal::types::{AppPackDesktopEntry, AppPackIndexFile, AppPackSnapshotMode, InstalledAppPackEntry};
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
use zip::result::ZipError;

fn create_image(path: &Path) -> Result<()> {
    Command::new("qemu-img")
        .arg("create")
        .arg("-f")
        .arg("qcow2")
        .arg(path)
        .arg("32G")
        .status()
        .map_err(|e| anyhow!("Failed to create image: {}", e))?;

    Ok(())
}

fn get_xfreerdp3_pids() -> Result<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg("ps aux | grep xfreerdp3 | grep -v grep | awk '{print $2}'")
        .output()
        .map_err(|e| anyhow!("Failed to execute command: {}", e))?;

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

        println!("Waiting for processes to exit...");
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    Ok(())
}

fn zip_appack(config: &AppPackIndexFile) -> Result<()> {
    let zip_name = format!("{}_{}.zip", config.id, config.version);
    let zip_file =
        std::fs::File::create(zip_name).map_err(|e| anyhow!("Failed to create zip file: {}", e))?;
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

            let mut f1 =
                std::fs::File::open(&entry.entry).context(format!("Failed to open entry {entry_path:?}"))?;
            let file_in_zip = format!("desktop/{}", entry_file_name.display());
            zip.start_file(&file_in_zip, zip_options)
                .context(format!("Failed to start zip entry {file_in_zip}"))?;
            std::io::copy(&mut f1, &mut zip)
                .context(format!("Failed to copy to archive {file_in_zip}"))?;

            let mut f1 =
                std::fs::File::open(&entry.icon).context(format!("Failed to open icon {entry_icon_path:?}"))?;
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
                },
            };

            let installed_desktop_entry = AppPackDesktopEntry {
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
        .map_err(|e| anyhow!("Failed to start file AppPack: {}", e))?;
    zip.write_all(installed_entry_str.as_bytes())
        .map_err(|e| anyhow!("Failed to write AppPack.yaml to zip: {}", e))?;

    // Add image
    println!("Adding image file to package. This will take a while.");
    zip.start_file("image.qcow2", zip_options).context("Failed to start image.qcow2".to_string())?;
    let mut f1 =
        std::fs::File::open(&config.image).context(format!("Failed to open image file {}", config.image))?;
    std::io::copy(&mut f1, &mut zip).context(format!("Failed to copy to archive file {}", config.image))?;
    println!("Added \"image.qcow2\" to package");

    zip.finish()
        .map_err(|e| anyhow!("Failed to finish zip: {}", e))?;

    Ok(())
}

pub fn creator_new() -> Result<()> {
    let assets_path_str =
        std::env::var("SNAP").map_err(|e| anyhow!("Failed to get assets path: {}", e))?;
    let assets_path = Path::new(&assets_path_str).join("assets");
    std::fs::create_dir("AppPack")
        .map_err(|e| anyhow!("Failed to create AppPack directory: {}", e))?;
    std::fs::create_dir("AppPack/readme")
        .map_err(|e| anyhow!("Failed to create readme directory: {}", e))?;
    std::fs::create_dir("AppPack/desktop")
        .map_err(|e| anyhow!("Failed to create desktop directory: {}", e))?;

    std::fs::copy(
        assets_path.join("creator").join("README.md"),
        "AppPack/readme/README.md",
    )?;
    std::fs::copy(
        assets_path.join("creator").join("AppPack.yaml"),
        "AppPack/AppPack.yaml",
    )?;
    std::fs::copy(
        assets_path.join("creator").join("myapp.desktop"),
        "AppPack/desktop/myapp.desktop",
    )?;
    std::fs::copy(
        assets_path.join("creator").join("ms-cmd.svg"),
        "AppPack/desktop/ms-cmd.svg",
    )?;

    create_image(Path::new("AppPack/image.qcow2"))?;

    Ok(())
}

pub fn creator_boot_install() -> Result<()> {
    let config = AppPackIndexFile::new(Path::new("AppPack.yaml"))?;

    let mut command = config.get_boot_install_command();

    command.status()?;

    Ok(())
}

pub fn creator_boot() -> Result<()> {
    let config = AppPackIndexFile::new(Path::new("AppPack.yaml"))?;
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
                    },
                    Err(e) => {
                        println!("Waiting for QMP socket connection: {}", e);
                        thread::sleep(Duration::from_millis(200));
                    }
                };
            }

            // 2. Ok(Some(status)): Child has EXITED
            Ok(Some(status)) => {
                eprintln!("QEMU process unexpectedly exited with status: {}", status);
                return Err(anyhow!("QEMU process died before QMP socket was ready.").context("QEMU failed to start"));
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
    let config = AppPackIndexFile::new(Path::new("AppPack.yaml"))?;
    let socket_addr = "./qmp-appack.sock";
    let stream = UnixStream::connect(socket_addr)
        .map_err(|e| anyhow!("Failed to connect to QMP socket: {}", e))?;
    let mut qmp = Qmp::from_stream(&stream);

    qmp
        .handshake()
        .map_err(|e| anyhow!("Failed to handshake with QMP: {}", e))?;

    // 1. Close RDP connections (ctrl+c on xfreerdp?)
    terminate_xfreerdp3()?;

    // 2. Pause VM
    qmp.execute(&qmp::stop {}).context("Failed to stop VM")?;

    // 2.5 (TODO) Check if the image already have snapshots

    // 3. Take a snapshot (internal)
    match config.snapshot {
        AppPackSnapshotMode::OnClose => {
            take_snapshot_blocking(&mut qmp, "appack-init")?;
            take_snapshot_blocking(&mut qmp, "appack-onclose")?;
        }
        AppPackSnapshotMode::Never => {
            take_snapshot_blocking(&mut qmp, "appack-init")?;
        }
        AppPackSnapshotMode::NeverLoad => {},
    }

    // 4. Destroy the VM. Why do this gracefully?
    qmp.execute(&qmp::quit {})
        .map_err(|e| anyhow!("Failed to quit QMP: {}", e))?;

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
    let config = AppPackIndexFile::new(Path::new("AppPack.yaml"))?;
    match zip_appack(&config) {
        Ok(_) => Ok(()),
        Err(e) => {
            let zip_name = format!("{}_{}.zip", config.id, config.version);
            let _ = std::fs::remove_file(zip_name); // Ignore error

            Err(e)
        }
    }
}
