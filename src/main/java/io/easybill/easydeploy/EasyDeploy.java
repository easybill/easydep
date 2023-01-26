package io.easybill.easydeploy;

import com.google.inject.Guice;
import com.google.inject.Module;
import io.easybill.easydeploy.github.GitHubAccessProvider;
import io.easybill.easydeploy.github.GitHubReleasePollTask;
import io.github.cdimascio.dotenv.Dotenv;
import org.jetbrains.annotations.NotNull;

public final class EasyDeploy {

  public static void main(@NotNull String[] args) {
    // load the environment variables from an optional .env file
    var environment = Dotenv.configure().ignoreIfMissing().load();

    // construct the injector
    Module module = binder -> {
      binder.bind(Dotenv.class).toInstance(environment);
      binder.bind(GitHubAccessProvider.class).toInstance(GitHubAccessProvider.createFromEnv(environment));
    };
    var injector = Guice.createInjector(module);

    // start the poll task
    var pollTask = injector.getInstance(GitHubReleasePollTask.class);
    pollTask.scheduleBlocking();
  }
}
