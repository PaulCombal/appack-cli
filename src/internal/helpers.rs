use crate::internal::types::{AppPackLocalSettings, InstalledAppPackEntry};
use anyhow::{Context, Result, anyhow};
use qapi::{Qmp, Stream, qmp};
use std::fs::File;
use std::io::BufReader;
use std::net::{Ipv4Addr, TcpListener};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::Command;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

pub fn zip_dir(
    zip: &mut ZipWriter<File>,
    zip_options: &SimpleFileOptions,
    dirpath: &Path,
) -> Result<()> {
    let root_dir_name = dirpath
        .file_name()
        .ok_or_else(|| anyhow!("Invalid directory path"))?
        .to_str()
        .ok_or_else(|| anyhow!("Directory name contains invalid UTF-8"))?;

    zip_dir_recursive(zip, zip_options, dirpath, Path::new(root_dir_name))?;

    let dir_name_in_zip = format!("{}/", root_dir_name);
    zip.add_directory(&dir_name_in_zip, *zip_options)?;

    Ok(())
}

fn zip_dir_recursive(
    zip: &mut ZipWriter<File>,
    zip_options: &SimpleFileOptions,
    current_path: &Path,
    path_in_zip_prefix: &Path,
) -> Result<()> {
    for entry in std::fs::read_dir(current_path)? {
        let entry = entry?;
        let path = entry.path();

        let name = entry.file_name();
        let path_in_zip = path_in_zip_prefix.join(name);
        let path_in_zip_str = path_in_zip
            .to_str()
            .ok_or_else(|| anyhow!("Path contains invalid UTF-8: {:?}", path))?;

        if path.is_dir() {
            let dir_name_in_zip = format!("{}/", path_in_zip_str);
            zip.add_directory(&dir_name_in_zip, *zip_options)
                .context("Failed to add directory to zip")?;

            zip_dir_recursive(zip, zip_options, &path, &path_in_zip)?;
        } else if path.is_file() {
            zip.start_file(path_in_zip_str, *zip_options)
                .context("Failed to start file in zip")?;

            let mut f = File::open(&path).context(format!("Failed to open file {path:?}"))?;

            std::io::copy(&mut f, zip).context(format!("Failed to copy file {path:?} to zip"))?;
        }
    }

    Ok(())
}

pub fn get_os_assigned_port() -> Result<u16> {
    let listener = TcpListener::bind(format!("{}:0", Ipv4Addr::LOCALHOST))?;
    let port = listener.local_addr()?.port();
    Ok(port)
}

pub fn take_snapshot_blocking(
    qmp: &mut Qmp<Stream<BufReader<&UnixStream>, &UnixStream>>,
    snapshot_name: &str,
) -> Result<()> {
    let blocks = qmp
        .execute(&qmp::query_block {})
        .context("Failed to get block info")?;
    let blocks = blocks
        .iter()
        .filter(|b| b.inserted.is_some())
        .collect::<Vec<_>>();

    if blocks.len() != 1 {
        return Err(anyhow!(
            "Expected 1 block device, got {} ({blocks:?})",
            blocks.len()
        ));
    }

    let block = &blocks[0];
    let block_inserted = block
        .inserted
        .clone()
        .context("BlockInfo does not contain 'inserted' data.")?;

    let block_node_name = block_inserted
        .node_name
        .context("BlockDeviceInfo does not contain 'node_name'.")?;

    let job_name = format!("{snapshot_name}-snapshot");

    qmp.execute(&qmp::snapshot_save {
        tag: snapshot_name.to_string(),
        vmstate: block_node_name.clone(),
        devices: [block_node_name.clone()].to_vec(),
        job_id: job_name.clone(),
    })
    .context("Failed to make snapshot")?;

    // Wait for the snapshot to finish
    loop {
        let jobs = qmp
            .execute(&qmp::query_jobs {})
            .context("Failed to get jobs")?;
        let job = jobs.into_iter().find(|j| j.id == job_name);
        if job.is_none() {
            return Err(anyhow!("Failed to find job with id '{job_name}'"));
        }

        let job = job.unwrap();

        println!("Job status: {:#?}", job);

        match job.status {
            qmp::JobStatus::concluded => {
                if let Some(err) = job.error {
                    return Err(anyhow!("Failed to take snapshot: {}", err));
                }
                println!("Snapshot complete");
                break;
            }
            qmp::JobStatus::created
            | qmp::JobStatus::running
            | qmp::JobStatus::waiting
            | qmp::JobStatus::pending => {
                std::thread::sleep(std::time::Duration::from_secs(1));
                println!("Snapshot in progress, waiting...");
            }
            _ => {
                return Err(anyhow!("Snapshot in unknown state: {job:?}"));
            }
        }
    }

    Ok(())
}

