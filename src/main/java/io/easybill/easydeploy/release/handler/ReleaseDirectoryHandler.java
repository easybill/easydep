package io.easybill.easydeploy.release.handler;

import dev.derklaro.aerogel.Inject;
import dev.derklaro.aerogel.Singleton;
import io.github.cdimascio.dotenv.Dotenv;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Objects;
import org.jetbrains.annotations.NotNull;
import org.kohsuke.github.GHRelease;

@Singleton
public final class ReleaseDirectoryHandler {

  private static final String DEPLOYMENTS_DIR_NAME = "deployments";
  private static final String BASE_REPO_DIR_NAME = ".easydep_base_repo";

  private final Path deploymentsBaseDirectory;
  private final Path baseRepositoryDirectory;
  private final Path currentDeploymentDirectory;

  @Inject
  public ReleaseDirectoryHandler(@NotNull Dotenv env) throws IOException {
    // the deployment base is the directory in which all releases are located
    // the base repository directory is the directory in which the initial git clone is located
    var deployBaseDirectory = Objects.requireNonNull(env.get("EASYDEP_DEPLOY_BASE_DIRECTORY"));
    var deploymentBaseDirectory = Path.of(deployBaseDirectory).normalize().toAbsolutePath();
    this.baseRepositoryDirectory = deploymentBaseDirectory.resolve(BASE_REPO_DIR_NAME);

    // the current deployment directory is used as the symlink to the current release directory
    var currentDeploymentDirectory = env.get("EASYDEP_DEPLOY_LINK_DIRECTORY", "current");
    this.currentDeploymentDirectory = deploymentBaseDirectory
      .resolve(currentDeploymentDirectory)
      .normalize()
      .toAbsolutePath();

    // resolve the deployment base directory & create it if needed
    this.deploymentsBaseDirectory = deploymentBaseDirectory.resolve(DEPLOYMENTS_DIR_NAME);
    Files.createDirectories(this.deploymentsBaseDirectory);
  }

  public @NotNull Path deploymentsBaseDirectory() {
    return this.deploymentsBaseDirectory;
  }

  public @NotNull Path baseRepositoryDirectory() {
    return this.baseRepositoryDirectory;
  }

  public @NotNull Path currentDeploymentDirectory() {
    return this.currentDeploymentDirectory;
  }

  public @NotNull Path resolveDeploymentDirectory(@NotNull GHRelease release) {
    var directoryName = Long.toString(release.getId());
    return this.deploymentsBaseDirectory.resolve(directoryName);
  }
}
