package io.easybill.easydeploy.release;

import com.google.inject.Inject;
import com.google.inject.Singleton;
import io.github.cdimascio.dotenv.Dotenv;
import java.nio.file.Path;
import java.util.Objects;
import org.jetbrains.annotations.NotNull;
import org.kohsuke.github.GHRelease;

@Singleton
public final class ReleaseDirectoryHandler {

  private static final String BASE_REPO_DIR_NAME = ".easydep_base_repo";

  private final Path deploymentBaseDirectory;
  private final Path baseRepositoryDirectory;
  private final Path currentDeploymentDirectory;

  @Inject
  public ReleaseDirectoryHandler(@NotNull Dotenv env) {
    // the deployment base is the directory in which all releases are located
    // the base repository directory is the directory in which the initial git clone is located
    var deployBaseDirectory = Objects.requireNonNull(env.get("EASYDEP_DEPLOY_BASE_DIRECTORY"));
    this.deploymentBaseDirectory = Path.of(deployBaseDirectory).normalize().toAbsolutePath();
    this.baseRepositoryDirectory = this.deploymentBaseDirectory.resolve(BASE_REPO_DIR_NAME);

    // the current deployment directory is used as the symlink to the current release directory
    var currentDeploymentDirectory = env.get("EASYDEP_DEPLOY_CURRENT_DIRECTORY", "current");
    this.currentDeploymentDirectory = Path.of(currentDeploymentDirectory).normalize().toAbsolutePath();
  }

  public @NotNull Path baseRepositoryDirectory() {
    return this.baseRepositoryDirectory;
  }

  public @NotNull Path currentDeploymentDirectory() {
    return this.currentDeploymentDirectory;
  }

  public @NotNull Path resolveDeploymentDirectory(@NotNull GHRelease release) {
    var directoryName = Long.toString(release.getId());
    return this.deploymentBaseDirectory.resolve(directoryName);
  }
}
