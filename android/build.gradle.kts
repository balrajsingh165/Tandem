// Root Gradle build script: declares AGP, Kotlin, Hilt, and protobuf plugin
// versions for the single-module Android build. No build logic lives here.

plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.kotlin.android) apply false
    alias(libs.plugins.kotlin.compose) apply false
    alias(libs.plugins.ksp) apply false
    alias(libs.plugins.hilt) apply false
    alias(libs.plugins.protobuf) apply false
}
