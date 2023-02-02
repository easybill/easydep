package io.easybill.easydeploy.release.task;

import dev.derklaro.aerogel.Inject;
import io.easybill.easydeploy.release.handler.ScriptExecutionHandler;
import io.easybill.easydeploy.task.ChainedTask;
import io.easybill.easydeploy.task.TaskExecutionContext;
import java.nio.file.Path;
import org.apache.commons.lang3.tuple.Pair;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;
import org.kohsuke.github.GHRelease;

public final class DeployScriptExecuteTask extends ChainedTask<Pair<GHRelease, Path>> {

  private static final String DEPLOY_SCRIPT_NAME = "execute.sh";
  private static final String DEPLOY_LOG_FORMAT = "Deployment %s";

  private final ScriptExecutionHandler scriptExecutionHandler;

  @Inject
  public DeployScriptExecuteTask(@NotNull ScriptExecutionHandler scriptExecutionHandler) {
    super("Deployment Script Execute");
    this.scriptExecutionHandler = scriptExecutionHandler;
  }

  @Override
  protected @Nullable Object internalExecute(
    @NotNull TaskExecutionContext<?, ?> context,
    @NotNull Pair<GHRelease, Path> input
  ) throws Exception {
    // run the deployment script
    this.scriptExecutionHandler.runScriptIfExists(
      input.getRight(),
      DEPLOY_SCRIPT_NAME,
      DEPLOY_LOG_FORMAT.formatted(input.getLeft().getId()),
      context,
      input);

    // return nothing and let the context wait for the future
    return null;
  }
}
