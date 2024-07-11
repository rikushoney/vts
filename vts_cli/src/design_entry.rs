use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use clap::Subcommand;
use thiserror::Error;

use vts_abc::{Abc, BlifLutMapper};
use vts_core::blif::reader as blif_reader;
use vts_core::blif::BlifReader;

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
    BlifRead(#[from] blif_reader::Error),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(
        "the yosys frontend is not yet supported (see {}/1)",
        GITHUB_REPO_ISSUES
    )]
    RequiresYosys,
    #[error(
        "piping requires ABC::Io_ReadBlifMv to support strings (see {}/2)",
        GITHUB_REPO_ISSUES
    )]
    RequiresAbcBlifReadString,
}

type Result<T> = std::result::Result<T, Error>;

enum FileFormat {
    Blif,
    Verilog,
    SystemVerilog,
}

impl FileFormat {
    fn guess<P>(filename: P) -> Option<Self>
    where
        P: AsRef<Path>,
    {
        match filename.as_ref().extension()?.to_str()? {
            "blif" => Some(Self::Blif),
            "v" => Some(Self::Verilog),
            "sv" => Some(Self::SystemVerilog),
            _ => None,
        }
    }
}

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
    },
}

fn check_file_is_not_pipe(filename: &PathBuf) -> Result<()> {
    if matches!(filename.to_str(), Some("-")) {
        Err(Error::RequiresAbcBlifReadString)
    } else {
        Ok(())
    }
}

fn check_file_exists_and_guess_format(filename: &PathBuf) -> Result<FileFormat> {
    if !filename.exists() {
        return Err(Error::FileNotFound(filename.clone()));
    }
    FileFormat::guess(filename).ok_or(Error::UnknownFileFormat)
}

fn check(input_filename: &PathBuf) -> Result<()> {
    check_file_is_not_pipe(input_filename)?;
    let input_format = check_file_exists_and_guess_format(input_filename)?;
    match input_format {
        FileFormat::Blif => {
            let input_file = fs::File::open(input_filename)?;
            let mut reader =
                BlifReader::from_reader(BufReader::new(input_file), input_filename.to_str())?;
            let _netlist = reader.parse_netlist()?;
            Ok(())
        }
        _ => {
            return Err(Error::RequiresYosys);
        }
    }
}

fn lutmap(input_filename: &PathBuf, output_filename: &PathBuf) -> Result<()> {
    check_file_is_not_pipe(input_filename)?;
    check_file_is_not_pipe(output_filename)?;
    let input_format = check_file_exists_and_guess_format(input_filename)?;
    match input_format {
        FileFormat::Blif => {
            let abc = Abc::new()?;
            // TODO(rikus): accept LUT size as an argument
            BlifLutMapper::new(input_filename, 4).run(&abc, output_filename)?;
        }
        _ => {
            return Err(Error::RequiresYosys);
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
            } => lutmap(
                input_filename,
                output_filename.as_ref().unwrap_or(&PathBuf::from("-")),
            ),
        }
    }
}
