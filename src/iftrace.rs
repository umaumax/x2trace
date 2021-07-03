use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use num_traits::FromPrimitive;

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
    NormalEnter = 0x0,
    InternalOrExternalEnter = 0x1,
    InternalOrNormalExit = 0x2,
    ExternalExit = 0x3,
}

fn update_to_complete_event(event: &mut chrome::Event, end_timestamp: Duration) {
    let mut duration = end_timestamp - event.timestamp;
    let zero_duration = Duration::new(0, 0);
    if duration == zero_duration {
        let virtual_duration = Duration::from_nanos(200);
        duration = virtual_duration;
        let event_args = event.args.get_or_insert(HashMap::new());
        event_args.insert(String::from("virtual_duration"), String::from("true"));
    }
    event.duration = duration;
    event.event_type = chrome::EventType::Complete;
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
    let mut pre_timestamp = Duration::new(0, 0);
    let mut event_stack = Vec::<chrome::Event>::new();
    while (cur.position() as usize) < cur_len - 1 {
        let timestamp_with_extra_flag = cur.read_u32::<LittleEndian>().unwrap();
        if timestamp_with_extra_flag == 0 {
            log::warn!("get zero timestamp, maybe broken file");
            break;
        }
        // sub offset which used to distinguish broken file or not
        let dummy_offset = 1;
        let extra_flag =
            FromPrimitive::from_u32((timestamp_with_extra_flag - dummy_offset) >> (32 - 2))
                .unwrap();
        let mut timestamp =
            Duration::from_micros((timestamp_with_extra_flag & !((0x3) << (32 - 2))) as u64);

        timestamp += pre_timestamp;

        let event: chrome::Event = match extra_flag {
            ExtraFlag::NormalEnter => {
                let func_addr = if !bit32_flag {
                    cur.read_u64::<LittleEndian>().unwrap()
                } else {
                    cur.read_u32::<LittleEndian>().unwrap() as u64
                };
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
            ExtraFlag::InternalOrExternalEnter => {
                // debug!("internal or external enter, func_addr");
                chrome::Event {
                    args: None,
                    category: String::from("internal or external"),
                    duration: Duration::from_millis(0),
                    event_type: chrome::EventType::DurationBegin,
                    name: String::from(""),
                    process_id: pid,
                    thread_id: tid,
                    timestamp: timestamp,
                }
            }
            ExtraFlag::InternalOrNormalExit => {
                // debug!("internal or normal exit");
                let mut event = event_stack.pop().unwrap();
                update_to_complete_event(&mut event, timestamp);
                if event.name == "" {
                    event.category = String::from("internal");
                    event.name = String::from("[internal]");
                }
                event
            }
            ExtraFlag::ExternalExit => {
                // debug!("external exit");

                let text_size = cur.read_u32::<LittleEndian>().unwrap();
                let mut text_buf = vec![0; text_size as usize];
                cur.read_exact(&mut text_buf).unwrap();
                let text_align = 4;
                let dummy_padding_size =
                    (((text_size) + (text_align - 1)) & !(text_align - 1)) - text_size;
                cur.seek(SeekFrom::Current(dummy_padding_size as i64))
                    .unwrap();
                let text = String::from_utf8(text_buf).unwrap();

                let mut event = event_stack.pop().unwrap();
                event.category = String::from("external");
                event.name = text;
                update_to_complete_event(&mut event, timestamp);
                event
            }
        };
        // if timestamp is same chrome tracing viewer doesn't show the item,
        // so add virtual duration to end timestamp
        match event.event_type {
            chrome::EventType::DurationBegin => {
                event_stack.push(event);
            }
            chrome::EventType::Complete => {
                events.push(event);
            }
            _ => {
                return Err(anyhow!("invalid event type '{:?}'", event.event_type));
            }
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
