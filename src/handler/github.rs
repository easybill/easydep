use crate::entity::options::Options;
use jsonwebtoken::EncodingKey;
use octocrab::Octocrab;
use secrecy::SecretString;
use tokio::fs;

pub(crate) async fn read_installation_token(
    options: &Options,
) -> anyhow::Result<SecretString, anyhow::Error> {
    let app_id = options.github_app_id.parse::<u64>()?.into();

    // read the private key
    let file_content = fs::read(&options.github_app_key_path).await?;
    let private_key = EncodingKey::from_rsa_pem(file_content.as_slice())?;

    // build the octocrab instance and retrieve the installation
    let octocrab = Octocrab::builder().app(app_id, private_key).build()?;
    let installation = octocrab
        .apps()
        .get_repository_installation(&options.github_repo_org, &options.github_repo_name)
        .await?;

    // get the access token for the installation (this token can later be used to f. ex. pull
    // code from github). See https://github.com/orgs/community/discussions/24575#discussioncomment-3244524
    let (_, token) = octocrab.installation_and_token(installation.id).await?;
    Ok(token)
}
