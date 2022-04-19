use anyhow::Context;
use anyhow::Result;

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

pub fn parse_proc_maps(proc_maps_filename: impl AsRef<Path>) -> Result<HashMap<String, u64>> {
    let input = fs::read_to_string(&proc_maps_filename).with_context(|| {
        format!(
            "parse_proc_maps(): Failed to open file {:?}",
            proc_maps_filename.as_ref()
        )
    })?;
    let maps = rsprocmaps::from_str(&input);

    let mut filename2addr_map: HashMap<String, u64> = HashMap::new();

    for map in maps {
        if map.is_err() {
            // NOTE: failed to parse without blank space or memory map name
            continue;
        }
        let map = map?;
        if map.offset == 0 {
            if let rsprocmaps::Pathname::Path(path) = map.pathname {
                let filepath = PathBuf::from(path);
                filename2addr_map.insert(
                    filepath
                        .file_name()
                        .with_context(|| format!("failed to parse file name from {:?}", filepath))?
                        .to_string_lossy()
                        .to_string(),
                    map.address_range.begin,
                );
            }
        }
    }

    Ok(filename2addr_map)
}
