use crate::commands;

pub fn run() -> Result<(), commands::Error> {
    let lock_file = crate::get_disabled_file();
    std::fs::create_dir_all(lock_file.parent().unwrap())?;

    if let Err(err) = std::fs::remove_file(&lock_file) {
        if err.kind() == std::io::ErrorKind::NotFound {
            eprintln!("Knight was already enabled.");
            return Ok(());
        }

        return Err(err.into());
    }

    eprintln!("Knight is now enabled!");
    Ok(())
}
