package io.easybill.easydeploy.release;

import dev.derklaro.aerogel.Inject;
import dev.derklaro.aerogel.Name;
import dev.derklaro.aerogel.Singleton;
import io.easybill.easydeploy.release.handler.ReleaseDirectoryHandler;
import io.easybill.easydeploy.release.task.DeploymentSymlinkTask;
import io.easybill.easydeploy.task.ChainedTask;
import io.easybill.easydeploy.task.TaskExecutionContext;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.locks.Lock;
import java.util.concurrent.locks.ReentrantLock;
import org.apache.commons.lang3.tuple.Pair;
import org.jetbrains.annotations.NotNull;
import org.kohsuke.github.GHRelease;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

@Singleton
public final class ReleaseProcessor {

  private static final Logger LOGGER = LoggerFactory.getLogger(ReleaseProcessor.class);

  private final Lock deploymentLock = new ReentrantLock(true);

  private final ChainedTask<GHRelease> deployTaskChain;
  private final ChainedTask<Pair<GHRelease, Path>> deployRollbackTaskChain;

  private final ReleaseDirectoryHandler releaseDirectoryHandler;

  private volatile long lastExecutedRelease;
  private volatile Pair<GHRelease, TaskExecutionContext<?, ?>> lastScheduledRelease;

  @Inject
  public ReleaseProcessor(
    @NotNull ReleaseDirectoryHandler directoryHandler,
    @NotNull DeploymentSymlinkTask deployRollbackTaskChain,
    @NotNull @Name("deploy") ChainedTask<GHRelease> deployTaskChain
  ) {
    this.releaseDirectoryHandler = directoryHandler;

    // set the task injected task chains
    this.deployTaskChain = deployTaskChain;
    this.deployRollbackTaskChain = deployRollbackTaskChain;

    // resolve the last executed deployment by trying to follow the symlink of the current
    // deployment directory. The associated deployment directory name is the last release we processed.
    // If there is no association just set the last id to -1 which will cause the enqueue method to process
    // the next incoming release as a new one
    try {
      var currentDeploymentDir = directoryHandler.currentDeploymentDirectory().toRealPath();
      this.lastExecutedRelease = Long.parseLong(currentDeploymentDir.getFileName().toString());
      LOGGER.info("Resolved last executed release id: {}", this.lastExecutedRelease);
    } catch (IOException exception) {
      this.lastExecutedRelease = -1;
      LOGGER.warn("Unable to resolve last executed deployment: {}", exception.getMessage());
    }
  }

  public void enqueueRelease(@NotNull GHRelease release) {
    this.deploymentLock.lock();
    try {
      if (release.getId() > this.lastExecutedRelease) {
        // new release we should process
        this.lastExecutedRelease = release.getId();
        this.cancelRunningRelease();
        this.processNewRelease(release);
      }

      if (this.lastExecutedRelease > release.getId()) {
        // rollback to an old release
        this.lastExecutedRelease = release.getId();
        this.cancelRunningRelease();
        this.rollbackToOldRelease(release);
      }
    } finally {
      this.deploymentLock.unlock();
    }
  }

  private void cancelRunningRelease() {
    var lastScheduledRelease = this.lastScheduledRelease;
    if (lastScheduledRelease != null) {
      LOGGER.debug("Cancelling currently running release operation");

      this.lastScheduledRelease = null;
      lastScheduledRelease.getRight().cancel();
    }
  }

  private void processNewRelease(@NotNull GHRelease release) {
    this.scheduleReleaseTask(release, this.deployTaskChain, release);
  }

  private void rollbackToOldRelease(@NotNull GHRelease release) {
    // get the release directory for the release and schedule a rollback if it still exists
    var deploymentDirectory = this.releaseDirectoryHandler.resolveDeploymentDirectory(release);
    if (Files.exists(deploymentDirectory)) {
      // schedule the rollback task
      var taskInput = Pair.of(release, deploymentDirectory);
      this.scheduleReleaseTask(release, this.deployRollbackTaskChain, taskInput);
    } else {
      // the release no longer exists, treat it as a new release
      this.processNewRelease(release);
    }
  }

  private <I, O> @NotNull CompletableFuture<O> scheduleReleaseTask(
    @NotNull GHRelease release,
    @NotNull ChainedTask<I> task,
    @NotNull I input
  ) {
    // enter a task context and set the release and the context as the last we've scheduled
    TaskExecutionContext<I, O> taskContext = task.enterContext();
    this.lastScheduledRelease = Pair.of(release, taskContext);

    // execute the task
    return taskContext.scheduleExecution(input);
  }
}
