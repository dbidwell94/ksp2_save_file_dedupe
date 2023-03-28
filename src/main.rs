use std::{
    cell::RefCell,
    collections::{HashSet, VecDeque},
    error::Error,
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    sync::Arc,
    thread::{self, JoinHandle},
};

use serde::{Deserialize, Serialize};
use serde_json::from_reader;

use crate::ksp::ObjectEvent;
mod ksp;

#[derive(Debug, Serialize, Deserialize, Clone)]
enum Format {
    MB,
    GB,
    KB,
    B,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct ByteThreshold {
    size: f64,
    format: Format,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct KspConfig {
    save_location: String,
    byte_threshold: ByteThreshold,
    max_threads: u8,
    minify: bool
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("Checking for binary config file...");
    let config_file = OpenOptions::new().read(true).open("./config.json")?;
    let reader = BufReader::new(config_file);

    println!("\nParsing config...\n");
    let config = from_reader::<BufReader<File>, KspConfig>(reader)?;
    let ksp_dir = &config.save_location;

    let saves = get_saves(ksp_dir)?;

    dedupe_saves(saves, config)?;

    Ok(())
}

fn size_format_to_bytes(size: f64, format: &Format) -> u64 {
    match format {
        Format::B => size as u64,
        Format::KB => size as u64 * 1000,
        Format::MB => size as u64 * 1000 * 1000,
        Format::GB => size as u64 * 1000 * 1000 * 1000,
    }
}

fn get_saves(base_folder: &str) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut folder_queue: VecDeque<String> = VecDeque::new();
    let mut saves: Vec<PathBuf> = Vec::new();

    folder_queue.push_front(base_folder.to_owned());

    while folder_queue.len() > 0 {
        let folder_name = folder_queue.pop_back().unwrap();

        match fs::read_dir(&folder_name) {
            Ok(dir) => {
                for sub_dir in dir {
                    folder_queue.push_front(sub_dir?.path().to_str().unwrap().to_owned());
                }
                println!("Checking {0} for saves...", &folder_name);
            }
            _ => {
                let file_path = Path::new(&folder_name).to_owned();

                if file_path.extension().unwrap() != "json" {
                    continue;
                }

                saves.push(file_path);
            }
        }
    }

    Ok(saves)
}

fn dedupe_saves(saves: Vec<PathBuf>, config: KspConfig) -> Result<(), Box<dyn Error>> {
    println!(
        "\nDeduping save files using {0} thread(s)...\n",
        &config.max_threads
    );

    let config = Arc::new(config);
    let mut threads: Vec<JoinHandle<()>> = Vec::new();

    for save in saves {
        let arc_config = config.clone();
        threads.push(thread::spawn(move || {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&save)
                .map_err(|e| RefCell::new(e))
                .unwrap();

            let metadata = file.metadata().unwrap();
            println!("Parsing {0}", save.to_str().unwrap());

            let original_file_size = metadata.len();

            let threshold_size = size_format_to_bytes(
                arc_config.byte_threshold.size,
                &arc_config.byte_threshold.format,
            );

            if metadata.len() < threshold_size {
                println!(
                    "File size of {0}MB does not exceed threshold of {1}MB\n",
                    original_file_size / 1000 / 1000,
                    threshold_size / 1000 / 1000
                );
                return ();
            }
            match parse_save(file, save) {
                Ok(new_file_size) => {
                    println!(
                        "Successfully parsed file. New file size is {0}MB which is {1}B smaller\n",
                        new_file_size / 1000 / 1000,
                        original_file_size - new_file_size
                    );
                    return ();
                }
                _ => {
                    println!("An error ocurred parsing the file. File is either not a valid save or possibly corrupted.\n");
                    return ();
                }
            }
        }));
    }

    for thread in threads {
        match thread.join() {
            Err(_) => {}
            _ => {}
        }
    }

    Ok(())
}

fn parse_save(save: File, file_path: PathBuf) -> Result<u64, Box<dyn Error>> {
    let reader = BufReader::new(save);

    let mut save_data = serde_json::from_reader::<BufReader<File>, ksp::KspSaveData>(reader)?;
    println!("found object events");
    dedupe_save(&mut save_data.travel_log_data)?;

    Ok(save_changes(save_data, file_path)?)
}

fn dedupe_save(save: &mut ksp::TravelLogData) -> Result<(), Box<dyn Error>> {
    println!("Deduping save file.");
    println!("Original array length: {0}", &save.object_events.len());

    let set = save
        .object_events
        .drain(..)
        .collect::<HashSet<ObjectEvent>>();

    save.object_events.extend(set);
    println!("New array length: {0}", &save.object_events.len());
    Ok(())
}

fn save_changes(save_data: ksp::KspSaveData, location: PathBuf) -> Result<u64, Box<dyn Error>> {
    let mut oo = OpenOptions::new();
    oo.read(true).write(true).truncate(false);

    let file = oo.open(&location)?;
    file.set_len(0)?;

    let writer = BufWriter::new(file);
    serde_json::to_writer(writer, &save_data)?;

    let metadata = oo.open(location)?.metadata()?;

    Ok(metadata.len())
}
