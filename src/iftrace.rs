use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use num_traits::{FromPrimitive, ToPrimitive};

use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::Cursor;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
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

    let zero_duration = Duration::new(0, 0);
    let cur_len = cur.get_ref().len();
    let mut pre_timestamp = zero_duration;
    let mut event_stack = Vec::<chrome::Event>::new();
    while (cur.position() as usize) < cur_len - 1 {
        let mut timestamp_data = cur.read_i32::<LittleEndian>().unwrap();
        let mut exit_flag = false;
        if timestamp_data < 0 {
            timestamp_data *= -1;
            exit_flag = true;
        }
        let mut timestamp = Duration::from_micros(timestamp_data as u64);
        if timestamp == zero_duration {
            log::warn!("get zero timestamp, maybe broken file");
            break;
        }
        // sub offset to distinguish broken file
        timestamp -= Duration::from_micros(1);
        timestamp += pre_timestamp;
        let extra_info = if !exit_flag {
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
            extra_info
        } else {
            ToPrimitive::to_u64(&ExtraFlag::Exit).unwrap() << (64 - 2)
        };
        // debug!(
        // "timestamp = {:?}, extra_info = {:#02x}",
        // timestamp, extra_info
        // );
        let extra_flag = FromPrimitive::from_u64(extra_info >> (64 - 2));
        let event: chrome::Event = match extra_flag {
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

                let text = if event_type == chrome::EventType::DurationBegin {
                    let mut text = vec![0; text_size as usize];
                    cur.read_exact(&mut text).unwrap();
                    let text_align = 4;
                    let dummy_padding_size =
                        (((text_size) + (text_align - 1)) & !(text_align - 1)) - text_size;
                    cur.seek(SeekFrom::Current(dummy_padding_size as i64))
                        .unwrap();
                    String::from_utf8(text).unwrap()
                } else {
                    String::from("")
                };

                chrome::Event {
                    args: None,
                    category: String::from("external"),
                    duration: Duration::from_millis(0),
                    event_type: event_type,
                    name: text,
                    process_id: pid,
                    thread_id: tid,
                    timestamp: timestamp,
                }
            }
            None => {
                return Err(anyhow!("Failed parse binary at {}", cur.position()));
            }
        };
        // if timestamp is same chrome tracing viewer doesn't show the item,
        // so add virtual duration to end timestamp
        match event.event_type {
            chrome::EventType::DurationBegin => {
                event_stack.push(event);
            }
            chrome::EventType::DurationEnd => {
                let begin_event = event_stack.pop().unwrap();
                let mut target_event = begin_event;
                let mut duration = timestamp - target_event.timestamp;
                if duration == zero_duration {
                    let virtual_duration = Duration::from_nanos(200);
                    duration = virtual_duration;
                    let event_args = target_event.args.get_or_insert(HashMap::new());
                    event_args.insert(String::from("virtual_duration"), String::from("true"));
                }
                target_event.duration = duration;
                target_event.event_type = chrome::EventType::Complete;
                events.push(target_event);
            }
            _ => {}
        }
        pre_timestamp = timestamp;
    }
    if event_stack.len() > 0 {
        log::warn!(
            "parsed event stack size is {}, maybe broken file",
            event_stack.len()
        );
    }
    Ok(events)
}
