package io.easybill.easydeploy.task;

import io.easybill.easydeploy.event.EventPipeline;
import io.easybill.easydeploy.task.event.TaskTreeLifecycleEvent;
import io.easybill.easydeploy.task.event.TaskTreeTaskFailureEvent;
import io.easybill.easydeploy.task.event.TaskTreeTaskFinishedEvent;
import java.util.HashMap;
import java.util.Map;
import java.util.concurrent.CancellationException;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.UnaryOperator;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public final class TaskExecutionContext<I, O> {

  private static final Logger LOGGER = LoggerFactory.getLogger(TaskExecutionContext.class);
  private static final ExecutorService TASK_EXECUTOR = Executors.newSingleThreadExecutor();

  private static final CancellationException CONTEXT_CANCEL_MARKER = new CancellationException(
    "Requested context task cancellation");

  private static final int STATE_READY = 0x01;
  private static final int STATE_EXECUTING = 0x02;
  private static final int STATE_WAITING = 0x04;
  private static final int STATE_CANCELLED = 0x08;
  private static final int STATE_DONE = 0x10;

  // the current state of this context
  private final AtomicInteger state = new AtomicInteger(STATE_READY);

  // the event pipeline for this context
  private final EventPipeline eventPipeline = new EventPipeline();

  // our future that will be completed with the result of the task execution
  private final CompletableFuture<O> ourFuture = new CompletableFuture<>();

  // additional step information that can be optionally passed in by the task
  private final Map<String, String> additionalTaskInformation = new HashMap<>(16);

  // the current execution state
  private ChainedTask<Object> currentTask;
  private CompletableFuture<?> waitingFuture;

  TaskExecutionContext(@NotNull ChainedTask<?> currentTask) {
    //noinspection unchecked
    this.currentTask = (ChainedTask<Object>) currentTask;
  }

  public @NotNull EventPipeline eventPipeline() {
    return this.eventPipeline;
  }

  public @NotNull CompletableFuture<O> scheduleExecution(@NotNull I input) {
    if (this.state.compareAndSet(STATE_READY, STATE_EXECUTING)) {
      // schedule the execution of the initial task, which is always the "current task" set in this context
      TASK_EXECUTOR.submit(() -> this.resumeExecutionAt(this.currentTask, input));
    }

    // return the future that the caller can use to wait for the execution to finish
    return this.ourFuture;
  }

  public <V> void waitForFutureCompletion(@NotNull CompletableFuture<V> future) {
    this.waitForFutureCompletion(future, null);
  }

  public <V> void waitForFutureCompletion(
    @NotNull CompletableFuture<V> future,
    @Nullable UnaryOperator<CompletableFuture<?>> decorator
  ) {
    // we can only do this if where currently executing a task
    // and not already waiting for an execution to finish
    if (this.state.compareAndSet(STATE_EXECUTING, STATE_WAITING)) {
      // set the current future we're waiting on
      this.waitingFuture = future;

      // call the decorator on the future if needed
      // this way mapping can be done on the result without having issues with
      // cancellation of the future chain
      CompletableFuture<?> targetFuture = future;
      if (decorator != null) {
        targetFuture = decorator.apply(future);
      }

      // resume the task execution when the future completed successfully
      var nextTask = this.currentTask.next;
      targetFuture
        .thenAcceptAsync(result -> this.resumeExecutionAt(nextTask, result), TASK_EXECUTOR)
        .exceptionally(throwable -> {
          this.postCompleteExceptionally(throwable);
          this.eventPipeline.post(new TaskTreeTaskFailureEvent(this.currentTask, throwable));
          return null;
        });

      return;
    }

    // if this context was cancelled, we can directly cancel the incoming future
    if (this.inState(STATE_CANCELLED)) {
      LOGGER.debug("Cancelling incoming future result wait request as the current context was cancelled");
      future.cancel(true);
    } else {
      throw new IllegalStateException("Unable to switch to future waiting state, current state: " + this.state.get());
    }
  }

  public void cancel() {
    // just mark this state as cancelled for now
    // in case some task is still running which uses this state, it would be too unsafe
    // to directly call all cancellation listeners as there still might be task depending on
    // previous output which might get removed by doing so
    this.state.set(STATE_CANCELLED);

    // if we're waiting for a future to complete, just let that one explode
    var waitingFuture = this.waitingFuture;
    if (waitingFuture != null) {
      waitingFuture.cancel(true);
    }
  }

  public void registerAdditionalInformation(@NotNull String key, @Nullable String value) {
    this.additionalTaskInformation.put(key, value);
  }

  public @NotNull Map<String, String> additionalTaskInformation() {
    return this.additionalTaskInformation;
  }

  private boolean inState(int expectedState) {
    return this.state.get() == expectedState;
  }

  private void resumeExecutionAt(@Nullable ChainedTask<Object> nextTask, @Nullable Object previousTaskOutput) {
    // check if this context was cancelled
    if (this.inState(STATE_CANCELLED)) {
      this.postCompleteExceptionally(CONTEXT_CANCEL_MARKER);
      return;
    }

    // check if we are (or were) waiting for a future to finish
    if (this.inState(STATE_WAITING)) {
      // the future we're listing for should be completed at this point in order to resume
      if (!this.waitingFuture.isDone()) {
        this.fail("Waiting future is not done when trying to resume");
        return;
      }

      // remove the waiting future and move back to the executing state
      this.waitingFuture = null;
      this.state.set(STATE_EXECUTING);
    }

    // only resume the execution if this context is in the executing state
    if (this.inState(STATE_EXECUTING)) {
      // validate the previous task output
      var currentTask = this.currentTask;
      if (previousTaskOutput == null) {
        this.fail("Task " + currentTask.displayName + " returned no task output");
        return;
      }

      // if the next task is null we reached the tail of the command chain
      if (nextTask == null) {
        // mark this context as done
        this.state.set(STATE_DONE);

        // notify the event pipeline that this context finished
        this.eventPipeline.post(new TaskTreeLifecycleEvent(currentTask, TaskTreeLifecycle.CHAIN_FINISH));

        // complete the waiting future with the previous task output, we expect the output
        // to always match the expected result type and can therefore do an unsafe cast here
        //noinspection unchecked
        this.ourFuture.complete((O) previousTaskOutput);
        return;
      }

      // notify the event pipeline that the previous task executed successfully & remove added task information
      this.eventPipeline.post(new TaskTreeTaskFinishedEvent(currentTask, previousTaskOutput));
      this.additionalTaskInformation.clear();

      // set the current task we're executing
      this.currentTask = nextTask;

      try {
        // execute the next task, check if a subsequent request was made to wait for a future to complete
        LOGGER.debug("Executing next task in chain: {}", nextTask.displayName);
        Object taskResult = nextTask.internalExecute(this, previousTaskOutput);
        if (this.inState(STATE_WAITING)) {
          // don't continue with executing the next task, that will be handled by the listener
          // which was added to the future for which the wait process was requested
          return;
        }

        // execute the next task
        this.resumeExecutionAt(nextTask.next, taskResult);
      } catch (Exception exception) {
        this.postCompleteExceptionally(exception);
        this.eventPipeline.post(new TaskTreeTaskFailureEvent(nextTask, exception));
      }
    }
  }

  private void fail(@NotNull String causeDescription) {
    this.postCompleteExceptionally(new IllegalStateException(causeDescription));
  }

  private void postCompleteExceptionally(@NotNull Throwable exception) {
    // notify the event pipeline that this chain failed
    this.eventPipeline.post(new TaskTreeLifecycleEvent(this.currentTask, TaskTreeLifecycle.CHAIN_FAILURE));

    // complete our future with the given exception
    this.ourFuture.completeExceptionally(exception);

    // move into the done state unless this context is marked as cancelled
    if (!this.inState(STATE_CANCELLED)) {
      this.state.set(STATE_DONE);
    }

    // for debugging reasons just print out the full exception again
    LOGGER.warn("Execution of task {} failed:", this.currentTask.displayName, exception);
  }
}
