// The frus plugin for Android Studio and the other IDEs of the IntelliJ platform (IntelliJ IDEA,
// RustRover, CLion...). It depends on the platform alone, so it installs in any of them.
//
//   ./gradlew test          the unit tests
//   ./gradlew buildPlugin   build/distributions/frus-intellij-<version>.zip, to install from disk
//   ./gradlew runIde        an Android Studio with the plugin, in a sandbox of its own
//   ./gradlew verifyPlugin  checks it against the IDEs it claims to support
//
// Add `-PlocalIde="C:/Program Files/Android/Android Studio"` to build against an installed IDE
// instead of downloading one.

import org.jetbrains.intellij.platform.gradle.TestFrameworkType
import org.jetbrains.intellij.platform.gradle.tasks.VerifyPluginTask

plugins {
    id("org.jetbrains.kotlin.jvm") version "2.2.21"
    id("org.jetbrains.intellij.platform") version "2.20.0"
}

group = "dev.frus"
version = providers.gradleProperty("pluginVersion").get()

kotlin {
    jvmToolchain(21)
}

repositories {
    mavenCentral()
    intellijPlatform {
        defaultRepositories()
    }
}

dependencies {
    intellijPlatform {
        val local = providers.gradleProperty("localIde").orNull
        if (local != null) {
            local(local)
        } else {
            androidStudio(providers.gradleProperty("androidStudioVersion"))
        }
        testFramework(TestFrameworkType.Platform)
        pluginVerifier()
    }
    testImplementation("junit:junit:4.13.2")
    // The platform's test framework reads JUnit 5's API on its classpath, used or not.
    testImplementation("org.opentest4j:opentest4j:1.3.0")
}

intellijPlatform {
    pluginConfiguration {
        version = providers.gradleProperty("pluginVersion")
        ideaVersion {
            sinceBuild = providers.gradleProperty("pluginSinceBuild")
            untilBuild = provider { null }
        }
    }
    pluginVerification {
        ides {
            recommended()
        }
        failureLevel = listOf(
            VerifyPluginTask.FailureLevel.COMPATIBILITY_PROBLEMS,
            VerifyPluginTask.FailureLevel.INVALID_PLUGIN,
        )
    }
    publishing {
        token = providers.environmentVariable("JETBRAINS_MARKETPLACE_TOKEN")
    }
}
