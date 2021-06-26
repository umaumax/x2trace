use std::env;
use std::fs::File;
use std::io::Write;

use log::info;
use structopt::StructOpt;

use x2trace::iftrace;

#[derive(StructOpt)]
struct Cli {
    #[structopt(parse(from_os_str))]
    input_filepath: std::path::PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let args = Cli::from_args();
    let filepath = args.input_filepath;
    let events = iftrace::parse_file(&filepath)?;

    let events_json = serde_json::to_string_pretty(&events)?;
    info!("{}", events_json);

    let mut outfile = File::create("out.json")?;
    outfile.write_all(events_json.as_bytes())?;
    Ok(())
}
