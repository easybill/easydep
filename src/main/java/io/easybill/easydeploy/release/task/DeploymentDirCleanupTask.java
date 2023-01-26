package io.easybill.easydeploy.release.task;

import io.easybill.easydeploy.task.ChainedTask;
import io.easybill.easydeploy.task.TaskExecutionContext;
import java.nio.file.Path;
import org.apache.commons.io.file.PathUtils;
import org.apache.commons.io.file.StandardDeleteOption;
import org.apache.commons.lang3.tuple.Pair;
import org.jetbrains.annotations.NotNull;
import org.kohsuke.github.GHRelease;

public final class DeploymentDirCleanupTask extends ChainedTask<Pair<GHRelease, Path>> {

  private static final String GIT_FOLDER = ".git";

  public DeploymentDirCleanupTask() {
    super("Deployment Directory Cleanup");
  }

  @Override
  protected @NotNull Object internalExecute(
    @NotNull TaskExecutionContext<?, ?> context,
    @NotNull Pair<GHRelease, Path> input
  ) throws Exception {
    // remove the .git directory from the deployment folder
    var gitDirectory = input.getRight().resolve(GIT_FOLDER);
    PathUtils.deleteDirectory(gitDirectory, StandardDeleteOption.OVERRIDE_READ_ONLY);

    // pass through the input to the next task
    return input;
  }
}
