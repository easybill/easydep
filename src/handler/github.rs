use crate::entity::options::Options;
use jsonwebtoken::EncodingKey;
use octocrab::models::repos::Release;
use octocrab::models::Installation;
use octocrab::Octocrab;
use secrecy::SecretString;
use tokio::fs;

pub(crate) async fn read_installation_token(
    options: &Options,
) -> anyhow::Result<SecretString, anyhow::Error> {
    // read the installation
    let octocrab = authenticated_github_client(options).await?;
    let installation = find_installation(&octocrab, options).await?;

    // get the access token for the installation (this token can later be used to f. ex. pull
    // code from github). See https://github.com/orgs/community/discussions/24575#discussioncomment-3244524
    let (_, token) = octocrab.installation_and_token(installation.id).await?;
    Ok(token)
}

pub(crate) async fn read_latest_release(
    options: &Options,
) -> anyhow::Result<Option<Release>, anyhow::Error> {
    // build an octocrab instance and authenticate it as the target installation
    let octocrab = authenticated_github_client(options).await?;
    let installation = find_installation(&octocrab, options).await?;
    let app_scoped_octocrab = octocrab.installation(installation.id);

    // list the last 100 releases and find the latest for the current env
    let repo_handler =
        app_scoped_octocrab.repos(&options.github_repo_org, &options.github_repo_name);
    let last_releases = repo_handler
        .releases()
        .list()
        .per_page(100)
        .send()
        .await?
        .items;
    let mut possible_releases: Vec<Release> = last_releases
        .into_iter()
        .filter(|release| !release.draft)
        .filter(|release| {
            // pre-release + prod -> false
            // pre-release + staging -> true
            // release + prod -> true
            // release + staging -> false
            let prod = options.prod_environment();
            release.prerelease != prod
        })
        .collect();

    // sort the releases by id, descending
    possible_releases.sort_by(|left, right| right.id.cmp(&left.id));
    Ok(possible_releases.first().cloned())
}

async fn authenticated_github_client(options: &Options) -> anyhow::Result<Octocrab, anyhow::Error> {
    let app_id = options.github_app_id.parse::<u64>()?.into();

    // read the private key
    let file_content = fs::read(&options.github_app_key_path).await?;
    let private_key = EncodingKey::from_rsa_pem(file_content.as_slice())?;

    // build the octocrab instance
    let octocrab = Octocrab::builder().app(app_id, private_key).build()?;
    Ok(octocrab)
}

async fn find_installation(
    octocrab: &Octocrab,
    options: &Options,
) -> anyhow::Result<Installation, anyhow::Error> {
    octocrab
        .apps()
        .get_repository_installation(&options.github_repo_org, &options.github_repo_name)
        .await
        .map_err(Into::into)
}
