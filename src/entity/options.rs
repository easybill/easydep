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
        default_value_t = 10,
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
