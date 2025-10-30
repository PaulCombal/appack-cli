mod internal;

use crate::internal::creator::{creator_boot, creator_boot_install, creator_new, creator_snapshot, creator_test};
use crate::internal::info::print_info;
use crate::internal::install_appack::{install_appack};
use crate::internal::types::AppPackLocalSettings;
use crate::internal::uninstall_appack::uninstall_appack;
use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};

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

    Info,
    Test,
}

#[derive(Debug, Subcommand, ValueEnum, Clone)]
enum CliCreatorAction {
    New,
    Boot,
    BootInstall,
    Snapshot,
}

fn main() -> Result<()> {
    let args = Cli::parse();

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
        },
        CliAction::Info => {
            print_info(&settings);
        },
        CliAction::Test => {
            creator_test()?;
        },
    }

    Ok(())
}
