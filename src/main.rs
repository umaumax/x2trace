use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use anyhow::{anyhow, Result};
use cpp_demangle::Symbol;
use log::info;
use structopt::StructOpt;

use x2trace::chrome;
use x2trace::file;
use x2trace::iftrace;
use x2trace::objdump;

#[derive(StructOpt)]
struct IftracerCli {
    #[structopt(parse(from_os_str), help = "Target trace log files")]
    input_files: Vec<std::path::PathBuf>,
    #[structopt(
        long = "bin",
        parse(from_os_str),
        default_value(""),
        help = "Target binary filepath"
    )]
    bin_filepath: std::path::PathBuf,
    #[structopt(long = "text", help = "Deprecated option")]
    text_flag: bool,
    #[structopt(
        long = "bit",
        default_value("auto"),
        help = "Target arch is 32bit or not"
    )]
    bit: String,
    #[structopt(
        long = "function-file-location",
        help = "Disable add function file location to output args field"
    )]
    function_file_location: bool,
    #[structopt(long = "no-demangle", help = "Disable demangle function name")]
    no_demangle: bool,
}

#[derive(StructOpt)]
struct Cli {
    #[structopt(
        short = "p",
        long = "pretty",
        help = "Output tracing json file pretty or not"
    )]
    pretty: bool,
    #[structopt(subcommand)]
    pub sub: CliSubCommands,
}

#[derive(StructOpt)]
enum CliSubCommands {
    #[structopt(name = "iftracer", about = "Select iftracer")]
    IftracerCli(IftracerCli),
}

fn main() -> Result<()> {
    match env::var("RUST_LOG") {
        Ok(_) => {}
        Err(_) => env::set_var("RUST_LOG", "debug"),
    }
    env_logger::init();
    let args = Cli::from_args();
    let ret = match &args.sub {
        CliSubCommands::IftracerCli(sub_args) => run_iftracer_main(&args, &sub_args),
    };
    ret?;
    Ok(())
}

fn parse_elf_bit_option(opt: &str, target_bin_file: &Path) -> Result<bool> {
    match opt {
        "32" => Ok(true),
        "64" => Ok(false),
        "auto" => {
            let elf_bit_size = file::detect_elf_bit_size(target_bin_file);
            match elf_bit_size {
                Ok(file::ElfBitSize::Bit32) => Ok(true),
                Ok(file::ElfBitSize::Bit64) => Ok(false),
                Err(e) => return Err(e),
            }
        }
        s => {
            return Err(anyhow!(
                "Failed parse --bit flag '{}' choose from [32, 64, auto]",
                s
            ))
        }
    }
}

fn run_iftracer_main(args: &Cli, sub_args: &IftracerCli) -> Result<()> {
    info!("[parse trace file step]");
    let mut events = if sub_args.text_flag {
        iftrace::parse_text_files(&sub_args.input_files)?
    } else {
        let bit32flag =
            parse_elf_bit_option(sub_args.bit.as_str(), sub_args.bin_filepath.as_path())?;
        iftrace::parse_binary_files(&sub_args.input_files, bit32flag)?
    };

    let mut address_hash = HashSet::new();
    for event in events.iter() {
        // info!("address: {}", &event.name);
        if event.name.starts_with("0x") {
            address_hash.insert(&event.name);
        }
    }
    let address_list = address_hash.into_iter().collect::<Vec<_>>();

    // rename address to function name by objdump
    if !sub_args.bin_filepath.as_path().to_str().unwrap().is_empty() {
        info!("[objdump step]");
        let objdump_command = match env::var("OBJDUMP") {
            Ok(val) => val,
            Err(_) => "objdump".to_string(),
        };
        let add2info_map =
            objdump::get_addr2info_map(&objdump_command, &sub_args.bin_filepath, &address_list)?;
        info!("{:?}", add2info_map);
        for mut event in &mut events {
            if let Some(info) = add2info_map.get(&event.name) {
                let mut name = info.function_name.to_string();
                if !sub_args.no_demangle {
                    if let Ok(sym) = Symbol::new(&name) {
                        name = sym.to_string();
                    }
                }
                event.name = name;
                if event.event_type == chrome::EventType::DurationEnd {
                    continue;
                }
                if sub_args.function_file_location && !info.file_location.is_empty() {
                    let event_args = event.args.get_or_insert(HashMap::new());
                    event_args.insert(
                        String::from("file_location"),
                        info.file_location.to_string(),
                    );
                }
            }
        }
    }

    info!("[json parse step]");
    let events_json = if args.pretty {
        serde_json::to_string_pretty(&events)?
    } else {
        serde_json::to_string(&events)?
    };
    // info!("{}", events_json);

    info!("[json output step]");
    let mut outfile = File::create("out.json")?;
    outfile.write_all(events_json.as_bytes())?;
    Ok(())
}
