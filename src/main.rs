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

mod internal;
mod types;
mod utils;

use crate::internal::creator::{
    creator_boot, creator_boot_install, creator_new, creator_pack, creator_snapshot,
};
use crate::internal::info::print_info;
use crate::internal::install_appack::install_appack;
use crate::internal::launch::launch;
use crate::internal::list_installed::list_installed;
use crate::internal::reset::reset;
use crate::internal::uninstall_appack::uninstall_appack;
use crate::internal::version::print_version;
use crate::types::local_settings::AppPackLocalSettings;
use crate::utils::logger::log_debug;
use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    action: CliAction,
}

#[derive(Debug, Subcommand)]
enum CliAction {
    #[clap(alias = "i")]
    Install {
        file: PathBuf,
    },

    #[clap(alias = "u")]
    Uninstall {
        id: String,
    },

    Creator {
        action: CliCreatorAction,
    },

    #[clap(alias = "li")]
    ListInstalled,

    Launch {
        id: String,
        rdp_args: Option<String>,
        #[clap(long)]
        version: Option<String>,
    },

    Reset {
        id: String,
        #[clap(long)]
        version: Option<String>,
    },

    Version,
    Info {
        file: PathBuf,
    },
}

#[derive(Debug, Subcommand, ValueEnum, Clone)]
enum CliCreatorAction {
    New,
    Boot,
    BootInstall,
    Snapshot,
    Pack,
}

fn main() -> Result<()> {
    log_debug("AppPack starting");

    let args = match Cli::try_parse() {
        Ok(args) => args,
        Err(e) => {
            log_debug("Error parsing arguments:");
            log_debug(&e);

            // Keep the clap error message formatting
            Cli::parse();
            return Err(anyhow::anyhow!(e));
        }
    };

    let settings = AppPackLocalSettings::default();

    match args.action {
        CliAction::Install { file } => install_appack(file, settings)?,
        CliAction::Uninstall { id } => uninstall_appack(&settings, &id)?,
        CliAction::Creator { action } => match action {
            CliCreatorAction::New => {
                creator_new()?;
            }
            CliCreatorAction::BootInstall => {
                creator_boot_install()?;
            }
            CliCreatorAction::Boot => {
                creator_boot()?;
            }
            CliCreatorAction::Snapshot => {
                creator_snapshot()?;
            }
            CliCreatorAction::Pack => {
                creator_pack()?;
            }
        },
        CliAction::ListInstalled => {
            list_installed(settings)?;
        }
        CliAction::Version => {
            print_version(&settings)?;
        }
        CliAction::Info { file } => {
            print_info(&file)?;
        }
        CliAction::Launch {
            id,
            version,
            rdp_args,
        } => {
            launch(&settings, id, version.as_deref(), rdp_args.as_deref())?;
        }
        CliAction::Reset { id, version } => {
            reset(&settings, id, version.as_deref())?;
        }
    }

    Ok(())
}
