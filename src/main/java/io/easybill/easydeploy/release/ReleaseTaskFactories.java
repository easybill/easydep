package io.easybill.easydeploy.release;

import dev.derklaro.aerogel.Name;
import dev.derklaro.aerogel.auto.Factory;
import io.easybill.easydeploy.release.task.BaseDeploymentDirCleanupTask;
import io.easybill.easydeploy.release.task.ChainEventScriptExecuteTask;
import io.easybill.easydeploy.release.task.DeployScriptExecuteTask;
import io.easybill.easydeploy.release.task.DeploymentDirCleanupTask;
import io.easybill.easydeploy.release.task.DeploymentSymlinkTask;
import io.easybill.easydeploy.release.task.GitPrepareTask;
import io.easybill.easydeploy.release.task.GitRepoInitTask;
import io.easybill.easydeploy.release.task.TagCheckTask;
import io.easybill.easydeploy.task.ChainedTask;
import org.jetbrains.annotations.NotNull;
import org.kohsuke.github.GHRelease;

final class ReleaseTaskFactories {

  private ReleaseTaskFactories() {
    throw new UnsupportedOperationException();
  }

  @Factory
  @Name("deploy")
  private static @NotNull ChainedTask<GHRelease> buildDeployTaskChain(
    @NotNull TagCheckTask tagCheckTask,
    @NotNull GitRepoInitTask gitRepoInitTask,
    @NotNull ChainEventScriptExecuteTask eventScriptExecuteTask,
    @NotNull DeployScriptExecuteTask scriptExecuteTask,
    @NotNull DeploymentSymlinkTask symlinkTask,
    @NotNull BaseDeploymentDirCleanupTask cleanupTask
  ) {
    return tagCheckTask
      .addLeafTask(gitRepoInitTask)
      .addLeafTask(new GitPrepareTask())
      .addLeafTask(eventScriptExecuteTask)
      .addLeafTask(new DeploymentDirCleanupTask())
      .addLeafTask(scriptExecuteTask)
      .addLeafTask(symlinkTask)
      .addLeafTask(cleanupTask);
  }
}
