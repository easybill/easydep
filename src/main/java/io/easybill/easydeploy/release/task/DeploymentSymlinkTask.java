package io.easybill.easydeploy.release.task;

import com.google.inject.Inject;
import io.easybill.easydeploy.release.ReleaseDirectoryHandler;
import io.easybill.easydeploy.task.ChainedTask;
import io.easybill.easydeploy.task.TaskExecutionContext;
import io.easybill.easydeploy.util.SymlinkUtil;
import io.easybill.easydeploy.util.TokenizedInputParser;
import io.github.cdimascio.dotenv.Dotenv;
import java.nio.file.Path;
import java.util.Set;
import java.util.stream.Collectors;
import org.apache.commons.lang3.tuple.Pair;
import org.jetbrains.annotations.NotNull;
import org.kohsuke.github.GHRelease;

public final class DeploymentSymlinkTask extends ChainedTask<Pair<GHRelease, Path>> {

  private final ReleaseDirectoryHandler directoryHandler;
  private final Set<Pair<String, Path>> additionalSymlinks;

  @Inject
  public DeploymentSymlinkTask(@NotNull ReleaseDirectoryHandler directoryHandler, @NotNull Dotenv env) {
    super("Deployment Symlink");
    this.directoryHandler = directoryHandler;

    // parse the additional symlinks
    var additionalLinks = env.get("EASYDEP_DEPLOY_ADDITIONAL_SYMLINKS", "");
    this.additionalSymlinks = TokenizedInputParser.tokenizeInput(additionalLinks).entrySet().stream()
      .map(entry -> {
        var targetPath = Path.of(entry.getValue()).normalize().toAbsolutePath();
        return Pair.of(entry.getKey(), targetPath);
      })
      .collect(Collectors.toSet());
  }

  @Override
  protected @NotNull Object internalExecute(
    @NotNull TaskExecutionContext<?, ?> context,
    @NotNull Pair<GHRelease, Path> input
  ) throws Exception {
    // link the current deployment directory to the created deployment directory
    SymlinkUtil.createSymlink(this.directoryHandler.currentDeploymentDirectory(), input.getRight());

    // create all additional symlinks
    for (var additionalSymlink : this.additionalSymlinks) {
      var linkPath = input.getRight().resolve(additionalSymlink.getLeft()).normalize().toAbsolutePath();
      SymlinkUtil.createSymlink(linkPath, additionalSymlink.getRight());
    }

    return input.getLeft();
  }
}
