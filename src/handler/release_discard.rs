use crate::entity::options::Options;
use std::fs::{read_dir, remove_dir_all};
use std::path::{Path, PathBuf};

pub(crate) fn discord_oldest_release(options: &Options) -> anyhow::Result<(), anyhow::Error> {
    let max_stored_releases = options.max_releases_to_store as usize;
    let base_directory = Path::new(&options.base_directory).join("releases");

    // get all directory paths in the directory
    let mut release_directories: Vec<(PathBuf, i32)> = read_dir(base_directory)?
        .filter_map(|res| res.ok())
        .filter(|entry| {
            entry
                .file_type()
                .map(|file_type| file_type.is_dir())
                .unwrap_or(false)
        })
        .map(|res| res.path())
        .filter_map(|path| {
            path.file_name()
                .and_then(|dir_name| dir_name.to_str().map(|name| name.to_string()))
                .and_then(|dir_name| dir_name.parse::<i32>().ok())
                .map(|id| (path, id))
        })
        .collect();

    // check if there is a need to discord some entries
    // we subtract one entry here as it is the base git repository which does not count
    let stored_releases = release_directories.len().checked_sub(1).unwrap_or(0);
    if max_stored_releases >= stored_releases {
        return Ok(());
    }

    // sort the parsed release directories, ascending
    // then remove the oldest release (only remove one release per call)
    release_directories.sort_by(|left, right| left.1.cmp(&right.1));
    if let Some(release_to_remove) = release_directories.first() {
        let (release_directory, _) = release_to_remove;
        if release_directory.exists() {
            remove_dir_all(release_directory)?;
        }
    }

    Ok(())
}
