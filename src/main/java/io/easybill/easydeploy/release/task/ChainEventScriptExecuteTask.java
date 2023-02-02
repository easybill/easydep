package io.easybill.easydeploy.release.task;

import dev.derklaro.aerogel.Inject;
import io.easybill.easydeploy.release.handler.ScriptExecutionHandler;
import io.easybill.easydeploy.task.ChainedTask;
import io.easybill.easydeploy.task.TaskExecutionContext;
import io.easybill.easydeploy.task.TaskTreeLifecycle;
import io.easybill.easydeploy.task.event.TaskTreeLifecycleEvent;
import java.io.IOException;
import java.nio.file.Path;
import org.apache.commons.lang3.tuple.Pair;
import org.jetbrains.annotations.NotNull;
import org.kohsuke.github.GHRelease;

public final class ChainEventScriptExecuteTask extends ChainedTask<Pair<GHRelease, Path>> {

  private final ScriptExecutionHandler scriptExecutionHandler;

  @Inject
  public ChainEventScriptExecuteTask(@NotNull ScriptExecutionHandler scriptExecutionHandler) {
    super("Chain Event Execute");
    this.scriptExecutionHandler = scriptExecutionHandler;
  }

  @Override
  protected @NotNull Object internalExecute(
    @NotNull TaskExecutionContext<?, ?> context,
    @NotNull Pair<GHRelease, Path> input
  ) throws Exception {
    // add a listener that listens to all events that are published in the event chain
    context.eventPipeline().registerConsumer(TaskTreeLifecycleEvent.class, lifecycleEvent -> {
      // normalize the lifecycle name
      var lifecycle = lifecycleEvent.lifecycle();
      var normalizedLifecycleName = lifecycle.name().toLowerCase();

      // check if we should include the task name (this is only relevant if the event is not called for the whole ctx)
      if (lifecycle == TaskTreeLifecycle.TASK_FAILURE || lifecycle == TaskTreeLifecycle.TASK_SUCCESS) {
        // include a normalized version of the task name (all lower case, spaces replaced with underscore)
        var normalizedTaskName = lifecycleEvent.lastTask().displayName().toLowerCase().replace(' ', '_');
        this.runScript(input.getRight(), "%s.%s".formatted(normalizedLifecycleName, normalizedTaskName));
      } else {
        // no need to append the task name, just use the lifecycle name
        this.runScript(input.getRight(), "%s".formatted(normalizedLifecycleName));
      }
    }, /* very low priority to get called first */ 0);

    return input;
  }

  private void runScript(@NotNull Path directory, @NotNull String scriptName) throws IOException {
    // append the script suffix and pass on the execution to the handler
    var finalScriptName = "%s.sh".formatted(scriptName);
    this.scriptExecutionHandler.runScriptIfExists(directory, finalScriptName, "Lifecycle Event", null, null);
  }
}
