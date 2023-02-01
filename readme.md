# Easydep

Easydep is a small, simple tool to automatically pull a release from GitHub (using a GitHub app), run a script inside
the cloned repository, and symlink to the prepared directory to make it available as the latest release.

### Java Installation (Azul Zulu)

```shell
curl -s https://repos.azul.com/azul-repo.key | sudo gpg --dearmor -o /usr/share/keyrings/azul.gpg
echo "deb [signed-by=/usr/share/keyrings/azul.gpg] https://repos.azul.com/zulu/deb stable main" | sudo tee /etc/apt/sources.list.d/zulu.list
sudo apt install zulu17-jre-headless
```

### Env Configuration

#### Required Variables

* `EASYDEP_GITHUB_APP_ID`: The app id to use when accessing the GitHub api. This id is available from the app settings
  on GitHub (Settings -> Developer Settings -> GitHub Apps -> Select App to configure. The app id is visible in the
  About Category)
* `EASYDEP_GITHUB_APP_PRIVATE_KEY`: The app private key used to sign jwt when sending requests to the GitHub api. You
  can create one on the same page as the app id, in the section "Private Keys". **Note:** When setting the key as an
  environment variable, make sure that the key is in one single line **AND** that there is a space after the key header
  delimiter and before the key footer delimiter! It should look
  like `-----BEGIN RSA PRIVATE KEY----- <key data> -----END RSA PRIVATE KEY-----`. The value can also ba the path to the
  key to load.
* `EASYDEP_GITHUB_REPO_ORG`: The organization where the GitHub app is installed and where the repository is located from
  which releases are to be pulled. Note that the setting must be an organization, setting it to a username will not
  work.
* `EASYDEP_GITHUB_REPO_NAME`: The repository to pull the releases from. The repo must be within the scope of the given
  organization.
* `EASYDEP_DEPLOY_BASE_DIRECTORY`: The base directory into which releases should are to be pulled. Each release gets a
  new folder in the given base directory, and the symlink to the current deployment will be located in that directory as
  well.

#### Optional Variables

* `EASYDEP_DEPLOY_LINK_DIRECTORY`: The directory name of the symlink which should be created for the latest pulled
  release. This variable defaults to `current`.
* `EASYDEP_DEPLOY_LABELS`: The labels of the current server the tool runs on. The labels must be given in the
  format: `<key>:<value>`. A key must match the pattern `[a-zA-Z0-9_/-. ]+`. Multiple labels can be delimited
  using `;;`.
  Example: `label_key1:value1;;label_key2:value2`. By default, no labels are configured.
* `EASYDEP_DEPLOY_ADDITIONAL_SYMLINKS`: Additional symlinks that should be created linking from the deployment directory
  to a resolved, external path. The structure of the input is the same as for labels. The key represents the directory
  which should be virtually created and linked to the given resolved target directory (the value). By default, no
  additional symlinks will be created.
* `EASYDEP_RELEASE_PULL_DELAY_MILLIS`: The millis between each request to the GitHub API to get the latest release (in
  milliseconds). This variable defaults to `10000` (10 seconds).
* `EASYDEP_DEPLOY_DISCARDER_MAX`: The maximum amount of old releases to keep. If the given limit is exceeded, the oldest
  releases will be deleted when a new releases is executed. This value can be any number, however a value smaller or
  equal to zero indicates that all releases should be kept, any other value must be bigger than 2 (or will be set to two
  if isn't). The default value of this variable is 10.

### Systemd

```shell
sudo nano /etc/systemd/system/easydep.service
```

#### Service Configuration

```
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
ExecStart=/bin/java -Xms256M -Xmx256M -XX:+UseZGC -XX:+PerfDisableSharedMem -XX:+DisableExplicitGC -jar easydep.jar

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
must contain a bash script that is located at `.deploy/execute.sh`. That script will be executed when a release was
made, and is responsible to prepare the cloned repository before it gets linked as the latest release.

Some pitfalls that might happen:

1. Git commands are no longer available. Before the script gets executed, the `.git` folder gets removed from the
   repository, making it impossible to still use git commands.
2. The script is executed from the root directory of the repository, not from the `.deploy` folder. This means that all
   executed commands are running in the root directory, not from the `.deploy` directory.
3. Additional symlinks (such as log files) are created after the script was executed, and are therefore not present.

### Release Configuration

To set labels for a release to (for example) only get deployed to a few specific servers, the body of the release can
contain the labels which a target server need to fulfil in order to accept the release. An example release body might
be:

```toml
labels = { server = "live1;;live2", "stage?" = "prod" }
```

As shown in the example, a label key (in this case `server`) can have multiple values which are delimited using `;;`.
The target server only needs to fulfil on of the label values (the `server` label can either be set to `live1`
or `live2`). If no label value is set for one of the labels, the deployment will not be executed in case the label is
required. To make a label not required the name of the label needs to be suffixed with `?`. However, the name of the
label will be without the question mark (for example the supplied key is `stage?`, the label name is `stage`). If a
non-required label is given, but a mismatched local value was found, the release gets ignored anyway.

### Compile from source

1. Clone this repository
2. Compile with gradle. On windows run `gradlew.bat build` on unix use `./gradlew build`
3. Take the final file from `build/libs/easydep.jar`

### How does it work?

In general this tool uses 2 threads:

1. A poll thread that checks in a preconfigured interval if there are any new releases.
2. A deployment execute thread that runs the current deployment process.

The deployment handler (that executes the deployments) holds the current deployment process (if any), and if a new
release is published while a deployment is running, will make sure that the current deployment process gets cancelled
before the new process starts.

Each executed step can register a cancel listener which gets executed when the current deployment process gets
cancelled. The cancel listeners are called in LIFO order to ensure that the task order is reversed correctly. The tasks
that are executed during a deployment are registered in a tree structure in order to keep the structure ordered and
simple.

This diagram shows the full execute chain:

```mermaid
flowchart TB
  subgraph Release Processing
  P1["GitHub Release Poll"]-->P2["Release ID compare"]
  P3["Cancel current deployment (if any)"]
  P4["Set current executed deployment"]
  
  P2-- Same ID -->P5["Ignore"]
  P2-- Higher ID -->P3-->P4
  P2-- Lower ID -->P3-->P4-->P6["Check if old release is still locally present"]
  end

  P4-->T1
  P6-- Yes -->T6
  P6-- No -->T1

  subgraph Task Tree
  T1["Release Tag Check"]-->T2["Git Repo Init"]-->T3["Git Tag Checkout"]-->T4["Deployment Directory Git Cleanup"]-->T5["Deploy Script Execute"]-->T6["Deployment Dir Symlink"]-->T7["Old Deployment Cleanup"]    
  end
 
  subgraph Cancel Listeners
  T2-.->C1["Repo Directory Delete"]
  T5-.->C2["Deploy Script Process Kill"]
  end
```
