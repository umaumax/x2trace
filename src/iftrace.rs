use anyhow::Result;
use std::fs;
use std::fs::File;
use std::io::Read;

use crate::chrome;

fn get_file_as_byte_vec(filename: &String) -> Result<Vec<u8>> {
    let mut f = File::open(&filename).expect("no file found");
    let metadata = fs::metadata(&filename).expect("unable to read metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    f.read(&mut buffer).expect("buffer overflow");
    Ok(buffer)
}

pub fn parse_buffer(buffer: Vec<u8>) -> Result<Vec<chrome::Event>> {
    let mut events: Vec<chrome::Event> = Vec::with_capacity(10);
    Ok(events)
}
