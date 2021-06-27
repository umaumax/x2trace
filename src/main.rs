use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::Write;

use log::info;
use structopt::StructOpt;

use x2trace::iftrace;
use x2trace::objdump;

#[derive(StructOpt)]
struct Cli {
    #[structopt(parse(from_os_str))]
    input_filepath: std::path::PathBuf,
    #[structopt(long = "bin", parse(from_os_str), default_value(""))]
    bin_filepath: std::path::PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let args = Cli::from_args();
    let filepath = args.input_filepath;
    let mut events = iftrace::parse_file(&filepath)?;

    let mut address_hash = HashSet::new();
    for event in events.iter() {
        info!("address: {}", &event.name);
        address_hash.insert(&event.name);
    }
    let address_list = address_hash.into_iter().collect::<Vec<_>>();

    // rename address to function name by objdump
    if !args.bin_filepath.as_path().to_str().unwrap().is_empty() {
        let objdump_command = match env::var("OBJDUMP") {
            Ok(val) => val,
            Err(_) => "objdump".to_string(),
        };
        let add2info_map =
            objdump::get_addr2info_map(&objdump_command, &args.bin_filepath, &address_list)?;
        info!("{:?}", add2info_map);
        for mut event in &mut events {
            if let Some(info) = add2info_map.get(&event.name) {
                event.name = info.function_name.to_string();
                if !info.file_location.is_empty() {
                    let mut event_args = HashMap::new();
                    event_args.insert(
                        String::from("file_location"),
                        info.file_location.to_string(),
                    );
                    event.args = Some(event_args);
                }
            }
        }
    }

    let events_json = serde_json::to_string_pretty(&events)?;
    info!("{}", events_json);
    let mut outfile = File::create("out.json")?;
    outfile.write_all(events_json.as_bytes())?;
    Ok(())
}
