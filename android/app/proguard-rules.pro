# R8/ProGuard keep rules: protobuf generated classes, Ktor reflection points,
# Hilt generated components. Keep minimal; prefer consumer rules from libraries.

-keep class com.tandem.gateway.proto.v1.** { *; }
-keepclassmembers class * extends com.google.protobuf.GeneratedMessageLite {
    <fields>;
}

-keepclassmembers class io.ktor.** { volatile <fields>; }
-dontwarn io.ktor.**
-dontwarn org.slf4j.**

-keep class dagger.hilt.internal.aggregatedroot.codegen.** { *; }

-keep class org.bouncycastle.** { *; }
-dontwarn org.bouncycastle.**
