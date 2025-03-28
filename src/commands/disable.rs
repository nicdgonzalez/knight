use crate::commands;

pub fn run() -> Result<(), commands::Error> {
    let lock_file = crate::get_lock_file();
    std::fs::create_dir_all(lock_file.parent().unwrap())?;
    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&lock_file)?;

    eprintln!("Knight is now disabled.");
    Ok(())
}
