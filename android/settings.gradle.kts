// Gradle settings for the Tandem Gateway build: single :app module,
// pluginManagement and dependencyResolutionManagement repositories.

pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "Tandem"

include(":app")
