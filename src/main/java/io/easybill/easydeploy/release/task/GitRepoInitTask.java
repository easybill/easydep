package io.easybill.easydeploy.release.task;

import com.google.inject.Inject;
import io.easybill.easydeploy.github.GitHubAccessProvider;
import io.easybill.easydeploy.release.ReleaseDirectoryHandler;
import io.easybill.easydeploy.task.ChainedTask;
import io.easybill.easydeploy.task.TaskExecutionContext;
import java.nio.file.Files;
import org.apache.commons.io.file.PathUtils;
import org.apache.commons.io.file.StandardDeleteOption;
import org.apache.commons.lang3.tuple.Pair;
import org.eclipse.jgit.api.Git;
import org.eclipse.jgit.api.RemoteSetUrlCommand;
import org.eclipse.jgit.lib.Constants;
import org.eclipse.jgit.transport.URIish;
import org.jetbrains.annotations.NotNull;
import org.kohsuke.github.GHRelease;

public final class GitRepoInitTask extends ChainedTask<GHRelease> {


  private static final String GH_APP_FETCH_URL_FORMAT = "https://x-access-token:%s@github.com/%s/%s.git";

  private final ReleaseDirectoryHandler directoryHandler;
  private final GitHubAccessProvider gitHubAccessProvider;

  @Inject
  public GitRepoInitTask(
    @NotNull ReleaseDirectoryHandler directoryHandler,
    @NotNull GitHubAccessProvider gitHubAccessProvider
  ) {
    super("Base Git Repo Init");
    this.directoryHandler = directoryHandler;
    this.gitHubAccessProvider = gitHubAccessProvider;
  }

  @Override
  protected @NotNull Object internalExecute(
    @NotNull TaskExecutionContext<?, ?> context,
    @NotNull GHRelease input
  ) throws Exception {
    // get a token to fetch the repo & build the fetch url
    var cloneToken = this.gitHubAccessProvider.accessToken();
    var fetchUrl = GH_APP_FETCH_URL_FORMAT.formatted(
      cloneToken,
      input.getOwner().getOwnerName(),
      input.getOwner().getName());

    // clone the repo initially if needed
    var baseRepoDir = this.directoryHandler.baseRepositoryDirectory();
    if (Files.notExists(baseRepoDir)) {
      // clone the repo
      Git.cloneRepository()
        .setURI(fetchUrl)
        .setNoCheckout(true)
        .setDirectory(baseRepoDir.toFile())
        .call()
        .close();
    } else {
      // open the existing git repo and set the new fetch url for it
      try (var git = Git.open(baseRepoDir.toFile())) {
        git.remoteSetUrl()
          .setRemoteUri(new URIish(fetchUrl))
          .setRemoteName(Constants.DEFAULT_REMOTE_NAME)
          .setUriType(RemoteSetUrlCommand.UriType.FETCH)
          .call();
      }
    }

    // copy the base directory to the target directory for the release
    var deploymentDir = this.directoryHandler.resolveDeploymentDirectory(input);
    PathUtils.copyDirectory(baseRepoDir, deploymentDir);

    // register a cancel listener which removes the created deployment directory
    context.registerCancellationTask(
      () -> PathUtils.deleteDirectory(deploymentDir, StandardDeleteOption.OVERRIDE_READ_ONLY));

    // construct the result
    return Pair.of(input, deploymentDir);
  }
}