pub fn delete_snapshot_blocking(
    qmp: &mut Qmp<Stream<BufReader<&UnixStream>, &UnixStream>>,
    snapshot_name: &str,
) -> Result<()> {
    let blocks = qmp
        .execute(&qmp::query_block {})
        .context("Failed to get block info")?;
    let blocks = blocks
        .iter()
        .filter(|b| b.inserted.is_some())
        .collect::<Vec<_>>();

    if blocks.len() != 1 {
        return Err(anyhow!(
            "Expected 1 block device, got {} ({blocks:?})",
            blocks.len()
        ));
    }

    let block = &blocks[0];
    let block_inserted = block
        .inserted
        .clone()
        .context("BlockInfo does not contain 'inserted' data.")?;

    if let Some(snapshots) = block_inserted.image.base.snapshots {
        let is_snapshot_present = snapshots
            .iter()
            .any(|snapshot| snapshot.name == snapshot_name);

        if !is_snapshot_present {
            return Err(anyhow!("Cannot delete snapshot {snapshot_name}")
                .context("Failed to delete snapshot, it is not found."));
        }
    }

    let block_node_name = block_inserted
        .node_name
        .context("BlockDeviceInfo does not contain 'node_name'.")?;

    let job_name = format!("{snapshot_name}-del-snapshot");

    qmp.execute(&qmp::snapshot_delete {
        tag: snapshot_name.to_string(),
        devices: [block_node_name.clone()].to_vec(),
        job_id: job_name.clone(),
    })
    .context("Failed to make snapshot")?;

    // Wait for the snapshot to finish
    loop {
        let jobs = qmp
            .execute(&qmp::query_jobs {})
            .context("Failed to get jobs")?;
        let job = jobs.into_iter().find(|j| j.id == job_name);
        if job.is_none() {
            return Err(anyhow!("Failed to find job with id '{job_name}'"));
        }

        let job = job.unwrap();

        println!("Job status: {:#?}", job);

        match job.status {
            qmp::JobStatus::concluded => {
                if let Some(err) = job.error {
                    return Err(anyhow!("Failed to delete snapshot: {}", err));
                }
                println!("Snapshot '{snapshot_name}' deleted");
                break;
            }
            qmp::JobStatus::created
            | qmp::JobStatus::running
            | qmp::JobStatus::waiting
            | qmp::JobStatus::pending => {
                std::thread::sleep(std::time::Duration::from_millis(500));
                println!("Snapshot deletion in progress, waiting...");
            }
            _ => {
                return Err(anyhow!("Snapshot deletion in unknown state: {job:?}"));
            }
        }
    }

    Ok(())
}

pub fn has_snapshot(snapshot_name: &str, image_name: &Path) -> Result<bool> {
    let output = Command::new("qemu-img")
        .arg("snapshot")
        .arg("-lU")
        .arg(image_name)
        .output()
        .context("Failed to get image snapshots")?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to get image snapshots (output failed: {output:?})"
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let contains_snapshot = stdout.contains(&format!(" {snapshot_name} "));

    Ok(contains_snapshot)
}

// This could be a method of settings
pub fn get_app_installed(
    settings: &AppPackLocalSettings,
    id: &str,
    version: Option<&str>,
) -> Result<InstalledAppPackEntry> {
    let all_installed = settings
        .get_installed()
        .context("Failed to get installed app packs")?;

    let matches = all_installed.installed.iter().filter(|i| i.id == id);

    let filtered: Vec<&InstalledAppPackEntry> = match version {
        Some(v) => matches.filter(|i| i.version == v).collect(),
        None => matches.collect(),
    };

    match filtered.len() {
        0 => Err(anyhow!("AppPack (or version) is not installed")),
        1 => Ok(filtered[0].clone()),
        _ => Err(anyhow!(
            "Multiple versions installed — please specify a version"
        )),
    }
}
