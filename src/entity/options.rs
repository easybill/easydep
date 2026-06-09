use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone)]
pub(crate) struct Symlink {
    pub link_name: String,
    pub target: PathBuf,
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct Options {
    #[arg(long = "debug", env = "EASYDEP_LOG_DEBUG", default_value_t = false)]
    pub debug: bool,
    #[arg(
        long = "token",
        env = "EASYDEP_REQUEST_AUTH_TOKEN",
        hide_env_values = true
    )]
    pub auth_token: String,
    #[arg(long = "app-id", env = "EASYDEP_GITHUB_APP_ID")]
    pub github_app_id: String,
    #[arg(long = "app-key-path", env = "EASYDEP_GITHUB_APP_PRIVATE_KEY")]
    pub github_app_key_path: String,
    #[arg(long = "repo-org", env = "EASYDEP_GITHUB_REPO_ORG")]
    pub github_repo_org: String,
    #[arg(long = "repo-name", env = "EASYDEP_GITHUB_REPO_NAME")]
    pub github_repo_name: String,
    #[arg(long = "bind", env = "EASYDEP_BIND_HOST")]
    pub bind_host: String,
    #[arg(long = "basedir", env = "EASYDEP_DEPLOY_BASE_DIRECTORY")]
    pub base_directory: String,
    #[arg(
        long = "currentdir",
        env = "EASYDEP_DEPLOY_LINK_DIRECTORY",
        default_value = "current"
    )]
    pub deploy_link_dir: String,
    #[arg(
        long = "publish-delay",
        env = "EASYDEP_DEPLOY_PUBLISH_DELAY",
        default_value_t = 15,
        value_parser = clap::value_parser!(i64).range(0..)
    )]
    pub deploy_publish_delay_seconds: i64,
    #[arg(
        long = "cache-time",
        env = "EASYDEP_RELEASE_CACHE_TIME",
        default_value_t = 15
    )]
    pub release_cache_minutes: u64,
    #[arg(
        long = "max-stored-releases",
        env = "EASYDEP_MAX_STORED_RELEASES",
        default_value_t = 5,
        value_parser = clap::value_parser!(u64).range(3..)
    )]
    pub max_releases_to_store: u64,
    #[arg(
        long = "revision-file",
        env = "EASYDEP_REVISION_FILE",
        default_value = "REVISION"
    )]
    pub git_revision_file: String,
    #[arg(long = "environment", env = "EASYDEP_ENV", default_value = "")]
    pub environment: String,
    // parsed internally, not exposed
    #[arg(
        long = "symlinks",
        env = "EASYDEP_DEPLOY_ADDITIONAL_SYMLINKS",
        default_value = ""
    )]
    additional_symlinks: String,
}

impl Options {
    pub fn parse_additional_symlinks(&self) -> Vec<Symlink> {
        self.additional_symlinks
            .split(";;")
            .map(|part| part.split_once(':'))
            .filter(|split| split.is_some())
            .map(|split| {
                let (link_name, target) = split.unwrap();
                Symlink {
                    link_name: link_name.to_string(),
                    target: PathBuf::from(target.to_string()),
                }
            })
            .collect()
    }

    pub fn prod_environment(&self) -> bool {
        self.environment.is_empty() || self.environment == "prod"
    }

    pub fn environment_suffix(&self) -> String {
        if self.environment.is_empty() || self.environment == "prod" {
            String::from("")
        } else {
            format!("-{}", self.environment.clone())
        }
    }
}

#[cfg(test)]
pub(crate) fn make_test_options(base_directory: &str, environment: &str) -> Options {
    Options {
        debug: false,
        auth_token: String::new(),
        github_app_id: String::new(),
        github_app_key_path: String::new(),
        github_repo_org: String::new(),
        github_repo_name: String::new(),
        bind_host: String::new(),
        base_directory: base_directory.to_string(),
        deploy_link_dir: String::from("current"),
        deploy_publish_delay_seconds: 15,
        release_cache_minutes: 15,
        max_releases_to_store: 5,
        git_revision_file: String::from("REVISION"),
        environment: environment.to_string(),
        additional_symlinks: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_options(environment: &str, additional_symlinks: &str) -> Options {
        let mut options = make_test_options("", environment);
        options.additional_symlinks = additional_symlinks.to_string();
        options
    }

    #[test]
    fn parse_additional_symlinks_empty_returns_empty_vec() {
        let options = make_options("", "");
        assert!(options.parse_additional_symlinks().is_empty());
    }

    #[test]
    fn parse_additional_symlinks_single_entry() {
        let options = make_options("", "log:/var/log/myapp");
        let result = options.parse_additional_symlinks();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].link_name, "log");
        assert_eq!(result[0].target, PathBuf::from("/var/log/myapp"));
    }

    #[test]
    fn parse_additional_symlinks_multiple_entries() {
        let options = make_options("", "log:/var/log/myapp;;cache:/var/cache/myapp");
        let result = options.parse_additional_symlinks();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].link_name, "log");
        assert_eq!(result[0].target, PathBuf::from("/var/log/myapp"));
        assert_eq!(result[1].link_name, "cache");
        assert_eq!(result[1].target, PathBuf::from("/var/cache/myapp"));
    }

    #[test]
    fn parse_additional_symlinks_filters_malformed_entries() {
        let options = make_options("", "log:/var/log;;malformed;;cache:/var/cache");
        let result = options.parse_additional_symlinks();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].link_name, "log");
        assert_eq!(result[1].link_name, "cache");
    }

    #[test]
    fn parse_additional_symlinks_trailing_separator_does_not_crash() {
        let options = make_options("", "log:/var/log;;");
        let result = options.parse_additional_symlinks();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].link_name, "log");
    }

    #[test]
    fn prod_environment_classifies_all_inputs() {
        assert!(make_options("", "").prod_environment(), "empty == prod");
        assert!(make_options("prod", "").prod_environment(), "explicit prod");
        assert!(
            !make_options("staging", "").prod_environment(),
            "other != prod"
        );
    }

    #[test]
    fn environment_suffix_per_input() {
        assert_eq!(make_options("", "").environment_suffix(), "");
        assert_eq!(make_options("prod", "").environment_suffix(), "");
        assert_eq!(make_options("staging", "").environment_suffix(), "-staging");
    }
}
