# Easydep

![License](https://img.shields.io/github/license/easybill/easydep)
![Build status](https://img.shields.io/github/actions/workflow/status/easybill/easydep/ci.yml)
![Release version](https://img.shields.io/github/v/release/easybill/easydep)

Easydep is a small, HTTP driven & simple tool to automatically clone a repository from GitHub(using a GitHub app),
checking out a specific tag, execute a script located inside repository, and symlink to the prepared directory to make
it available as the latest release. It was also built to allow parallel deployment to multiple servers without one
server finishing with one version of the deployment way before the others.

### Env Configuration

#### Required Variables

* `EASYDEP_GITHUB_APP_ID`: The app id to use when accessing the GitHub api. This id is available from the app settings
  on GitHub (Settings -> Developer Settings -> GitHub Apps -> Select App to configure. The app id is visible in the
  About Category)
* `EASYDEP_GITHUB_APP_PRIVATE_KEY`: The app private key used to sign jwt when sending requests to the GitHub api. The
  given variable must be a path to the key file in PEM format.
* `EASYDEP_GITHUB_REPO_ORG`: The organization where the GitHub app is installed and where the repository is located from
  which releases are to be pulled. Note that the setting must be an organization, setting it to a username will not
  work.
* `EASYDEP_GITHUB_REPO_NAME`: The repository to pull the releases from. The repo must be within the scope of the given
  organization.
* `EASYDEP_DEPLOY_BASE_DIRECTORY`: The base directory into which releases should are to be pulled. Each release gets a
  new folder in the given base directory, and the symlink to the current deployment will be located in that directory as
  well.
* `EASYDEP_BIND_HOST`: The host and port (in the format `ip:port` or `[ipv6]:port`) to bind the HTTP server that is to
  handle the incoming requests.
* `EASYDEP_REQUEST_AUTH_TOKEN`: The authentication header value (Bearer token) that each http request is required to
  have set in order to make an HTTP request. **Note**: Empty values do not mean that no token is required.

#### Optional Variables

* `EASYDEP_DEPLOY_LINK_DIRECTORY`: The directory name of the symlink which should be created for the latest pulled
  release. This variable defaults to `current`.
* `EASYDEP_DEPLOY_ADDITIONAL_SYMLINKS`: Additional symlinks that should be created linking from the deployment directory
  to a resolved, external path. The structure of the input is the same as for labels. The key represents the directory
  which should be virtually created and linked to the given resolved target directory (the value). By default, no
  additional symlinks will be created.
* `EASYDEP_DEPLOY_PUBLISH_DELAY`: The delay (in seconds) that should be applied between receiving the publish HTTP
  request and the actual execution of the publication step. This can for example be used when synchronizing between
  multiple steps is required. This variable defaults to `15`.
* `EASYDEP_RELEASE_CACHE_TIME`: The time (in minutes) a publish request should be cached locally on the server. Within
  the given timespan the request to prepare and cancel/publish the deployment must be made. This variable defaults
  to `15`.
* `EASYDEP_MAX_STORED_RELEASES`: The maximum amount of old releases to keep. If the given limit is exceeded, the oldest
  releases will be deleted when a new releases is executed. The given value can be any positive number than is larger
  than 2. This variable defaults to `10`.
* `EASYDEP_LOG_DEBUG`: Set this variable to `true` to enable debug logging. This variable defaults to `false`.
* `EASYDEP_REVISION_FILE`: Sets the name of the file to write the current git revision to. If set to an empty string the
  current revision is not written to any file. This variable defaults to `REVISION`.

### Systemd

```shell
sudo mkdir -p /usr/lib/easydep
sudo touch /etc/default/easydep
sudo nano /etc/systemd/system/easydep.service
```

#### Service Configuration

```service
[Unit]
Description=Easydep
After=network-online.target
Wants=network-online.target

[Service]
Type=simple

User=www-data
Group=www-data

Restart=always
RestartSec=10

WorkingDirectory=/usr/lib/easydep
EnvironmentFile=/etc/default/easydep
ExecStart=/usr/bin/easydep

[Install]
WantedBy=multi-user.target
```

#### Start & Enable

```shell
sudo systemctl daemon-reload
sudo systemctl start easydep.service
sudo systemctl enable easydep.service
```

### Preparing the target repository

The repository that should be pulled and deployed by this tool needs to be "prepared" as well. The target repository
must contain a bash script that is located at `.easydep/execute.sh`. That script will be executed when a release was
made, and is responsible to prepare the cloned repository before it gets linked as the latest release.

Some pitfalls that might happen:

1. Git commands are no longer available. Before the script gets executed, the `.git` folder gets removed from the
   repository, making it impossible to still use git commands.
2. The script is executed from the root directory of the repository, not from the `.deploy` folder. This means that all
   executed commands are running in the root directory, not from the `.deploy` directory.
3. Additional symlinks (such as log files) are created after the script was executed, and are therefore not present.

### Script execution order

All scripts must be located in the `.easydep` directory. There are two steps that can have scripts, and 2 results that
can occur from these steps. Based on the step result, the according script is executed.

The main script is the `execute.sh` script. The script is always executed after the git repository was prepared and some
general cleanup steps were taken (all within the `init` task). The script is responsible for preparing the pulled
repository for deployment.

The following step results exist:

* `success`: The step was executed successfully.
* `failure`: There was some kind of failure when executing the step.

The following steps exist:

* `init`: The first step. During this step the repository gets checked out and prepared and the `execute.sh`
  gets executed.
* `publish`: The second step that is responsible for publishing a release.

The script naming for lifecycle scripts is always the same: `[step name]_[step result].sh` (all lowercase).
**Note**: there is no 100% guarantee that the failure script for the init task is executed. When the task fails early
(for example while fetching the git repository) there is no way to execute the script. Therefore, scripts should not be
used to handle errors, the client that is making the request should handle failures and gets the full logging output.

### Compile from source

1. Clone this repository
2. If you're on Linux you might need to install `build-essentials`
3. Make sure you have [Cargo installed](https://doc.rust-lang.org/cargo/getting-started/installation.html) and run `cargo build --release`
4. Take the final file from `target/release/easydep[.extension]`

### Download pre-compiled binary

The binaries for easydep are pre-compiled available attached to each
[release](https://github.com/easybill/easydep/releases). These are currently pre-compiled for
the following targets:

| Target | Architectures     |
|--------|-------------------|
| Apple  | x64, x86, aarch64 |
| Linux  | x64, x86, aarch64 |

### How does it work?

There are 3 HTTP handlers which can be called to deploy a release to a server:

* `/deploy/start`: Starts the deployment process. This will run the init task. The required query parameters for this
  route are:
    * `release_id`: A unique, positive id for the release, can be for example the release id from GitHub. This id must be
      incremental in order to make the release discarder work properly.
    * `tag_name`: The name of the tag (can also be a branch or commit) that should be processed for the deployment.
* `/deploy/publish`: Publishes the deployment that was previously prepared and links it as the current one. The base time
  supplied via the query parameter gives the time that the request was sent out initially. The server will add the
  configured publish delay seconds and execute the publication task at the given time. The required query parameters for
  this route are:
    * `release_id`: The unique id of the release to publish. This id must be same as supplied to the start route.
    * `base_time`: The unix epoch base time seconds when the release publication was requested.
* `/deploy/cancel`: Cancels a deployment that wasn't published yet. The handler for the route will wait for the init
  handler to complete before starting the cancellation task. When the client gets a successful response, the deployment
  is completely cancelled and all associated data is removed from the server. The required query parameter for this
  route is:
    * `release_id`: The unique id of the release to cancel. This id must be same as supplied to the start route.

All routes can respond with the following status codes:

| Status | Meaning                                                                                                                                                                                           |
|--------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 200    | The requested action was completed successfully. The body of the response can contain additional informative data (for example command outputs)                                                   |
| 400    | The client sent invalid data and the handler is not able to process the request with/for that data (at the current time). The body of the responds contains detailed information about the cause. |
| 401    | The client sent an invalid authorization token.                                                                                                                                                   |
| 500    | There was an internal error processing the request. The body of the response contains additional information (for example a short error message or log outputs).                                  |

It is up to the calling client to decide if a release gets cancelled or publish and when the release is published. Just
note that cancelling or publishing a deployment is no longer possible after it expired in the cache (see
the `EASYDEP_RELEASE_CACHE_TIME` config option for more information).
