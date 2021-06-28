use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::Cursor;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

use crate::chrome;

pub fn parse_text_files(files: &Vec<PathBuf>) -> Result<Vec<chrome::Event>> {
    let mut events: Vec<chrome::Event> = Vec::new();
    for file in files {
        let mut result = parse_text_file(&file)?;
        events.append(&mut result);
    }
    Ok(events)
}

pub fn parse_text_file(filename: &PathBuf) -> Result<Vec<chrome::Event>> {
    let buffer = get_file_as_byte_vec(filename)?;
    parse_text_buffer(buffer)
}

fn file_name_to_tid(filename: &str) -> Result<u32> {
    let fields: Vec<&str> = filename.split('.').collect();
    let tid = fields.last().unwrap().parse::<u32>().unwrap();
    Ok(tid)
}

pub fn parse_binary_files(files: &Vec<PathBuf>, bit32_flag: bool) -> Result<Vec<chrome::Event>> {
    let mut events: Vec<chrome::Event> = Vec::new();
    if files.len() == 0 {
        return Ok(events);
    }
    let pid = file_name_to_tid(files[0].to_str().unwrap()).unwrap();
    for file in files {
        let tid = file_name_to_tid(file.to_str().unwrap()).unwrap();
        let mut result = parse_binary_file(&file, bit32_flag, pid, tid)?;
        events.append(&mut result);
    }
    Ok(events)
}

fn parse_binary_file(
    filename: &PathBuf,
    bit32_flag: bool,
    pid: u32,
    tid: u32,
) -> Result<Vec<chrome::Event>> {
    let buffer = get_file_as_byte_vec(filename)?;
    parse_binary_buffer(buffer, bit32_flag, pid, tid)
}

fn get_file_as_byte_vec(filename: &PathBuf) -> Result<Vec<u8>> {
    let mut f = File::open(&filename).expect("no file found");
    let metadata = fs::metadata(&filename).expect("unable to read metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    f.read(&mut buffer).expect("buffer overflow");
    Ok(buffer)
}

fn parse_line_to_event(line: &str) -> Result<chrome::Event> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() != 5 {
        return Err(anyhow!(
            "Failed parse line '{}' required format! is 'tid timestamp(us) action[enter/exit] caller_addressv callee_adderss' e.g. '239949354 1624633549138701 enter 0x1002d47e8 0x100166be0'",
            line
        ));
    }
    let tid = fields[0].parse::<u32>()?;
    let timestamp = Duration::from_micros(fields[1].parse::<u64>()?);
    let action = fields[2];
    let _caller_address = fields[3];
    let callee_address = fields[4];

    let func_name: String = callee_address.to_string();

    let event_type = match action {
        "enter" => chrome::EventType::DurationBegin,
        "exit" => chrome::EventType::DurationEnd,
        _ => {
            return Err(anyhow!(
                "Failed parse line '{}' invalid action '{}'",
                line,
                action
            ))
        }
    };

    let event = chrome::Event {
        args: None,
        category: String::from("category"),
        duration: Duration::from_millis(0),
        event_type: event_type,
        name: func_name,
        process_id: 1234,
        thread_id: tid,
        timestamp: timestamp,
    };
    Ok(event)
}

fn parse_text_buffer(buffer: Vec<u8>) -> Result<Vec<chrome::Event>> {
    let mut events: Vec<chrome::Event> = Vec::with_capacity(10);
    let mut cur = Cursor::new(buffer);

    let mut line = String::new();
    while cur.read_line(&mut line).unwrap() > 0 {
        // debug!("iftracer input line: {:?}", line);
        let event = parse_line_to_event(&line)?;
        events.push(event);
        line.clear();
    }
    Ok(events)
}

#[derive(FromPrimitive, ToPrimitive)]
enum ExtraFlag {
    Enter = 0x0,
    Exit = 0x1,
    Internal = 0x2,
    External = 0x3,
}

#[derive(FromPrimitive, ToPrimitive)]
enum InternalProcessFlag {
    Enter = 0x0,
    Exit = 0x1,
}
#[derive(FromPrimitive, ToPrimitive)]
enum ExternalProcessFlag {
    Enter = 0x0,
    Exit = 0x1,
}

