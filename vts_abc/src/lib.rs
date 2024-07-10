use std::ffi::{c_char, CString};
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use thiserror::Error;

use vts_abc_sys::AbcFrame;

#[derive(Debug, Error)]
pub enum Error {
    #[error("an instance of Abc already exists")]
    InstanceExists,
    #[error("Abc requires an input BLIF filename")]
    MissingInput,
    #[error("Abc requires an output BLIF filename")]
    MissingOutput,
    #[error("Abc failed to read BLIF file \"{0}\"")]
    ReadBlif(PathBuf),
    #[error("Abc failed to execute command \"{0}\"")]
    CommandFailed(String),
    #[error("Abc failed to write BLIF file \"{0}\"")]
    WriteBlif(PathBuf),
    #[error("Abc failed to set the LUT library")]
    SetLutLibrary,
}

pub type Result<T> = std::result::Result<T, Error>;

static ABC_LOCKED: AtomicBool = AtomicBool::new(false);

pub struct Abc(*mut AbcFrame);

fn abc_start() {
    unsafe { vts_abc_sys::abc_start() }
}

fn abc_stop() {
    unsafe { vts_abc_sys::abc_stop() }
}

fn abc_get_global_frame() -> *mut AbcFrame {
    unsafe { vts_abc_sys::abc_get_global_frame() }
}

fn abc_execute_command(framework: *mut AbcFrame, command: *const c_char) -> i32 {
    unsafe { vts_abc_sys::abc_execute_command(framework, command) }
}

fn abc_frame_set_lut_library(framework: *mut AbcFrame, library: *const c_char) -> i32 {
    unsafe { vts_abc_sys::abc_frame_set_lut_library(framework, library) }
}

impl Abc {
    pub fn new() -> Result<Abc> {
        let locked = ABC_LOCKED.swap(true, Ordering::SeqCst);
        if !locked {
            abc_start();
            Ok(Abc(abc_get_global_frame()))
        } else {
            Err(Error::InstanceExists)
        }
    }

    pub(crate) fn execute_command(&self, command: &str) -> i32 {
        let command = CString::new(command).expect("command should not contain nul bytes");
        abc_execute_command(self.0, command.as_ptr())
    }

    pub(crate) fn set_lut_library(&self, library: &str) -> i32 {
        let lut_library = CString::new(library).expect("lut library should not contain nul bytes");
        abc_frame_set_lut_library(self.0, lut_library.as_ptr())
    }
}

impl Drop for Abc {
    fn drop(&mut self) {
        abc_stop();
        let was_locked = ABC_LOCKED.swap(false, Ordering::SeqCst);
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

impl Command {
    pub fn new() -> Self {
        Self {
            input_filename: None,
            output_filename: None,
            command_buffer: String::new(),
        }
    }

    pub fn read_blif<P>(&mut self, filename: P) -> &mut Self
    where
        P: AsRef<Path>,
    {
        self.input_filename = Some(filename.as_ref().to_path_buf());
        self
    }

    pub fn write_blif<P>(&mut self, filename: P) -> &mut Self
    where
        P: AsRef<Path>,
    {
        self.output_filename = Some(filename.as_ref().to_path_buf());
        self
    }

    fn push_command(&mut self, command: &str) {
        if !self.command_buffer.is_empty() {
            self.command_buffer.push_str("; ");
        }
        self.command_buffer.push_str(command);
    }

    impl_command!(strash);
    impl_command!("&get -n" as amp_get);
    impl_command!("&fraig -x" as amp_fraig);
    impl_command!("&put" as amp_put);
    impl_command!(scorr);
    impl_command!(dc2);
    impl_command!(dretime);
    impl_command!(dch);
    impl_command!("if" as map_if);
    impl_command!(mfs2);
    impl_command!(lutpack);

    pub fn execute(&mut self, abc: &Abc) -> Result<()> {
        let input_filename = self.input_filename.as_ref().ok_or(Error::MissingInput)?;
        let output_filename = self.output_filename.as_ref().ok_or(Error::MissingOutput)?;
        let read_blif = format!("read_blif \"{}\"", input_filename.display());
        if abc.execute_command(&read_blif) != 0 {
            return Err(Error::ReadBlif(input_filename.clone()));
        }
        if abc.execute_command(&self.command_buffer) != 0 {
            return Err(Error::CommandFailed(self.command_buffer.clone()));
        }
        let write_blif = format!("write_blif \"{}\"", output_filename.display());
        if abc.execute_command(&write_blif) != 0 {
            return Err(Error::WriteBlif(output_filename.clone()));
        }
        Ok(())
    }
}

pub struct BlifLutMapper {
    input_filename: PathBuf,
    lut_size: usize,
}

fn generate_lut_library(max_lut_size: usize) -> String {
    (1..=max_lut_size).fold(String::new(), |mut lut_lib, lut_size| {
        let _ = writeln!(lut_lib, "{lut_size} 1.0 1.0\n");
        lut_lib
    })
}

impl BlifLutMapper {
    pub fn new<P>(input_filename: P, lut_size: usize) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            input_filename: input_filename.as_ref().to_path_buf(),
            lut_size,
        }
    }

    pub fn run<P>(&self, abc: &Abc, output_filename: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let output_filename = output_filename.as_ref().to_path_buf();
        let lut_library = generate_lut_library(self.lut_size);
        if abc.set_lut_library(&lut_library) != 0 {
            return Err(Error::SetLutLibrary);
        }
        // https://github.com/YosysHQ/yosys/blob/6583444/passes/techmap/abc.cc#L34
        Command::new()
            .read_blif(&self.input_filename)
            .strash()
            .amp_get()
            .amp_fraig()
            .amp_put()
            .scorr()
            .dc2()
            .dretime()
            .strash()
            .dch()
            .map_if()
            .mfs2()
            .lutpack()
            .write_blif(output_filename)
            .execute(abc)
    }
}

#[cfg(test)]
#[serial_test::serial]
mod tests {
    use super::*;

    #[test]
    fn test_abc_new() {
        let _abc = Abc::new().unwrap();
    }

    #[test]
    fn test_abc_set_lut_library() {
        let lut_library = generate_lut_library(4);
        assert_eq!(lut_library, "1 1.0 1.0\n2 1.0 1.0\n3 1.0 1.0\n4 1.0 1.0\n");
        let abc = Abc::new().unwrap();
        assert_eq!(abc.set_lut_library(&lut_library), 0);
    }

    #[test]
    fn test_abc_is_not_threadsafe() {
        {
            let _abc = Abc::new().unwrap();
            assert!(matches!(Abc::new(), Err(Error::InstanceExists)));
        }
        let _abc = Abc::new().unwrap();
    }
}
