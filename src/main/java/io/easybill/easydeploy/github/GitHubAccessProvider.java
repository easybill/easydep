package io.easybill.easydeploy.github;

import io.github.cdimascio.dotenv.Dotenv;
import java.io.IOException;
import java.security.KeyFactory;
import java.security.PrivateKey;
import java.security.spec.RSAPrivateKeySpec;
import java.util.Base64;
import java.util.Objects;
import java.util.regex.Pattern;
import org.bouncycastle.asn1.ASN1Sequence;
import org.bouncycastle.asn1.pkcs.RSAPrivateKey;
import org.jetbrains.annotations.NotNull;
import org.kohsuke.github.GHRepository;
import org.kohsuke.github.GitHubBuilder;
import org.kohsuke.github.authorization.AuthorizationProvider;
import org.kohsuke.github.authorization.OrgAppInstallationAuthorizationProvider;
import org.kohsuke.github.extras.authorization.JWTTokenProvider;

public final class GitHubAccessProvider {

  private static final String AUTH_TOKEN_PREFIX = "token ";
  private static final Pattern PKCS1_KEY_PATTERN =
    Pattern.compile("^--+BEGIN RSA PRIVATE KEY--+ (.*) --+.+END.*--+");

  private final GHRepository targetRepo;
  private final AuthorizationProvider appAuthProvider;

  private GitHubAccessProvider(@NotNull GHRepository targetRepo, @NotNull AuthorizationProvider appAuthProvider) {
    this.targetRepo = targetRepo;
    this.appAuthProvider = appAuthProvider;
  }

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
    var matcher = PKCS1_KEY_PATTERN.matcher(keyData);
    if (!matcher.matches()) {
      throw new IllegalArgumentException("Invalid pem key data supplied");
    }

    // decode the base64 encoded content
    var pkcs1Key = Base64.getMimeDecoder().decode(matcher.group(1));

    // parse the key in PKCS#1 format (which is the format that GitHub exports)
    // we could convert to PKCS#8 here, which java understands natively, but this solution
    // is much easier than adding the PKCS#8 headers manually
    var asn1Key = RSAPrivateKey.getInstance(ASN1Sequence.fromByteArray(pkcs1Key));
    var keySpec = new RSAPrivateKeySpec(asn1Key.getModulus(), asn1Key.getPrivateExponent());

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
