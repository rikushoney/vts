use thiserror::Error;

use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

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

pub struct Abc(*mut vts_abc_sys::AbcFrame);

impl Abc {
    pub fn new() -> Result<Abc> {
        let locked = ABC_LOCKED.swap(true, Ordering::SeqCst);
        if !locked {
            unsafe {
                vts_abc_sys::abc_start();
            }
            Ok(Abc(unsafe { vts_abc_sys::abc_get_global_frame() }))
        } else {
            Err(Error::InstanceExists)
        }
    }

    pub(crate) fn execute_command(&self, command: &str) -> i32 {
        let command = CString::new(command).expect("command should not contain nul bytes");
        unsafe { vts_abc_sys::abc_execute_command(self.0, command.as_ptr()) }
    }

    pub(crate) fn set_lut_library(&self, library: &str) -> i32 {
        let lut_library = CString::new(library).expect("lut library should not contain nul bytes");
        unsafe { vts_abc_sys::abc_frame_set_lut_library(self.0, lut_library.as_ptr()) }
    }
}

impl Drop for Abc {
    fn drop(&mut self) {
        unsafe { vts_abc_sys::abc_stop() };
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

    pub fn strash(&mut self) -> &mut Self {
        self.push_command("strash");
        self
    }

    pub fn amp_get(&mut self) -> &mut Self {
        self.push_command("&get -n");
        self
    }

    pub fn amp_fraig(&mut self) -> &mut Self {
        self.push_command("&fraig -x");
        self
    }

    pub fn amp_put(&mut self) -> &mut Self {
        self.push_command("&put");
        self
    }

    pub fn scorr(&mut self) -> &mut Self {
        self.push_command("scorr");
        self
    }

    pub fn dc2(&mut self) -> &mut Self {
        self.push_command("dc2");
        self
    }

    pub fn dretime(&mut self) -> &mut Self {
        self.push_command("dretime");
        self
    }

    pub fn dch(&mut self) -> &mut Self {
        self.push_command("dch -f");
        self
    }

    pub fn map_if(&mut self) -> &mut Self {
        self.push_command("if");
        self
    }

    pub fn mfs2(&mut self) -> &mut Self {
        self.push_command("mfs2");
        self
    }

    pub fn lutpack(&mut self) -> &mut Self {
        self.push_command("lutpack");
        self
    }

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
        if abc.set_lut_library(&format!("{} 1 1", self.lut_size)) != 0 {
            return Err(Error::SetLutLibrary);
        }
        // https://github.com/YosysHQ/yosys/blob/65834440add07421a15291551037a645d55a00aa/passes/techmap/abc.cc#L34
        Command::new()
            .read_blif(&self.input_filename)
            .strash()
            .amp_get()
            .amp_fraig()
            .amp_put()
            .scorr()
            .dc2()
            .dretime()
            .dch()
            .map_if()
            .mfs2()
            .lutpack()
            .write_blif(output_filename)
            .execute(abc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_abc() {
        let _abc = Abc::new().unwrap();
    }
}
