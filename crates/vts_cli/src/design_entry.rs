use std::path::{Path, PathBuf};

use clap::Subcommand;
use thiserror::Error;

use vts_abc::{Abc, BlifLutMapper};
use vts_core::interchange::yosys::Netlist as YosysNetlist;
use vts_yosys::{Command as YosysCmd, FileFormat, Yosys};

const GITHUB_REPO_ISSUES: &str = "https://github.com/rikushoney/vts/issues";

#[derive(Debug, Error)]
pub(super) enum Error {
    #[error("\"{0}\" does not exist")]
    FileNotFound(PathBuf),
    #[error("unknown input file format")]
    UnknownFileFormat,
    #[error(transparent)]
    Abc(#[from] vts_abc::Error),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(
        "piping requires Abc and Yosys to support reading from memory (see {}/2)",
        GITHUB_REPO_ISSUES
    )]
    RequiresAbcYosysReadString,
    #[error(transparent)]
    Yosys(#[from] vts_yosys::Error),
    #[error(transparent)]
    YosysNetlist(#[from] vts_core::interchange::yosys::Error),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Subcommand)]
pub(super) enum Command {
    Check {
        input_filename: PathBuf,
    },
    #[command(name = "lutmap")]
    LutMap {
        input_filename: PathBuf,
        #[arg(short = 'o')]
        output_filename: Option<PathBuf>,
        #[arg(short = 'k', default_value_t = 4)]
        k_lut: usize,
    },
}

fn check_file_is_not_pipe(filename: &Path) -> Result<()> {
    if matches!(filename.to_str(), Some("-")) {
        Err(Error::RequiresAbcYosysReadString)
    } else {
        Ok(())
    }
}

fn check_file_exists_and_guess_format(filename: &PathBuf) -> Result<FileFormat> {
    if !filename.exists() {
        return Err(Error::FileNotFound(filename.clone()));
    }
    FileFormat::guess(filename).map_err(|_| Error::UnknownFileFormat)
}

fn check(input_filename: &PathBuf) -> Result<()> {
    check_file_is_not_pipe(input_filename)?;
    let input_format = check_file_exists_and_guess_format(input_filename)?;
    if input_format == FileFormat::Json {
        let _netlist = YosysNetlist::from_file(input_filename)?;
        return Ok(());
    }
    let yosys = Yosys::new()?;
    let mut cmd = YosysCmd::new();
    match input_format {
        FileFormat::Verilog => {
            cmd.read_verilog(input_filename);
        }
        FileFormat::SV => {
            cmd.read_sv(input_filename);
        }
        FileFormat::Blif => {
            cmd.read_blif(input_filename);
        }
        FileFormat::Json => {
            // NOTE: Handled above to prevent unnecessary `Yosys` instance
            // creation.
            unreachable!()
        }
    }
    cmd.execute(&yosys)?;
    Ok(())
}

fn lutmap(input_filename: &PathBuf, output_filename: &PathBuf, k_lut: usize) -> Result<()> {
    check_file_is_not_pipe(input_filename)?;
    check_file_is_not_pipe(output_filename)?;
    let input_format = check_file_exists_and_guess_format(input_filename)?;
    match input_format {
        FileFormat::Blif => {
            let abc = Abc::new()?;
            BlifLutMapper::new(input_filename, k_lut).run(&abc, output_filename)?;
        }
        _ => {
            todo!()
        }
    }
    Ok(())
}

impl Command {
    pub(super) fn name(&self) -> &'static str {
        match self {
            Self::Check { .. } => "check",
            Self::LutMap { .. } => "lutmap",
        }
    }

    pub(super) fn run(&self) -> Result<()> {
        match self {
            Self::Check { input_filename } => check(input_filename),
            Self::LutMap {
                input_filename,
                output_filename,
                k_lut,
            } => lutmap(
                input_filename,
                output_filename.as_ref().unwrap_or(&PathBuf::from("-")),
                *k_lut,
            ),
        }
    }
}
