package io.easybill.easydeploy.task;

import org.jetbrains.annotations.Contract;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

public abstract class ChainedTask<I> {

  protected final String displayName;

  protected ChainedTask<Object> next;

  protected ChainedTask(@NotNull String displayName) {
    this.displayName = displayName;
  }

  protected abstract @Nullable Object internalExecute(
    @NotNull TaskExecutionContext<?, ?> context,
    @NotNull I input
  ) throws Exception;

  public @NotNull <V> TaskExecutionContext<I, V> enterContext() {
    return new TaskExecutionContext<>(this);
  }

  @Contract("_ -> this")
  public @NotNull <V> ChainedTask<I> addLeafTask(@NotNull ChainedTask<V> downstream) {
    // find the current leaf task
    ChainedTask<?> task = this;
    do {
      if (task.next == null) {
        //noinspection unchecked
        task.next = (ChainedTask<Object>) downstream;
        break;
      } else {
        task = task.next;
      }
    } while (true);
    return this;
  }
}
