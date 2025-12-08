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

use anyhow::{Context, anyhow};
use qapi::{Qmp, Stream, qmp};
use std::io::BufReader;
use std::os::unix::net::UnixStream;

pub fn take_snapshot_blocking(
    qmp: &mut Qmp<Stream<BufReader<&UnixStream>, &UnixStream>>,
    snapshot_name: &str,
) -> anyhow::Result<()> {
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
) -> anyhow::Result<()> {
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

pub fn has_snapshot_qmp(
    qmp: &mut Qmp<Stream<BufReader<&UnixStream>, &UnixStream>>,
    snapshot_name: &str,
) -> anyhow::Result<bool> {
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

        if is_snapshot_present {
            return Ok(true);
        }
    }

    Ok(false)
}
