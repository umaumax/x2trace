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
        let add2func_map =
            objdump::get_addr2func_map("objdump", &args.bin_filepath, &address_list)?;
        info!("{:?}", add2func_map);
        for mut event in &mut events {
            if let Some(func_name) = add2func_map.get(&event.name) {
                event.name = func_name.to_string();
            }
        }
    }

    let events_json = serde_json::to_string_pretty(&events)?;
    info!("{}", events_json);
    let mut outfile = File::create("out.json")?;
    outfile.write_all(events_json.as_bytes())?;
    Ok(())
}
