use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::time::Duration;

use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::Cursor;
use std::io::Read;
use std::path::PathBuf;

use crate::chrome;

pub fn parse_file(filename: &PathBuf) -> Result<Vec<chrome::Event>> {
    let buffer = get_file_as_byte_vec(filename)?;
    parse_buffer(buffer)
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

    let mut event_args = HashMap::new();
    event_args.insert(String::from("args"), String::from("value"));
    let event = chrome::Event {
        args: Some(event_args),
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

fn parse_buffer(buffer: Vec<u8>) -> Result<Vec<chrome::Event>> {
    let mut events: Vec<chrome::Event> = Vec::with_capacity(10);
    let mut cur = Cursor::new(buffer);

    let mut line = String::new();
    while cur.read_line(&mut line).unwrap() > 0 {
        debug!("iftracer input line: {:?}", line);
        let event = parse_line_to_event(&line)?;
        events.push(event);
        line.clear();
    }
    Ok(events)
}
