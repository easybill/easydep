package io.easybill.easydeploy.task.event;

import io.easybill.easydeploy.task.ChainedTask;
import io.easybill.easydeploy.task.TaskTreeLifecycle;
import org.jetbrains.annotations.NotNull;

public final class TaskTreeTaskFinishedEvent extends TaskTreeLifecycleEvent {

  private final Object taskResult;

  public TaskTreeTaskFinishedEvent(@NotNull ChainedTask<?> lastTask, @NotNull Object taskResult) {
    super(lastTask, TaskTreeLifecycle.TASK_SUCCESS);
    this.taskResult = taskResult;
  }

  public @NotNull Object taskResult() {
    return this.taskResult;
  }
}
