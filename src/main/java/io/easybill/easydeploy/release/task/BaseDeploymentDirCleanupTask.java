package io.easybill.easydeploy.release.task;

import com.google.common.primitives.Longs;
import com.google.inject.Inject;
import io.easybill.easydeploy.release.ReleaseDirectoryHandler;
import io.easybill.easydeploy.task.ChainedTask;
import io.easybill.easydeploy.task.TaskExecutionContext;
import io.github.cdimascio.dotenv.Dotenv;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Collections;
import java.util.Comparator;
import java.util.stream.Collectors;
import org.apache.commons.io.file.PathUtils;
import org.apache.commons.io.file.StandardDeleteOption;
import org.apache.commons.lang3.tuple.Pair;
import org.jetbrains.annotations.NotNull;
import org.kohsuke.github.GHRelease;

public final class BaseDeploymentDirCleanupTask extends ChainedTask<GHRelease> {

  private static final Comparator<Pair<Path, Long>> DIR_ID_COMPARATOR = Comparator.comparing(
    Pair::getRight,
    Collections.reverseOrder());

  private final int maxStoredReleases;
  private final ReleaseDirectoryHandler directoryHandler;

  @Inject
  public BaseDeploymentDirCleanupTask(@NotNull Dotenv env, @NotNull ReleaseDirectoryHandler directoryHandler) {
    super("Deployment Base Dir Cleanup");
    this.directoryHandler = directoryHandler;

    // parse the max stored releases as an int, but ensure that we at least keep 2 releases
    var maxStoredReleases = Integer.parseInt(env.get("EASYDEP_DEPLOY_DISCARDER_MAX", "10"));
    this.maxStoredReleases = maxStoredReleases > 0 ? Math.max(2, maxStoredReleases) : -1;
  }

  @Override
  protected @NotNull Object internalExecute(
    @NotNull TaskExecutionContext<?, ?> context,
    @NotNull GHRelease input
  ) throws Exception {
    // check if we should delete any releases
    if (this.maxStoredReleases != -1) {
      try (var stream = Files.walk(this.directoryHandler.deploymentBaseDirectory(), 0)) {
        var directoriesToRemove = stream
          .filter(path -> !Files.isSymbolicLink(path) && Files.isDirectory(path))
          .map(path -> {
            // parsing the file name to a long here makes it easier
            //  - to compare the directories
            //  - to ensure that the directory really is a deployed directory and not something we shouldn't delete
            var parsedId = Longs.tryParse(path.getFileName().toString());
            return Pair.of(path, parsedId);
          })
          .filter(pair -> pair.getRight() != null)
          .sorted(DIR_ID_COMPARATOR) // sort by the name of the directory (which is the release id)
          .skip(this.maxStoredReleases) // skip the newest releases
          .map(Pair::getLeft)
          .collect(Collectors.toSet());

        // remove the resolved directories
        if (!directoriesToRemove.isEmpty()) {
          for (var dir : directoriesToRemove) {
            PathUtils.deleteDirectory(dir, StandardDeleteOption.OVERRIDE_READ_ONLY);
          }
        }
      }
    }

    return input;
  }
}
