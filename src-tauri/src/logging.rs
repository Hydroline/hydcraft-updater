use std::{
    fs::{create_dir_all, OpenOptions},
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn append(game_dir: &Path, level: &str, message: impl AsRef<str>) {
    let path = game_dir.join(".hydcraft").join("logs").join("updater.log");
    let line = format!(
        "{} [{}] {}\n",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_secs())
            .unwrap_or_default(),
        level,
        message.as_ref().replace(['\r', '\n'], " ")
    );

    if let Some(parent) = path.parent() {
        if let Err(error) = create_dir_all(parent) {
            eprintln!("[HydCraft Updater] unable to create log directory: {error}");
            return;
        }
    }
    match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(mut file) => {
            if let Err(error) = file.write_all(line.as_bytes()) {
                eprintln!("[HydCraft Updater] unable to write log: {error}");
            }
        }
        Err(error) => eprintln!("[HydCraft Updater] unable to open log: {error}"),
    }
}
