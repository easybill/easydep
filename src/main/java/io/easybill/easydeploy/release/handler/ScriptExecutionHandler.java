package io.easybill.easydeploy.release.handler;

import dev.derklaro.aerogel.Singleton;
import io.easybill.easydeploy.task.TaskExecutionContext;
import io.easybill.easydeploy.task.TaskTreeLifecycle;
import io.easybill.easydeploy.task.event.TaskTreeLifecycleEvent;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Locale;
import java.util.Map;
import java.util.Random;
import java.util.concurrent.CompletableFuture;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

@Singleton
public final class ScriptExecutionHandler {

  public static final String LOG_DIR_NAME = ".scriptlog";

  private static final Logger LOGGER = LoggerFactory.getLogger(ScriptExecutionHandler.class);

  private static final Random RANDOM = new Random();
  private static final String EASYDEP_DIRECTORY_FORMAT = ".easydep/%s";

  public void runScriptIfExists(
    @NotNull Path directory,
    @NotNull String scriptName,
    @NotNull String scriptLogId,
    @NotNull Map<String, String> env,
    @Nullable TaskExecutionContext<?, ?> context,
    @Nullable Object successfulScriptReturnValue
  ) throws IOException {
    // validate that the script exists
    var scriptPathName = EASYDEP_DIRECTORY_FORMAT.formatted(scriptName);
    if (Files.notExists(directory.resolve(scriptPathName))) {
      LOGGER.debug("Unable to execute script at {}: script is missing", scriptPathName);
      return;
    }

    // create a temporary file that catches the log output of the script process
    var logFilePath = directory.resolve(LOG_DIR_NAME).resolve("%s.tmp".formatted(RANDOM.nextLong()));
    this.createLogFile(logFilePath);

    // build the process
    var processBuilder = new ProcessBuilder("bash", scriptPathName)
      .directory(directory.toFile())
      .redirectErrorStream(true)
      .redirectOutput(logFilePath.toFile());
    for (var envVarEntry : env.entrySet()) {
      processBuilder.environment().put(envVarEntry.getKey().toUpperCase(Locale.ROOT), envVarEntry.getValue());
    }

    // start the script process
    var process = processBuilder.start();

    // if a context is given, ensure that we destroy the process in case the execution fails
    var processHandle = process.toHandle();
    if (context != null) {
      context.eventPipeline().registerConsumer(TaskTreeLifecycleEvent.class, lifecycleEvent -> {
        if (lifecycleEvent.lifecycle() == TaskTreeLifecycle.CHAIN_FAILURE) {
          processHandle.destroyForcibly();
        }
      });

      // attach the log file information to the context
      context.registerAdditionalInformation(
        "easydep_%s_log".formatted(scriptName.replace('.', '_')),
        logFilePath.toAbsolutePath().toString());

      // configure the future based on the context wait method
      context.waitForFutureCompletion(
        processHandle.onExit(),
        future -> this.configureFuture(logFilePath, scriptLogId, process, future, successfulScriptReturnValue));
    } else {
      // just configure the exit future
      this.configureFuture(logFilePath, scriptLogId, process, processHandle.onExit(), successfulScriptReturnValue);
    }
  }

  private @NotNull CompletableFuture<?> configureFuture(
    @NotNull Path logPath,
    @NotNull String scriptLogId,
    @NotNull Process startedProcess,
    @NotNull CompletableFuture<?> input,
    @Nullable Object successfulScriptReturnValue
  ) {
    return input
      .thenAccept(handle -> {
        try {
          // print out the process log file lines to the target logger
          var logLines = Files.readAllLines(logPath, StandardCharsets.UTF_8);
          logLines.forEach(line -> LOGGER.info("[{}]: {}", scriptLogId, line));
        } catch (IOException ignored) {
        }
      })
      .thenApply(handle -> {
        // check if the process completed successfully
        var exitCode = startedProcess.exitValue();
        if (exitCode != 0) {
          throw new IllegalStateException("Script Process exited with non-zero exit code: " + exitCode);
        }
        return successfulScriptReturnValue;
      });
  }

  private void createLogFile(@NotNull Path logFilePath) throws IOException {
    // ensure that the parent directory exists
    var parent = logFilePath.getParent();
    if (parent != null && Files.notExists(parent)) {
      Files.createDirectories(parent);
    }

    // create the log file, if needed
    if (Files.notExists(logFilePath)) {
      Files.createFile(logFilePath);
    }
  }
}
