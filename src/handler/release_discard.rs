use crate::entity::options::Options;
use log::info;
use std::fs::{read_dir, remove_dir_all};
use std::path::{Path, PathBuf};

pub(crate) fn discard_oldest_release(options: &Options) -> anyhow::Result<(), anyhow::Error> {
    let max_stored_releases = options.max_releases_to_store as usize;
    let base_directory = Path::new(&options.base_directory).join("releases");

    // get all directory paths in the directory
    let mut release_directories: Vec<(PathBuf, u64)> = read_dir(base_directory)?
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
                .and_then(|dir_name| dir_name.parse::<u64>().ok())
                .map(|id| (path, id))
        })
        .collect();

    // check if there is a need to discard some entries
    let stored_releases = release_directories.len();
    if max_stored_releases >= stored_releases {
        info!(
            "No need to remove any release (stored releases: {}, number of releases to keep: {})",
            stored_releases, max_stored_releases
        );
        return Ok(());
    }

    // sort the parsed release directories, ascending
    // then remove the oldest release (only remove one release per call)
    release_directories.sort_by(|left, right| left.1.cmp(&right.1));
    if let Some(release_to_remove) = release_directories.first() {
        let (release_directory, release_id) = release_to_remove;
        if release_directory.exists() {
            info!("Removing oldest stored release {}", release_id);
            remove_dir_all(release_directory)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::options::make_test_options;
    use std::fs::{create_dir, create_dir_all, write};

    fn options_for(base_dir: &Path, max_releases: u64) -> Options {
        let mut options = make_test_options(base_dir.to_str().unwrap(), "");
        options.max_releases_to_store = max_releases;
        options
    }

    fn make_release_dir(base: &Path, id: &str) {
        create_dir(base.join("releases").join(id)).unwrap();
    }

    #[test]
    fn does_nothing_when_count_at_or_below_max() {
        let tmp = tempfile::tempdir().unwrap();
        let releases = tmp.path().join("releases");
        create_dir_all(&releases).unwrap();
        make_release_dir(tmp.path(), "100");
        make_release_dir(tmp.path(), "200");
        make_release_dir(tmp.path(), "300");

        let options = options_for(tmp.path(), 3);
        discard_oldest_release(&options).unwrap();

        assert!(releases.join("100").exists());
        assert!(releases.join("200").exists());
        assert!(releases.join("300").exists());
    }

    #[test]
    fn removes_oldest_by_numeric_id_not_lexicographic() {
        let tmp = tempfile::tempdir().unwrap();
        let releases = tmp.path().join("releases");
        create_dir_all(&releases).unwrap();
        for id in ["50", "100", "150", "200", "300"] {
            make_release_dir(tmp.path(), id);
        }

        let options = options_for(tmp.path(), 3);
        discard_oldest_release(&options).unwrap();

        assert!(!releases.join("50").exists(), "50 should have been removed");
        for id in ["100", "150", "200", "300"] {
            assert!(releases.join(id).exists(), "{} should still exist", id);
        }
    }

    #[test]
    fn ignores_non_numeric_directory_names() {
        let tmp = tempfile::tempdir().unwrap();
        let releases = tmp.path().join("releases");
        create_dir_all(&releases).unwrap();
        make_release_dir(tmp.path(), "100");
        make_release_dir(tmp.path(), "200");
        make_release_dir(tmp.path(), "abc");

        let options = options_for(tmp.path(), 1);
        discard_oldest_release(&options).unwrap();

        assert!(
            !releases.join("100").exists(),
            "oldest numeric dir should be removed"
        );
        assert!(releases.join("200").exists());
        assert!(
            releases.join("abc").exists(),
            "non-numeric dir is never a deletion candidate"
        );
    }

    #[test]
    fn non_numeric_dirs_do_not_count_towards_limit() {
        let tmp = tempfile::tempdir().unwrap();
        let releases = tmp.path().join("releases");
        create_dir_all(&releases).unwrap();
        make_release_dir(tmp.path(), "100");
        make_release_dir(tmp.path(), "abc");
        make_release_dir(tmp.path(), "def");

        let options = options_for(tmp.path(), 1);
        discard_oldest_release(&options).unwrap();

        assert!(
            releases.join("100").exists(),
            "only one numeric dir exists, so the limit is not exceeded and nothing is removed"
        );
        assert!(releases.join("abc").exists());
        assert!(releases.join("def").exists());
    }

    #[test]
    fn ignores_plain_files_in_releases_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let releases = tmp.path().join("releases");
        create_dir_all(&releases).unwrap();
        make_release_dir(tmp.path(), "100");
        make_release_dir(tmp.path(), "200");
        write(releases.join("50"), "not a directory").unwrap();

        let options = options_for(tmp.path(), 1);
        discard_oldest_release(&options).unwrap();

        assert!(!releases.join("100").exists());
        assert!(releases.join("200").exists());
        assert!(releases.join("50").exists());
    }

    #[test]
    fn missing_releases_directory_returns_err() {
        let tmp = tempfile::tempdir().unwrap();
        let options = options_for(tmp.path(), 3);
        let result = discard_oldest_release(&options);
        assert!(result.is_err());
    }
}
