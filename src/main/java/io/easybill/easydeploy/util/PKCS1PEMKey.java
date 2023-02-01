package io.easybill.easydeploy.util;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.spec.KeySpec;
import java.security.spec.PKCS8EncodedKeySpec;
import java.util.Base64;
import org.jetbrains.annotations.NotNull;

public final class PKCS1PEMKey {

  private static final String PKCS_1_PEM_HEADER = "-----BEGIN RSA PRIVATE KEY-----";
  private static final String PKCS_1_PEM_FOOTER = "-----END RSA PRIVATE KEY-----";

  private PKCS1PEMKey() {
    throw new UnsupportedOperationException();
  }

  public static @NotNull KeySpec fromInputOrFile(@NotNull String input) throws IOException {
    // check if the given input is already a pem key
    if (input.startsWith(PKCS_1_PEM_HEADER)) {
      var rawData = input
        .replace(PKCS_1_PEM_HEADER, "") // remove header
        .replace(PKCS_1_PEM_FOOTER, "") // remove footer
        .replace("\r\n", "") // remove windows line breaks
        .replace("\n", ""); // remove unix line breaks

      // decode the key data & convert it to pkcs 8
      var pkcs1KeyData = Base64.getMimeDecoder().decode(rawData);
      var pkcs8KeyData = toPkcs8(pkcs1KeyData);

      // decode and load the key spec
      return new PKCS8EncodedKeySpec(pkcs8KeyData);
    }

    // try to load the key from the given file
    var filePath = Path.of(input);
    if (Files.exists(filePath)) {
      // ensure that we get a valid key, in this case we can call this method recursively
      var keyData = Files.readString(filePath, StandardCharsets.UTF_8);
      if (keyData.startsWith(PKCS_1_PEM_HEADER)) {
        return fromInputOrFile(keyData);
      }
    }

    // unable to parse
    throw new IllegalArgumentException("Unable to read asn sequence from input/file: " + input);
  }

  // adapted from EncryptionUtils of https://github.com/Mastercard/client-encryption-java
  private static byte[] toPkcs8(byte[] pkcs1Bytes) {
    // generate the pkcs8 header from the given pkcs1 bytes
    var pkcs1Length = pkcs1Bytes.length;
    var totalLength = pkcs1Length + 22;
    var pkcs8Header = new byte[]{
      0x30, (byte) 0x82, (byte) ((totalLength >> 8) & 0xff), (byte) (totalLength & 0xff),
      0x2, 0x1, 0x0,
      0x30, 0xD, 0x6, 0x9, 0x2A, (byte) 0x86, 0x48, (byte) 0x86, (byte) 0xF7, 0xD, 0x1, 0x1, 0x1, 0x5, 0x0,
      0x4, (byte) 0x82, (byte) ((pkcs1Length >> 8) & 0xff), (byte) (pkcs1Length & 0xff)
    };

    // copy the header and the key bytes together
    var pkcs8bytes = new byte[pkcs8Header.length + pkcs1Bytes.length];
    System.arraycopy(pkcs8Header, 0, pkcs8bytes, 0, pkcs8Header.length);
    System.arraycopy(pkcs1Bytes, 0, pkcs8bytes, pkcs8Header.length, pkcs1Bytes.length);
    return pkcs8bytes;
  }
}
