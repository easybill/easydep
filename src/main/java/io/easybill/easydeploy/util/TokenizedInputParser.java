package io.easybill.easydeploy.util;

import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;
import org.jetbrains.annotations.NotNull;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public final class TokenizedInputParser {

  private static final Logger LOGGER = LoggerFactory.getLogger(TokenizedInputParser.class);

  private static final String VALUE_DELIMITER = ";;";
  private static final Pattern TOKEN_PATTERN = Pattern.compile("^([a-zA-Z0-9_./\\- ]+):(.+)$");

  private TokenizedInputParser() {
    throw new UnsupportedOperationException();
  }

  public static @NotNull Map<String, String> tokenizeInput(@NotNull String input) {
    // we require the keys/values to not contain a semicolon, therefore we
    // can split at each semicolon to get the input groups
    var groups = input.split(VALUE_DELIMITER);

    // match each group input and sort out invalid items
    Map<String, String> target = new HashMap<>();
    for (var group : groups) {
      // skip blank lines
      if (group.isBlank()) {
        continue;
      }

      var matcher = TOKEN_PATTERN.matcher(group);
      if (matcher.matches()) {
        // check for duplicate keys
        var knownValue = target.putIfAbsent(matcher.group(1), matcher.group(2));
        if (knownValue != null) {
          LOGGER.warn(
            "Detected duplicate token key {} (First Value: {}, Current Value: {})",
            matcher.group(1), knownValue, matcher.group(2));
        }
      } else {
        LOGGER.warn("Unexpected token encountered: {} - Should be in the format \"key:value;;\"", group);
      }
    }

    return target;
  }

  public static @NotNull List<String> splitAtDelimiter(@NotNull String input) {
    return List.of(input.split(VALUE_DELIMITER));
  }
}
