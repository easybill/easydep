package io.easybill.easydeploy.github;

import dev.derklaro.aerogel.Singleton;
import dev.derklaro.aerogel.auto.Factory;
import io.easybill.easydeploy.util.PKCS1PEMKey;
import io.github.cdimascio.dotenv.Dotenv;
import java.io.IOException;
import java.security.KeyFactory;
import java.security.PrivateKey;
import java.util.Objects;
import org.jetbrains.annotations.NotNull;
import org.kohsuke.github.GHRepository;
import org.kohsuke.github.GitHubBuilder;
import org.kohsuke.github.authorization.AuthorizationProvider;
import org.kohsuke.github.authorization.OrgAppInstallationAuthorizationProvider;
import org.kohsuke.github.extras.authorization.JWTTokenProvider;

@Singleton
public final class GitHubAccessProvider {

  private static final String AUTH_TOKEN_PREFIX = "token ";

  private final GHRepository targetRepo;
  private final AuthorizationProvider appAuthProvider;

  private GitHubAccessProvider(@NotNull GHRepository targetRepo, @NotNull AuthorizationProvider appAuthProvider) {
    this.targetRepo = targetRepo;
    this.appAuthProvider = appAuthProvider;
  }

  @Factory
  public static @NotNull GitHubAccessProvider createFromEnv(@NotNull Dotenv env) {
    try {
      // read the app configuration environment variables
      var appId = Objects.requireNonNull(env.get("EASYDEP_GITHUB_APP_ID"));
      var appKey = Objects.requireNonNull(env.get("EASYDEP_GITHUB_APP_PRIVATE_KEY"));

      // read the installation environment variables
      var githubRepoOrg = Objects.requireNonNull(env.get("EASYDEP_GITHUB_REPO_ORG"));
      var githubRepoName = Objects.requireNonNull(env.get("EASYDEP_GITHUB_REPO_NAME"));

      // create the jwt sign key & the auth providers
      var signKey = createJwtSignKey(appKey);
      var jwtAuthProvider = new JWTTokenProvider(appId, signKey);
      var orgAppAuthProvider = new OrgAppInstallationAuthorizationProvider(githubRepoOrg, jwtAuthProvider);

      // construct the client & fetch the target repository
      var client = new GitHubBuilder().withAuthorizationProvider(orgAppAuthProvider).build();
      var targetRepository = client.getRepository("%s/%s".formatted(githubRepoOrg, githubRepoName));

      // wrap the information
      return new GitHubAccessProvider(targetRepository, orgAppAuthProvider);
    } catch (Exception exception) {
      throw new IllegalStateException("Unable to construct github client:", exception);
    }
  }

  private static @NotNull PrivateKey createJwtSignKey(@NotNull String keyData) throws Exception {
    // load the key spec from the given key data
    var keySpec = PKCS1PEMKey.fromInputOrFile(keyData);

    // load the key and generate the private version of it
    var keyFactory = KeyFactory.getInstance("RSA");
    return keyFactory.generatePrivate(keySpec);
  }

  public @NotNull GHRepository targetRepository() {
    return this.targetRepo;
  }

  public @NotNull String accessToken() throws IOException {
    var encodedAuthToken = this.appAuthProvider.getEncodedAuthorization();
    return encodedAuthToken.replaceFirst(AUTH_TOKEN_PREFIX, "");
  }
}
