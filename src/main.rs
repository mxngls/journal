use std::env;
use std::ffi::OsStr;
use std::fs::{File, OpenOptions, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use jiff::{Zoned, fmt::strtime};

const JOURNAL_DIR: &str = "JOURNAL_DIR";
const DEFAULT_JOURNAL_PATH: &str = ".local/share/journal";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let entry = args.next();

    if let Some(ref arg) = entry {
        if arg == "-h" || arg == "--help" {
            println!(
                "Usage: {} [-h|--help] [FILENAME]",
                env::args().next().unwrap_or_else(|| "program".to_string())
            );
            return Ok(());
        }
    }

    // no arguments should remain
    if args.next().is_some() {
        return Err("Too many arguments. Aborting".into());
    }

    // determine journal directory
    let journal_dir = env::var(JOURNAL_DIR).map(PathBuf::from).or_else(|_| {
        env::home_dir()
            .map(|home| home.join(DEFAULT_JOURNAL_PATH))
            .ok_or("Could not determine home directory")
    })?;

    create_dir_all(&journal_dir)?;

    // parse provided entry argument
    let entry_path = match entry {
        Some(filename) => {
            let path = Path::new(&filename);

            let resolved_path = if path.is_absolute() {
                path.starts_with(&journal_dir)
                    .then_some(path.to_path_buf())
                    .ok_or("Entry must be within the journal directory")?
            } else {
                journal_dir.join(&filename)
            };

            resolved_path
                .extension()
                .and_then(OsStr::to_str)
                .filter(|&ext| ext == "txt")
                .ok_or("Entry must be a plain text file")?;

            resolved_path
                .file_stem()
                .and_then(OsStr::to_str)
                .filter(|&name| strtime::parse("%Y-%m-%d", name).is_ok())
                .ok_or("Entry filename must conform to the followin YYYY-MM-DD")?;

            resolved_path
        }
        None => {
            let entry_name = Zoned::now().date().to_string() + ".txt";
            journal_dir.join(&entry_name)
        }
    };

    // headers __always__ point to the current date time
    let entry_header = Zoned::now()
        .strftime("# %a %b %d %H:%M:%S %Z %Y")
        .to_string();

    // append to existing entry or create a new one
    if entry_path.exists() {
        let mut entry_file = OpenOptions::new().append(true).open(&entry_path)?;
        writeln!(entry_file, "\n{}\n\n", entry_header)?;
    } else {
        let mut entry_file = File::create(&entry_path)?;
        writeln!(entry_file, "{}\n\n", entry_header)?;
    }

    let editor = env::var("EDITOR")?;

    let status = Command::new(&editor)
        .args(match editor.as_str() {
            "vim" | "nvim" => vec!["-c", "normal Gzz"],
            _ => vec![],
        })
        .arg(&entry_path)
        .status()
        .map_err(|e| format!("Failed to execute {}: {}", &editor, e))?;

    if !status.success() {
        return Err(format!("{} exited with error code: {:?}", &editor, status.code()).into());
    };

    Ok(())
}
