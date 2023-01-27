package io.easybill.easydeploy.util;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import org.jetbrains.annotations.NotNull;

public final class SymlinkUtil {

  private SymlinkUtil() {
    throw new UnsupportedOperationException();
  }

  public static void createSymlink(@NotNull Path link, @NotNull Path target) throws IOException {
    // remove the current link (if it exists) and create the new one
    Files.deleteIfExists(link);
    Files.createSymbolicLink(link, target);
  }
}
