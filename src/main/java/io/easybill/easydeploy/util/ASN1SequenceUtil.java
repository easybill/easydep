package io.easybill.easydeploy.util;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Base64;
import java.util.regex.Pattern;
import org.bouncycastle.asn1.ASN1Primitive;
import org.bouncycastle.asn1.ASN1Sequence;
import org.jetbrains.annotations.NotNull;

public final class ASN1SequenceUtil {

  private static final Pattern PKCS1_KEY_PATTERN =
    Pattern.compile("^--+BEGIN RSA PRIVATE KEY--+ (.*) --+.+END.*--+");

  private ASN1SequenceUtil() {
    throw new UnsupportedOperationException();
  }

  public static @NotNull ASN1Primitive fromInputOrFile(@NotNull String input) throws IOException {
    var matcher = PKCS1_KEY_PATTERN.matcher(input);
    if (matcher.matches()) {
      // decode the base64 encoded content
      var pkcs1Key = Base64.getMimeDecoder().decode(matcher.group(1));
      return ASN1Sequence.fromByteArray(pkcs1Key);
    } else {
      // try to read from file
      var filePath = Path.of(input);
      if (Files.exists(filePath)) {
        var keyBytes = Files.readAllBytes(filePath);
        return ASN1Sequence.fromByteArray(keyBytes);
      }
    }

    // unable to parse
    throw new IllegalArgumentException("Unable to read asn sequence from input/file: " + input);
  }
}
