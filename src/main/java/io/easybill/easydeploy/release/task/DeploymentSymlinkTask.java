package io.easybill.easydeploy.release.task;

import com.google.inject.Inject;
import io.easybill.easydeploy.release.ReleaseDirectoryHandler;
import io.easybill.easydeploy.task.ChainedTask;
import io.easybill.easydeploy.task.TaskExecutionContext;
import java.nio.file.Files;
import java.nio.file.Path;
import org.apache.commons.lang3.tuple.Pair;
import org.jetbrains.annotations.NotNull;
import org.kohsuke.github.GHRelease;

public final class DeploymentSymlinkTask extends ChainedTask<Pair<GHRelease, Path>> {

  private final ReleaseDirectoryHandler directoryHandler;

  @Inject
  public DeploymentSymlinkTask(@NotNull ReleaseDirectoryHandler directoryHandler) {
    super("Deployment Symlink");
    this.directoryHandler = directoryHandler;
  }

  @Override
  protected @NotNull Object internalExecute(
    @NotNull TaskExecutionContext<?, ?> context,
    @NotNull Pair<GHRelease, Path> input
  ) throws Exception {
    // remove the current symlink
    var currentDirectory = this.directoryHandler.currentDeploymentDirectory();
    Files.deleteIfExists(currentDirectory);

    // link the current deployment dir as the new current dir
    Files.createSymbolicLink(currentDirectory, input.getRight());

    // todo: more linking needed here? do that before linking the current dir?

    return input.getLeft();
  }
}
