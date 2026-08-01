plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
    id("com.google.devtools.ksp")
}

android {
    namespace = "com.watermelon.converter"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.watermelon.converter"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        vectorDrawables {
            useSupportLibrary = true
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    buildFeatures {
        compose = true
    }
    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
        }
    }

    // JNI native libs configuration (fixed placement inside android block)
    sourceSets {
        getByName("main").jniLibs.srcDirs("src/main/jniLibs")
    }

    // Per-ABI splits (Blueprint §1.5 / §8.3): each device downloads a
    // single-ABI APK instead of one fat APK bundling resvg across every
    // ABI. Release ships only the two real-device ABIs; x86/x86_64 are
    // emulator/debug-only per §8.1 and are excluded here (and not built
    // for release at all — see cargoNdkBuild below).
    splits {
        abi {
            isEnable = true
            reset()
            include("arm64-v8a", "armeabi-v7a")
            isUniversalApk = false
        }
    }
}

dependencies {
    // Bumped 2026-07-22. Verified via web search: Compose BOM
    // 2026.06.00 (Android Developers blog) and navigation-compose
    // 2.9.8 (developer.android.com navigation release notes).
    // core-ktx, activity-compose, lifecycle-*, documentfile, coroutines,
    // and datastore-preferences below are UNVERIFIED patch guesses —
    // I did not find explicit current-version confirmation for each.
    // Leave these on the BOM-managed Compose artifacts as-is where
    // possible, and confirm the rest via `./gradlew dependencyUpdates`
    // or Android Studio's upgrade assistant before merging.
    val composeBom = platform("androidx.compose:compose-bom:2026.06.00")
    implementation(composeBom)
    androidTestImplementation(composeBom)

    implementation("androidx.core:core-ktx:1.15.0") // unverified
    implementation("androidx.activity:activity-compose:1.10.1") // unverified
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-graphics")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.navigation:navigation-compose:2.9.8") // verified
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.9.0") // unverified
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.9.0") // unverified
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.9.0") // unverified
    implementation("androidx.documentfile:documentfile:1.1.0") // unverified
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0") // unverified
    implementation("androidx.datastore:datastore-preferences:1.1.2") // unverified

    debugImplementation("androidx.compose.ui:ui-tooling")
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.9.0")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test:runner:1.6.2")
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}

// cargo-ndk build tasks (fixed for CI/testing reliability).
//
// Release ABIs are arm64-v8a + armeabi-v7a only (Blueprint §8.1/§8.3:
// x86/x86_64 are emulator/debug-only and must not ship in release). Debug
// keeps all 4 ABIs so the emulator (x86_64) and instrumented tests still work.
val releaseAbis = listOf("arm64-v8a", "armeabi-v7a")
val debugAbis = listOf("arm64-v8a", "armeabi-v7a", "x86_64", "x86")

fun registerCargoNdkTask(name: String, abis: List<String>) =
    tasks.register(name, Exec::class) {
        workingDir = rootDir.parentFile.resolve("svg-converter-core")
        val jniLibsDir = file("${projectDir}/src/main/jniLibs")

        onlyIf {
            val isCi = System.getenv("CI") == "true"
            val shouldRun = isCi || abis.any { abi ->
                val abiDir = file("$jniLibsDir/$abi")
                !abiDir.exists() || abiDir.listFiles()?.isEmpty() == true
            }
            if (!shouldRun) {
                println("Skipping cargo-ndk build (libs exist)")
            }
            shouldRun
        }

        val args = mutableListOf("cargo", "ndk")
        abis.forEach { args.add("-t"); args.add(it) }
        args.addAll(listOf("-o", "${projectDir}/src/main/jniLibs", "build", "--release"))
        commandLine(args)

        doLast {
            if (executionResult.get().exitValue != 0) {
                throw GradleException("cargo-ndk build failed. Check Rust/Android targets and NDK installation.")
            }
        }
    }

val cargoNdkBuildRelease = registerCargoNdkTask("cargoNdkBuildRelease", releaseAbis)
val cargoNdkBuildDebug = registerCargoNdkTask("cargoNdkBuildDebug", debugAbis)

tasks.matching { it.name == "preBuild" }.configureEach {
    dependsOn(
        if (gradle.startParameter.taskNames.any { it.contains("Release", ignoreCase = true) }) {
            cargoNdkBuildRelease
        } else {
            cargoNdkBuildDebug
        }
    )
}