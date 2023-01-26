package io.easybill.easydeploy.github;

import com.google.inject.Inject;
import com.google.inject.Singleton;
import io.easybill.easydeploy.release.ReleaseProcessor;
import io.github.cdimascio.dotenv.Dotenv;
import java.io.IOException;
import org.jetbrains.annotations.NotNull;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

@Singleton
public final class GitHubReleasePollTask {

  private static final Logger LOGGER = LoggerFactory.getLogger(GitHubReleasePollTask.class);

  private final long pullDelayMillis;
  private final ReleaseProcessor releaseProcessor;
  private final GitHubAccessProvider gitHubAccess;

  @Inject
  public GitHubReleasePollTask(
    @NotNull Dotenv env,
    @NotNull GitHubAccessProvider gitHubAccess,
    @NotNull ReleaseProcessor releaseProcessor
  ) {
    this.gitHubAccess = gitHubAccess;
    this.releaseProcessor = releaseProcessor;

    // get the pull delay millis from the env and ensure that it's not negative
    var delayMillis = env.get("EASYDEP_RELEASE_PULL_DELAY_MILLIS", "10000");
    this.pullDelayMillis = Math.max(100, Long.parseLong(delayMillis));
  }

  public void scheduleBlocking() {
    while (true) {
      try {
        // get the latest release & enqueue it if there is one
        var latestRelease = this.gitHubAccess.targetRepository().getLatestRelease();
        if (latestRelease != null) {
          this.releaseProcessor.enqueueRelease(latestRelease);
        }

        //noinspection BusyWait
        Thread.sleep(this.pullDelayMillis);
      } catch (IOException exception) {
        LOGGER.error("Unable to poll release from github", exception);
      } catch (InterruptedException exception) {
        Thread.currentThread().interrupt();
        break;
      }
    }
  }
}
