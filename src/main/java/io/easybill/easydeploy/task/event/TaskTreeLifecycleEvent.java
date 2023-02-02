package io.easybill.easydeploy.task.event;

import io.easybill.easydeploy.task.ChainedTask;
import io.easybill.easydeploy.task.TaskTreeLifecycle;
import org.jetbrains.annotations.NotNull;

public class TaskTreeLifecycleEvent {

  private final ChainedTask<?> lastTask;
  private final TaskTreeLifecycle lifecycle;

  public TaskTreeLifecycleEvent(@NotNull ChainedTask<?> lastTask, @NotNull TaskTreeLifecycle lifecycle) {
    this.lastTask = lastTask;
    this.lifecycle = lifecycle;
  }

  public @NotNull ChainedTask<?> lastTask() {
    return this.lastTask;
  }

  public @NotNull TaskTreeLifecycle lifecycle() {
    return this.lifecycle;
  }
}
