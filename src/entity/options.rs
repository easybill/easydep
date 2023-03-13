use structopt::StructOpt;

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
}
