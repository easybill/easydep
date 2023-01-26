package io.easybill.easydeploy.release.task;

import com.electronwill.nightconfig.toml.TomlFormat;
import com.electronwill.nightconfig.toml.TomlParser;
import io.easybill.easydeploy.task.ChainedTask;
import io.easybill.easydeploy.task.TaskExecutionContext;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;
import org.kohsuke.github.GHRelease;

public final class TagCheckTask extends ChainedTask<GHRelease> {

  private static final TomlFormat TOML_FORMAT = TomlFormat.instance();
  private static final ThreadLocal<TomlParser> TOML_PARSER = ThreadLocal.withInitial(TOML_FORMAT::createParser);

  public TagCheckTask() {
    super("TagCheck");
  }

  @Override
  protected @Nullable Object internalExecute(
    @NotNull TaskExecutionContext<?, ?> context,
    @NotNull GHRelease input
  ) throws Exception {
    // check the body of the release for further input in form of a toml config
    var body = input.getBody();
    if (!body.isBlank()) {
      // parse the body
      var tomlParser = TOML_PARSER.get();
      var parsedBody = tomlParser.parse(body);

      // todo: request cancel if tags don't match
    }

    return input;
  }
}
