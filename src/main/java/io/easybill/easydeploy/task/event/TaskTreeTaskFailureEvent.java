package io.easybill.easydeploy.task.event;

import io.easybill.easydeploy.task.ChainedTask;
import io.easybill.easydeploy.task.TaskTreeLifecycle;
import org.jetbrains.annotations.NotNull;

public final class TaskTreeTaskFailureEvent extends TaskTreeLifecycleEvent {

  private final Throwable caughtException;

  public TaskTreeTaskFailureEvent(@NotNull ChainedTask<?> lastTask, @NotNull Throwable caughtException) {
    super(lastTask, TaskTreeLifecycle.TASK_FAILURE);
    this.caughtException = caughtException;
  }

  public @NotNull Throwable caughtException() {
    return this.caughtException;
  }
}