fn parse_binary_buffer(
    buffer: Vec<u8>,
    bit32_flag: bool,
    pid: u32,
    tid: u32,
) -> Result<Vec<chrome::Event>> {
    let mut events: Vec<chrome::Event> = Vec::with_capacity(10);
    let mut cur = Cursor::new(buffer);

    let cur_len = cur.get_ref().len();
    while (cur.position() as usize) < cur_len - 1 {
        let timestamp = Duration::from_micros(cur.read_u64::<LittleEndian>().unwrap());
        let extra_info = if !bit32_flag {
            cur.read_u64::<LittleEndian>().unwrap()
        } else {
            let extra_info_32bit = cur.read_u32::<LittleEndian>().unwrap();
            match FromPrimitive::from_u64((extra_info_32bit >> (32 - 2)) as u64) {
                Some(ExtraFlag::Enter) | Some(ExtraFlag::Exit) => {
                    ((extra_info_32bit as u64 >> (32 - 2)) << (64 - 2)
                        | extra_info_32bit as u64 & ((0x1 << (32 - 2)) - 1))
                        as u64
                }
                Some(ExtraFlag::Internal) | Some(ExtraFlag::External) => {
                    ((extra_info_32bit as u64 >> (32 - 3)) << (64 - 3)
                        | extra_info_32bit as u64 & ((0x1 << (32 - 3)) - 1))
                        as u64
                }
                None => {
                    return Err(anyhow!("Failed parse binary at {}", cur.position()));
                }
            }
        };
        // debug!(
        // "timestamp = {:?}, extra_info = {:#02x}",
        // timestamp, extra_info
        // );
        let event: chrome::Event = match FromPrimitive::from_u64(extra_info >> (64 - 2)) {
            Some(ExtraFlag::Enter) => {
                let func_addr = extra_info & ((0x1 << (64 - 2 - 2)) - 1);
                // debug!("enter, func_addr = {:#02x}", func_addr);
                chrome::Event {
                    args: None,
                    category: String::from("call"),
                    duration: Duration::from_millis(0),
                    event_type: chrome::EventType::DurationBegin,
                    name: String::from("0x") + &format!("{:x}", func_addr),
                    process_id: pid,
                    thread_id: tid,
                    timestamp: timestamp,
                }
            }
            Some(ExtraFlag::Exit) => {
                let func_addr = extra_info & ((0x1 << (64 - 2 - 2)) - 1);
                // debug!("exit, func_addr = {:#02x}", func_addr);
                chrome::Event {
                    args: None,
                    category: String::from("call"),
                    duration: Duration::from_millis(0),
                    event_type: chrome::EventType::DurationEnd,
                    name: String::from("0x") + &format!("{:x}", func_addr),
                    process_id: pid,
                    thread_id: tid,
                    timestamp: timestamp,
                }
            }
            Some(ExtraFlag::Internal) => {
                // debug!("internal");

                let event = extra_info & ((0x1 << (64 - 2)) - 1);
                let event_flag = event >> (64 - 2 - 1);

                let event_type = match FromPrimitive::from_u64(event_flag) {
                    Some(InternalProcessFlag::Enter) => chrome::EventType::DurationBegin,
                    Some(InternalProcessFlag::Exit) => chrome::EventType::DurationEnd,
                    None => {
                        return Err(anyhow!(
                            "Failed parse internal process flag at {}",
                            cur.position()
                        ));
                    }
                };
                chrome::Event {
                    args: None,
                    category: String::from("internal"),
                    duration: Duration::from_millis(0),
                    event_type: event_type,
                    name: String::from("[internal processing]"),
                    process_id: pid,
                    thread_id: tid,
                    timestamp: timestamp,
                }
            }
            Some(ExtraFlag::External) => {
                // debug!("external");

                let event = extra_info & ((0x1 << (64 - 2)) - 1);
                let event_flag = event >> (64 - 2 - 1);
                let text_size = event & ((0x1 << (64 - 2 - 1)) - 1);

                let event_type = match FromPrimitive::from_u64(event_flag) {
                    Some(ExternalProcessFlag::Enter) => chrome::EventType::DurationBegin,
                    Some(ExternalProcessFlag::Exit) => chrome::EventType::DurationEnd,
                    None => {
                        return Err(anyhow!(
                            "Failed parse external process flag at {}",
                            cur.position()
                        ));
                    }
                };

                let mut text = Vec::with_capacity(text_size as usize);
                cur.read_exact(&mut text).unwrap();

                chrome::Event {
                    args: None,
                    category: String::from("external"),
                    duration: Duration::from_millis(0),
                    event_type: event_type,
                    name: String::from_utf8(text).unwrap(),
                    process_id: pid,
                    thread_id: tid,
                    timestamp: timestamp,
                }
            }
            None => {
                return Err(anyhow!("Failed parse binary at {}", cur.position()));
            }
        };
        events.push(event);
    }
    Ok(events)
}
