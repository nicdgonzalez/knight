use std::path::Path;

use tracing::error;

pub trait Persistent: Sized {
    fn from_file(file: &Path) -> anyhow::Result<Self>;
    fn save(&self, file: &Path) -> anyhow::Result<()>;
}

pub fn load_or_default<T>(path: &Path) -> T
where
    T: Persistent + Default,
{
    T::from_file(path).unwrap_or_else(|_| {
        let data = T::default();
        if let Err(err) = data.save(path) {
            error!("failed to save default data: {:#}: {err}", path.display());
        }
        data
    })
}
