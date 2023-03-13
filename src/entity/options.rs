use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, Clone)]
pub(crate) struct Symlink {
    pub link_name: String,
    pub target: PathBuf,
}

#[derive(StructOpt, Debug, Clone)]
pub(crate) struct Options {
    #[structopt(
        long = "token",
        env = "EASYDEP_REQUEST_AUTH_TOKEN",
        hide_env_values = true
    )]
    pub auth_token: String,
    #[structopt(long = "app-id", env = "EASYDEP_GITHUB_APP_ID")]
    pub github_app_id: String,
    #[structopt(long = "app-key-path", env = "EASYDEP_GITHUB_APP_PRIVATE_KEY")]
    pub github_app_key_path: String,
    #[structopt(long = "repo-org", env = "EASYDEP_GITHUB_REPO_ORG")]
    pub github_repo_org: String,
    #[structopt(long = "repo-name", env = "EASYDEP_GITHUB_REPO_NAME")]
    pub github_repo_name: String,
    #[structopt(long = "bind", env = "EASYDEP_BIND_HOST")]
    pub bind_host: String,
    #[structopt(long = "basedir", env = "EASYDEP_DEPLOY_BASE_DIRECTORY")]
    pub base_directory: String,
    #[structopt(
        long = "currentdir",
        env = "EASYDEP_DEPLOY_LINK_DIRECTORY",
        default_value = "current"
    )]
    pub deploy_link_dir: String,
    #[structopt(
        long = "publish-delay",
        env = "EASYDEP_DEPLOY_PUBLISH_DELAY",
        default_value = "15"
    )]
    pub deploy_publish_delay: i64,
    #[structopt(
        long = "cache-time",
        env = "EASYDEP_RELEASE_CACHE_TIME",
        default_value = "15"
    )]
    pub release_cache_seconds: u64,
    // parsed internally, not exposed
    #[structopt(
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
            .map(|part| part.split_once(":"))
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
}
