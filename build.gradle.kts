import com.github.jengelman.gradle.plugins.shadow.tasks.ShadowJar

plugins {
  id("java")
  id("checkstyle")
  id("application")
  alias(libs.plugins.shadow)
}

group = "io.easybill"
version = "1.0-SNAPSHOT"

repositories {
  mavenCentral()
}

dependencies {
  // libs that are not needed in runtime
  compileOnly(libs.annotations)

  // visible libs
  implementation(libs.jgit)
  implementation(libs.guice)
  implementation(libs.slf4j)
  implementation(libs.dotenv)
  implementation(libs.jjwt.api)
  implementation(libs.jacksonToml)
  implementation(libs.bcprovJdk15)
  implementation(libs.githubClient)

  // runtime only libs
  runtimeOnly(libs.logback)
  runtimeOnly(libs.jjwt.impl)
  runtimeOnly(libs.jjwt.jackson)

  // testing libs
  testImplementation(libs.junit.api)
  testRuntimeOnly(libs.junit.engine)
}

application {
  mainClass.set("io.easybill.easydeploy.EasyDeploy")
}

tasks.withType<JavaCompile>().configureEach {
  sourceCompatibility = JavaVersion.VERSION_17.toString()
  targetCompatibility = JavaVersion.VERSION_17.toString()
  // options
  options.encoding = "UTF-8"
  options.isIncremental = true
}

tasks.withType<ShadowJar> {
  archiveFileName.set("easydep.jar")
}

tasks.withType<Checkstyle> {
  maxErrors = 0
  maxWarnings = 0
  configFile = rootProject.file("checkstyle.xml")
}

extensions.configure<CheckstyleExtension> {
  toolVersion = "10.6.0"
}

tasks.withType<Test>().configureEach {
  useJUnitPlatform()
}
