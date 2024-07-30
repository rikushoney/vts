use std::ffi::{c_char, CString};
use std::marker::{PhantomData, PhantomPinned};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("an instance of Yosys already exists")]
    InstanceExists,
    #[error("Yosys requires an input filename")]
    MissingInput,
    #[error("Yosys requires an output filename")]
    MissingOutput,
    #[error("unknown file format \"{0}\"")]
    UnknownFormat(PathBuf),
    #[error("unsupported input file format \"{0}\"")]
    UnsupportedInput(PathBuf),
    #[error("unsupported output file format \"{0}\"")]
    UnsupportedOutput(PathBuf),
}

pub type Result<T> = std::result::Result<T, Error>;

static YOSYS_LOCKED: AtomicBool = AtomicBool::new(false);

pub struct Yosys {
    _data: [u8; 0],
    _marker: PhantomData<(*mut u8, PhantomPinned)>,
}

macro_rules! s {
    (cstr $input:tt) => {
        CString::new($input).expect(concat!(stringify!($input), " should not contain nul bytes"))
    };
    (path $input:tt) => {
        $input
            .to_str()
            .expect(concat!(stringify!($input), " should be valid utf-8"))
    };
}

fn yosys_setup() {
    unsafe { vts_yosys_sys::vts_yosys_setup() }
}

fn yosys_shutdown() {
    unsafe { vts_yosys_sys::vts_yosys_shutdown() }
}

fn yosys_run_pass(command: *const c_char) {
    unsafe { vts_yosys_sys::vts_yosys_run_pass(command, ptr::null_mut()) }
}

fn yosys_run_frontend(filename: *const c_char, command: *const c_char) -> i32 {
    unsafe { vts_yosys_sys::vts_yosys_run_frontend(filename, command, ptr::null_mut()) }
}

fn yosys_run_backend(filename: *const c_char, command: *const c_char) {
    unsafe { vts_yosys_sys::vts_yosys_run_backend(filename, command, ptr::null_mut()) }
}

impl Yosys {
    pub fn new() -> Result<Self> {
        let locked = YOSYS_LOCKED.swap(true, Ordering::SeqCst);
        if !locked {
            yosys_setup();
            Ok(Self {
                _data: [],
                _marker: PhantomData,
            })
        } else {
            Err(Error::InstanceExists)
        }
    }

    pub(crate) fn run_pass(&self, command: &str) {
        let command = s!(cstr command);
        yosys_run_pass(command.as_ptr());
    }

    pub(crate) fn run_frontend(&self, filename: &str, command: &str) -> i32 {
        let filename = s!(cstr filename);
        let command = s!(cstr command);
        yosys_run_frontend(filename.as_ptr(), command.as_ptr())
    }

    pub(crate) fn run_backend(&self, filename: &str, command: &str) {
        let filename = s!(cstr filename);
        let command = s!(cstr command);
        yosys_run_backend(filename.as_ptr(), command.as_ptr());
    }
}

impl Drop for Yosys {
    fn drop(&mut self) {
        yosys_shutdown();
        let was_locked = YOSYS_LOCKED.swap(false, Ordering::SeqCst);
        debug_assert!(was_locked);
    }
}

#[derive(Default)]
pub struct Command {
    input_filename: Option<PathBuf>,
    output_filename: Option<PathBuf>,
    command_buffer: String,
}

macro_rules! impl_command {
    (input $name:ident) => {
        pub fn $name<P>(&mut self, filename: P) -> &mut Self
        where
            P: AsRef<Path>,
        {
            self.input_filename = Some(filename.as_ref().to_path_buf());
            self
        }
    };
    (output $name:ident) => {
        pub fn $name<P>(&mut self, filename: P) -> &mut Self
        where
            P: AsRef<Path>,
        {
            self.output_filename = Some(filename.as_ref().to_path_buf());
            self
        }
    };
    ($name:ident) => {
        pub fn $name(&mut self) -> &mut Self {
            self.push_command(stringify!($name));
            self
        }
    };
    ($name:literal as $alias:ident) => {
        pub fn $alias(&mut self) -> &mut Self {
            self.push_command($name);
            self
        }
    };
}

macro_rules! impl_frontend {
    ($yosys:expr => $filename:ident : $command:literal) => {
        // TODO(rikus): Check and report yosys frontend errors.
        $yosys.run_frontend(s!(path $filename), $command);
    }
}

macro_rules! impl_backend {
    ($yosys:expr => $filename:ident : $command:literal) => {
        // TODO(rikus): Check and report yosys backend errors.
        $yosys.run_backend(s!(path $filename), $command);
    }
}

pub enum FileFormat {
    Verilog,
    SV,
    Blif,
    Json,
}

impl FileFormat {
    pub fn guess(filename: &Path) -> Result<Self> {
        let extension = filename
            .extension()
            .ok_or(Error::UnknownFormat(filename.to_path_buf()))?;
        Ok(match extension.to_string_lossy().as_ref() {
            "v" => Self::Verilog,
            "sv" => Self::SV,
            "blif" => Self::Blif,
            "json" => Self::Json,
            _ => {
                return Err(Error::UnknownFormat(filename.to_path_buf()));
            }
        })
    }
}

impl Command {
    pub fn new() -> Self {
        Self {
            input_filename: None,
            output_filename: None,
            command_buffer: String::new(),
        }
    }

    fn push_command(&mut self, command: &str) {
        if !self.command_buffer.is_empty() {
            self.command_buffer.push_str("; ");
        }
        self.command_buffer.push_str(command);
    }

    impl_command!(input read_verilog);
    impl_command!(input read_sv);
    impl_command!(input read_blif);

    impl_command!(output write_blif);
    impl_command!(output write_json);

    impl_command!(flatten);
    impl_command!(opt);
    impl_command!(pmuxtree);
    impl_command!(proc);
    impl_command!("design -reset" as reset_design);
    impl_command!(simplemap);

    pub fn execute(&mut self, yosys: &Yosys) -> Result<()> {
        let input_filename = self.input_filename.as_ref().ok_or(Error::MissingInput)?;
        let input_format = FileFormat::guess(input_filename)?;
        let output_filename = self.output_filename.as_ref().ok_or(Error::MissingOutput)?;
        let output_format = FileFormat::guess(output_filename)?;
        // TODO(rikus): Check and report yosys errors.
        match input_format {
            FileFormat::Verilog => {
                impl_frontend!(yosys => input_filename : "read_verilog");
            }
            FileFormat::SV => {
                impl_frontend!(yosys => input_filename : "read_verilog -sv");
            }
            FileFormat::Blif => {
                impl_frontend!(yosys => input_filename : "read_blif");
            }
            _ => {
                return Err(Error::UnsupportedInput(input_filename.to_path_buf()));
            }
        }
        yosys.run_pass(&self.command_buffer);
        match output_format {
            FileFormat::Blif => {
                impl_backend!(yosys => output_filename : "write_blif");
            }
            FileFormat::Json => {
                impl_backend!(yosys => output_filename : "write_json");
            }
            _ => {
                return Err(Error::UnsupportedOutput(output_filename.to_path_buf()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[serial_test::serial]
mod tests {
    use super::*;

    #[test]
    fn test_yosys_new() {
        let _yosys = Yosys::new().unwrap();
    }

    #[test]
    fn test_yosys_is_not_threadsafe() {
        {
            let _yosys = Yosys::new().unwrap();
            assert!(matches!(Yosys::new(), Err(Error::InstanceExists)));
        }
        let _yosys = Yosys::new().unwrap();
    }
}
