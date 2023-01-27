package io.easybill.easydeploy.release.task;

import com.fasterxml.jackson.databind.node.ObjectNode;
import com.fasterxml.jackson.dataformat.toml.TomlMapper;
import com.google.inject.Inject;
import io.easybill.easydeploy.task.ChainedTask;
import io.easybill.easydeploy.task.TaskExecutionContext;
import io.easybill.easydeploy.util.TokenizedInputParser;
import io.github.cdimascio.dotenv.Dotenv;
import java.util.Map;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;
import org.kohsuke.github.GHRelease;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public final class TagCheckTask extends ChainedTask<GHRelease> {

  private static final TomlMapper TOML_MAPPER = new TomlMapper();
  private static final Logger LOGGER = LoggerFactory.getLogger(TagCheckTask.class);

  private final Map<String, String> ourLabels;

  @Inject
  public TagCheckTask(@NotNull Dotenv env) {
    super("TagCheck");
    this.ourLabels = TokenizedInputParser.tokenizeInput(env.get("EASYDEP_DEPLOY_LABELS", ""));
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
      var parsedBody = TOML_MAPPER.readTree(body);

      // check if there were any labels submitted
      var labels = parsedBody.get("labels");
      if (labels instanceof ObjectNode objectNode) {
        // ensure that each label value matches
        var fields = objectNode.fields();
        while (fields.hasNext()) {
          var entry = fields.next();

          var labelName = entry.getKey();
          var presenceRequired = true;

          // if the label name ends with a question mark the presence is not required
          if (labelName.endsWith("?")) {
            presenceRequired = false;
            labelName = labelName.substring(0, labelName.length() - 1);
          }

          // check if a label with the key is registered locally, ignore the label if not
          var localValue = this.ourLabels.get(labelName);
          if (localValue == null) {
            if (presenceRequired) {
              LOGGER.debug("Ignoring release {} - required label {} is not set locally", input.getId(), labelName);

              // cancel the execution
              context.cancel();
              return null;
            } else {
              // not required, keep searching
              continue;
            }
          }

          // check if the given label values contain at least one match for our local label
          var possibleValues = TokenizedInputParser.splitAtDelimiter(entry.getValue().asText());
          if (!possibleValues.contains(localValue)) {
            LOGGER.debug(
              "Ignoring release {} as it doesn't target the current server (label mismatch) - Expected one of {} for {}; got {}",
              input.getId(), possibleValues, labelName, localValue);

            // cancel the execution
            context.cancel();
            return null;
          }
        }
      }
    }

    return input;
  }
}
