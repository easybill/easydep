package io.easybill.easydeploy.release.task;

import io.easybill.easydeploy.task.ChainedTask;
import io.easybill.easydeploy.task.TaskExecutionContext;
import java.nio.file.Path;
import org.apache.commons.lang3.tuple.Pair;
import org.eclipse.jgit.api.Git;
import org.eclipse.jgit.api.ResetCommand;
import org.eclipse.jgit.transport.TagOpt;
import org.jetbrains.annotations.NotNull;
import org.kohsuke.github.GHRelease;

public final class GitPrepareTask extends ChainedTask<Pair<GHRelease, Path>> {

  public GitPrepareTask() {
    super("Deployment Directory Git Pull");
  }

  @Override
  protected @NotNull Object internalExecute(
    @NotNull TaskExecutionContext<?, ?> context,
    @NotNull Pair<GHRelease, Path> input
  ) throws Exception {
    try (var gitRepo = Git.open(input.getRight().toFile())) {
      // fetch the repository
      gitRepo.fetch().setRemoveDeletedRefs(true).setTagOpt(TagOpt.FETCH_TAGS).call();

      // reset the repository to the associated tag
      gitRepo.reset().setMode(ResetCommand.ResetType.HARD).setRef(input.getLeft().getTagName()).call();
    }

    // just pass the input through to the next task
    return input;
  }
}
