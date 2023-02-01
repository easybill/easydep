package io.easybill.easydeploy.release.task;

import io.easybill.easydeploy.task.ChainedTask;
import io.easybill.easydeploy.task.TaskExecutionContext;
import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import org.apache.commons.lang3.tuple.Pair;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;
import org.kohsuke.github.GHRelease;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public final class DeployScriptExecuteTask extends ChainedTask<Pair<GHRelease, Path>> {

  private static final Logger LOGGER = LoggerFactory.getLogger(DeployScriptExecuteTask.class);

  private static final String DEPLOY_SCRIPT_PATH = ".easydep/execute.sh";

  public DeployScriptExecuteTask() {
    super("Deployment Script Execute");
  }

  @Override
  protected @Nullable Object internalExecute(
    @NotNull TaskExecutionContext<?, ?> context,
    @NotNull Pair<GHRelease, Path> input
  ) throws Exception {
    // resolve the log output path
    var logOutputFile = File.createTempFile("deploy-log-output", null);

    // build and start the process
    var process = new ProcessBuilder("bash", DEPLOY_SCRIPT_PATH)
      .directory(input.getRight().toFile())
      .redirectErrorStream(true)
      .redirectOutput(logOutputFile)
      .start();

    // convert the process to a handle and register a cancel hook for the process
    var processHandle = process.toHandle();
    context.registerCancellationTask(processHandle::destroyForcibly);

    // let the context wait for the process output and continue from there
    context.waitForFutureCompletion(processHandle.onExit(), future -> future
      .thenAccept(handle -> {
        try {
          // print out the process log file lines to the target logger
          var logLines = Files.readAllLines(logOutputFile.toPath(), StandardCharsets.UTF_8);
          logLines.forEach(line -> LOGGER.info("[Deployment {}]: {}", input.getLeft().getId(), line));
        } catch (IOException exception) {
          LOGGER.error("Unable to read log lines from file: {}", logOutputFile.getAbsolutePath(), exception);
        }
      })
      .thenApply(handle -> {
        // check if the process completed successfully
        var exitCode = process.exitValue();
        if (exitCode != 0) {
          throw new IllegalStateException("Deploy Script Process exited with non-zero exit code: " + exitCode);
        }
        return input;
      }));

    // return nothing and let the context wait for the future
    return null;
  }
}
