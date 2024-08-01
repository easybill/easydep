# Easydep

![License](https://img.shields.io/github/license/easybill/easydep)
![Build status](https://img.shields.io/github/actions/workflow/status/easybill/easydep/ci.yml)
![Release version](https://img.shields.io/github/v/release/easybill/easydep)

Easydep is a small, HTTP driven & simple tool to automatically clone a repository from GitHub (using a GitHub app),
checking out a specific tag, execute a script located inside repository, and symlink to the prepared directory to make
it available as the latest release. A client CLI tool is exposed to trigger actions on the servers or display their
status. The general design goal was to make most issues during the process a client problem, rather than trying to
somehow handle them on the server.

### Server

The server uses a TOML configuration file which contains all the settings, including the available release profiles.
The path to the configuration file can be set using the flag `--config-path` or using the environment variable
`EASYDEP_CONFIG_PATH`.

#### Script execution order

The easydep server uses scripts that are called based on the lifecycle of a deployment. These scripts are used to,
for example, initialize a deployment. All scripts must be located under the `.easydep/<profile_id>/<lifecycle>.sh` path.

There are 3 lifecycles:

* `init` - The initialize lifecycle. Called when initially starting a deployment after the git repo has been checked out
  and other init things were done (like creating the additional symlink, revision file, ...).
* `publish` - The publish lifecycle. Called when the symlink of the current release directory was switched to the
  deployment directory but before the oldest release is discarded.
* `delete` - The delete lifecycle. Called before the directory of the release that should be removed is deleted.

#### Example configuration

```toml
# Sets the bind host for the gRPC endpoint. The client CLI uses this endpoint to trigger actions on the server.
bind_host = "127.0.0.1:6666"
# The absolute path to the base folder where the server should store all deployment related files in.
base_directory = "/var/deploy"
# The GitHub app id that should be used for git related operations. Needs to have access to the content of a repository
# (read-only) to access the files & get information about releases
github_app_id = 12345678
# The path to the GitHub app private key.
github_app_pem_key_path = "/var/secret/gh_app.pem"
# The amount of releases that should be retained on the server. If more releases are stored than this count the oldest
# release will be deleted when publishing a new deployment
retained_releases = 10

[[deployment_configs]]
# The id of the deployment configuration (must be unique). The id is used when the client triggers a deployment to
# identify which deployment configuration should be used for the action.
id = "test"
# The target (or name) of the deployment configuration. This setting is used in directory names if multiple configuration
# exist for different purposes, but they are actually all targeting the same environment.
# Releases will for example be stored in <base>/releases/<target>/<release_id> rather than 
# <base>/releases/<profile_id>/<release_id>.
target = "staging"
# Indicates if this deployment configuration can only be extended and not used directly for executing a deployment.
# See `extended_script_configurations` on how configurations extend each other.
extend_only = false
# The owner of the source repo that is managed by this deployment profile.
source_repo_owner = "easybill"
# The name of the source repo that is manged by rhis deployment profile. Releases and tags are pulled from here.
source_repo_name = "easydep"
# The names of the repo branches that are allowed to use this release profile. This check is performed by using the
# target commitish provided by the GitHub api, so releases must be created from a branch rahter than a specific commit.
allowed_repo_branches = ["dev"]
# The names of the repo branches that are not allowed to use this release profile. Denied branches are checked before
# the allowed brances.  This check is performed by using the target commitish provided by the GitHub api, so releases 
# must be created from a branch rahter than a specific commit.
denied_repo_branches = ["main"]
# A file that will automatically be created when checking out a release in the deployment directory, containing the
# full commit SHA of the checked-out tag. Optional: if ommited no revision file is created.
revision_file_name = "REV"
# The ids of deploy profiles whose scripts should be called before the scripts of this deployment profile. This could
# for example be used to share init logic between two deployment profiles.
extended_script_configurations = []
# The symlinks that should be created relative from the deployment directory to some other directory.
# The `source` is the relative directory inside the deployment directory, which gets linked to the provided `target`.
# This setting allows to create links between files and directories, the link type is choosen based on the targer type.
# So links are created like: `<deployment-directory>/<source>` -> `<target>`
symlinks = [
  { source = "log", target = "/opt/log" }
]
```

### Client

The client uses a TOML configuration file which contains all the target servers which can execute deployments. The path
to the configuration file can be set using the flag `-c` or `--config-path` or using the environment variable
`EASYDEP_CONFIG_PATH`.

#### CLI

The client CLI is used to get status information and trigger actions on each server. The server handles errors that
are encountered gracefully, but makes no attempt to recover from them. For example: when init script fails the client
will be notified that the step failed, but could still publish the deployment anyway. The server only executes one
action at a time, so if some action is running the server will not accept any request to start another action.

#### CLI commands

Note: arguments in `<>` are required, arguments in `[]` are optional. Server ids starting with `t:` will be treated as
tags and match all servers that have the tag (`t:test` is the tag `test`, the prefix is stripped).

* Local client config:
  * `config list` - Lists all servers that are configured in the local client configuration.
  * `config add <server id> <server host> [tags...]` - Adds a new server to the local client configuration.
  * `config remove <server id>` - Removes a server from the local client configuration.
* Server status info:
  * `status [server id...]` - Requests status information from the provided server(s).
* Deployment Actions:
  * `deploy start <profile> <release id> [server id...]` - Start a deployment process for the given release (identified
    by the GitHub release id) using the given profile on the provided server(s).
  * `deploy publish <release id> [server id...]` - Publishes a previously started deployment on the given server(s).
  * `deploy delete <release id> [server id...]` - Deletes the release that was previously started. This action cannot be
    done if the release was already published. Use `rollback` in that case instead.
  * `deploy rollback <profile> [server id...]` - Rolls back to the previous deployment of a profile on the given server(
    s). This action
    unrelated to the `start/publish/delete` actions.
  * `deploy status <profile> [server id...]` - Prints the current deployment status for the given profile on the given
    server(s).

#### Example configuration

```toml
[[servers]]
# The id of the target server which can be used in cli commands (must be unique).
id = "target1"
# The address where the server is running. Must be a valid URI containing a scheme and host.
# Each host can only be used once per configuration.
address = "http://127.0.0.1:6666"
# The tags of the server configuration. Can be none, one or multiple which can also be used as "server ids" in cli 
# commands by using the `t:` prefix. So using `t:test` would map to a tag called `test` rather than a server id.
tags = ["test"]
```
