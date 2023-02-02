package io.easybill.easydeploy;

import dev.derklaro.aerogel.Injector;
import dev.derklaro.aerogel.auto.runtime.AutoAnnotationRegistry;
import dev.derklaro.aerogel.binding.BindingBuilder;
import io.easybill.easydeploy.github.GitHubReleasePollTask;
import io.github.cdimascio.dotenv.Dotenv;
import org.jetbrains.annotations.NotNull;

public final class EasyDeploy {

  public static void main(@NotNull String[] args) {
    // load the environment variables from an optional .env file
    var environment = Dotenv.configure().ignoreIfMissing().load();

    // build the injector & install the dotenv instance
    var injector = Injector.newInjector();
    injector.install(BindingBuilder.create().bind(Dotenv.class).toInstance(environment));

    // install the autoconfigure bindings to the injector
    var autoRegistry = AutoAnnotationRegistry.newRegistry();
    autoRegistry.installBindings(EasyDeploy.class.getClassLoader(), "auto-config.aero", injector);

    // start the poll task
    var pollTask = injector.instance(GitHubReleasePollTask.class);
    pollTask.scheduleBlocking();
  }
}
