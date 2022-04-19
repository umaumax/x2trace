use std::collections::HashMap;
use std::io::BufRead;
use std::io::Cursor;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use anyhow::{anyhow, Result};

#[derive(Debug)]
pub struct AddressInformation {
    pub address: String,
    pub file_location: String,
    pub function_name: String,
}

pub fn get_addr2info_map(
    objdump_command: &str,
    filepath: &PathBuf,
    base_address_str: &str,
    address_list: &Vec<&String>,
) -> Result<HashMap<String, AddressInformation>> {
    let base_address = i64::from_str_radix(base_address_str.trim_start_matches("0x"), 16)?;

    let child = Command::new(objdump_command)
        .arg("--disassemble")
        .arg("--prefix-addresses")
        .arg("--line-numbers")
        .arg(filepath)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("objdump command failed to start");
    // TODO: read async for speed up
    let output = child.wait_with_output()?;
    let exit_code = output.status.code().unwrap();
    if exit_code != 0 {
        let stderr_output = String::from_utf8(output.stderr).unwrap();
        return Err(anyhow!(
            "Failed to run objdump command: exit_code={}, stderr={}\n(Please set a correct architecture objdump command by 'OBJDUMP' environment variable.)",
            exit_code,
            stderr_output
        ));
    }
    let mut addr2info_map: HashMap<String, AddressInformation> = HashMap::new();

    let mut address_list_clone = address_list.clone();
    address_list_clone.sort();
    let address_list_sorted: Vec<String> = address_list_clone
        .iter()
        .map(|x| {
            let x = x.to_lowercase();
            if x.starts_with("0x") {
                x.strip_prefix("0x").unwrap().to_string()
            } else {
                x
            }
        })
        .collect();
    // for addr in address_list_sorted.iter() {
    // info!("address: {}", addr);
    // }
    let mut address_list_index = 0;
    let address_list_length = address_list_sorted.len();
    // TODO: get information of file line
    let mut cur = Cursor::new(output.stdout);
    let mut line = String::new();

    let mut addr_file_location = String::new();
    while cur.read_line(&mut line).unwrap() > 0 && address_list_index < address_list_length {
        if line.starts_with("/") {
            // filepath information
            addr_file_location = line.trim_end().to_string();
        }
        // debug!("objdump output line: {:?}", line);
        if line.starts_with("0") || line.starts_with(" ") {
        } else {
            line.clear();
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() <= 2 {
            return Err(anyhow!("Failed parse line '{}'", line));
        }
        let address = fields[0].trim_start_matches(|c| c == '0' || c == ' ');
        let runtime_address = base_address + i64::from_str_radix(address, 16)?;
        let address = &format!("{:x}", runtime_address);
        let func_name = fields[1].trim_matches(|c| c == '<' || c == '>');
        while address_list_index < address_list_length {
            let target_address = &address_list_sorted[address_list_index];
            if address == target_address {
                // found target address
                let hex_address = String::from("0x") + &target_address;
                let address_information = AddressInformation {
                    address: hex_address.clone(),
                    file_location: addr_file_location.clone(),
                    function_name: String::from(func_name),
                };
                addr2info_map.insert(hex_address, address_information);
                // debug!("objdump hit address: {:?}", line);
                address_list_index += 1;
                break;
            } else if address < target_address {
                // continue to search target address
                break;
            } else {
                // debug!("objdump output line: {:?}", line);
                // skipped to search target address
                address_list_index += 1;
                continue;
            }
        }
        line.clear();
    }
    Ok(addr2info_map)
}
