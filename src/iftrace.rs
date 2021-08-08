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

pub fn parse_binary_files(files: &Vec<PathBuf>, bit32_flag: bool) -> Result<Vec<chrome::Event>> {
    let mut events: Vec<chrome::Event> = Vec::new();
    if files.len() == 0 {
        return Ok(events);
    }
    for file in files {
        let mut result = parse_binary_file(&file, bit32_flag)?;
        events.append(&mut result);
    }
    Ok(events)
}

fn parse_binary_file(filename: &PathBuf, bit32_flag: bool) -> Result<Vec<chrome::Event>> {
    let buffer = get_file_as_byte_vec(filename)?;
    parse_binary_buffer(buffer, bit32_flag)
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
            "Failed parse line '{}' required format! is 'tid timestamp(us) action[enter/exit] caller_address callee_address' e.g. '239949354 1624633549138701 enter 0x1002d47e8 0x100166be0'",
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
        instant_scope: None,
        scope: None,
        id: None,
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

#[derive(PartialEq, FromPrimitive, ToPrimitive)]
enum ExtraFlag {
    NormalEnter = 0x0,
    ExtendEnter = 0x1,
    NormalExit = 0x2,
    ExtendExit = 0x3,
}

#[derive(PartialEq, FromPrimitive, ToPrimitive)]
enum ExtendType {
    DurationEnter = 0x0,
    DurationExit = 0x1,
    AsyncEnter = 0x2,
    AsyncExit = 0x3,
    Instant = 0x4,
}
impl From<ExtendType> for chrome::EventType {
    fn from(extend_type: ExtendType) -> Self {
        match extend_type {
            ExtendType::DurationEnter => chrome::EventType::DurationBegin,
            ExtendType::DurationExit => chrome::EventType::DurationEnd,
            ExtendType::AsyncEnter => chrome::EventType::AsyncNestableStart,
            ExtendType::AsyncExit => chrome::EventType::AsyncNestableEnd,
            ExtendType::Instant => chrome::EventType::Instant,
        }
    }
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

fn read_text_form_binary(cur: &mut std::io::Cursor<Vec<u8>>) -> String {
    let text_size = cur.read_u32::<LittleEndian>().unwrap();
    let mut text_buf = vec![0; text_size as usize];
    cur.read_exact(&mut text_buf).unwrap();
    let text_align = 4;
    let dummy_padding_size = (((text_size) + (text_align - 1)) & !(text_align - 1)) - text_size;
    cur.seek(SeekFrom::Current(dummy_padding_size as i64))
        .unwrap();
    String::from_utf8(text_buf).unwrap()
}

fn parse_binary_buffer(buffer: Vec<u8>, bit32_flag: bool) -> Result<Vec<chrome::Event>> {
    let mut events: Vec<chrome::Event> = Vec::with_capacity(10);
    let mut cur = Cursor::new(buffer);
    let cur_len = cur.get_ref().len();
    let mut event_stack = Vec::<chrome::Event>::new();

    let base_timestamp = Duration::from_micros(cur.read_u64::<LittleEndian>().unwrap());
    let mut pre_timestamp = base_timestamp;
    let pid = cur.read_i32::<LittleEndian>().unwrap() as u32;
    let tid = cur.read_i32::<LittleEndian>().unwrap() as u32;
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
                    instant_scope: None,
                    scope: None,
                    id: None,
                    timestamp: timestamp,
                }
            }
            ExtraFlag::ExtendEnter => {
                // debug!("internal or external enter, func_addr");
                let extend_type: ExtendType =
                    FromPrimitive::from_u32(cur.read_u32::<LittleEndian>().unwrap()).unwrap();
                let event_type = chrome::EventType::from(extend_type);
                let mut event = chrome::Event {
                    args: None,
                    category: String::from("extend"),
                    duration: Duration::from_millis(0),
                    event_type: event_type,
                    name: String::from(""),
                    process_id: pid,
                    thread_id: tid,
                    instant_scope: None,
                    scope: None,
                    id: None,
                    timestamp: timestamp,
                };
                if event_type == chrome::EventType::AsyncNestableStart {
                    event.name = read_text_form_binary(&mut cur);
                    event.id = Some(event.name.clone());
                    event.scope = Some(tid.to_string());
                }
                event
            }
            ExtraFlag::NormalExit => {
                // debug!("internal or normal exit");
                let mut event = event_stack.pop().unwrap();
                update_to_complete_event(&mut event, timestamp);
                if event.name == "" {
                    event.category = String::from("internal");
                    event.name = String::from("[internal]");
                }
                event
            }
            ExtraFlag::ExtendExit => {
                // debug!("external exit");
                let extend_type: ExtendType =
                    FromPrimitive::from_u32(cur.read_u32::<LittleEndian>().unwrap()).unwrap();
                let event_type = chrome::EventType::from(extend_type);
                let text = read_text_form_binary(&mut cur);
                if event_type == chrome::EventType::DurationEnd {
                    let mut event = event_stack.pop().unwrap();
                    event.category = String::from("external");
                    event.name = text;
                    update_to_complete_event(&mut event, timestamp);
                    event
                } else {
                    let instant_scope = if event_type == chrome::EventType::Instant {
                        Some(chrome::InstantScope::Global)
                    } else {
                        None
                    };
                    let id = if event_type == chrome::EventType::AsyncNestableEnd {
                        Some(text.clone())
                    } else {
                        None
                    };
                    let scope = if event_type == chrome::EventType::AsyncNestableEnd {
                        Some(tid.to_string())
                    } else {
                        None
                    };
                    chrome::Event {
                        args: None,
                        category: String::from("extend"),
                        duration: Duration::from_millis(0),
                        event_type: event_type,
                        name: text,
                        process_id: pid,
                        thread_id: tid,
                        instant_scope: instant_scope,
                        scope: scope,
                        id: id,
                        timestamp: timestamp,
                    }
                }
            }
        };
        // if timestamp is same chrome tracing viewer doesn't show the item,
        // so add virtual duration to end timestamp
        match event.event_type {
            chrome::EventType::DurationBegin => {
                event_stack.push(event);
            }
            chrome::EventType::AsyncNestableStart
            | chrome::EventType::AsyncNestableEnd
            | chrome::EventType::Instant
            | chrome::EventType::Complete => {
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
        for event in event_stack {
            events.push(event);
        }
    }
    Ok(events)
}
