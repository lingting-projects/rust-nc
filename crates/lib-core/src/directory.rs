use anyhow::Result;
use arc_swap::ArcSwapOption;
use framework_core::ApplicationDirectory;
use std::sync::{Arc, LazyLock};

static DIRECTORY: LazyLock<ArcSwapOption<ApplicationDirectory>> =
    LazyLock::new(ArcSwapOption::empty);

pub fn directorys() -> Result<Arc<ApplicationDirectory>> {
    if let Some(v) = DIRECTORY.load_full() {
        return Ok(v.clone());
    }

    let directory = ApplicationDirectory::root("live.lingting.nc.rust")?;
    let v = Arc::new(directory);
    DIRECTORY.store(Some(v.clone()));
    Ok(v)
}
