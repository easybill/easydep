package io.easybill.easydeploy.release;

import com.google.inject.Inject;
import com.google.inject.Provider;
import com.google.inject.Singleton;
import io.easybill.easydeploy.release.task.DeployScriptExecuteTask;
import io.easybill.easydeploy.release.task.DeploymentDirCleanupTask;
import io.easybill.easydeploy.release.task.DeploymentSymlinkTask;
import io.easybill.easydeploy.release.task.GitPrepareTask;
import io.easybill.easydeploy.release.task.GitRepoInitTask;
import io.easybill.easydeploy.release.task.TagCheckTask;
import io.easybill.easydeploy.task.ChainedTask;
import java.nio.file.Path;
import org.apache.commons.lang3.tuple.Pair;
import org.jetbrains.annotations.NotNull;
import org.kohsuke.github.GHRelease;

@Singleton
final class ReleaseTaskProvider {

  private final Provider<TagCheckTask> tagCheckTaskProvider;
  private final Provider<GitRepoInitTask> gitRepoInitTaskProvider;
  private final Provider<DeploymentSymlinkTask> symlinkTaskProvider;

  @Inject
  public ReleaseTaskProvider(
    @NotNull Provider<TagCheckTask> tagCheckTaskProvider,
    @NotNull Provider<GitRepoInitTask> gitRepoInitTaskProvider,
    @NotNull Provider<DeploymentSymlinkTask> symlinkTaskProvider
  ) {
    this.tagCheckTaskProvider = tagCheckTaskProvider;
    this.gitRepoInitTaskProvider = gitRepoInitTaskProvider;
    this.symlinkTaskProvider = symlinkTaskProvider;
  }

  public @NotNull ChainedTask<GHRelease> buildDeployTaskChain() {
    return this.tagCheckTaskProvider.get()
      .addLeafTask(this.gitRepoInitTaskProvider.get())
      .addLeafTask(new GitPrepareTask())
      .addLeafTask(new DeploymentDirCleanupTask())
      .addLeafTask(new DeployScriptExecuteTask())
      .addLeafTask(this.symlinkTaskProvider.get());
  }

  public @NotNull ChainedTask<Pair<GHRelease, Path>> buildDirectorySymlinkTaskChain() {
    return this.symlinkTaskProvider.get();
  }
}
