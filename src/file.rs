use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::str;

use anyhow::{anyhow, Result};

#[derive(Debug)]
pub enum ElfBitSize {
    Bit32,
    Bit64,
}

pub fn detect_elf_bit_size(filepath: &Path) -> Result<ElfBitSize> {
    let child = Command::new("file")
        .arg(filepath)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("file command failed to start");
    let output = child.wait_with_output()?;
    let exit_code = output.status.code().unwrap();
    if exit_code != 0 {
        let stderr_output = String::from_utf8(output.stderr).unwrap();
        return Err(anyhow!(
            "Failed to run file command: exit_code={}, stderr={}",
            exit_code,
            stderr_output
        ));
    }

    let line = str::from_utf8(&output.stdout).unwrap();

    if line.contains("32-bit") {
        Ok(ElfBitSize::Bit32)
    } else if line.contains("64-bit") {
        Ok(ElfBitSize::Bit64)
    } else {
        Err(anyhow!("Failed parse file result '{}'", line))
    }
}
