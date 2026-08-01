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
}

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2024.09.02")
    implementation(composeBom)
    androidTestImplementation(composeBom)

    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.activity:activity-compose:1.9.2")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-graphics")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.navigation:navigation-compose:2.8.0")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.5")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.8.5")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.5")
    implementation("androidx.documentfile:documentfile:1.0.1")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.8.1")
    implementation("androidx.datastore:datastore-preferences:1.1.1")

    debugImplementation("androidx.compose.ui:ui-tooling")
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.8.1")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test:runner:1.6.2")
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}

// Improved cargo-ndk build task (fixed for CI/testing reliability)
val cargoNdkBuild by tasks.registering(Exec::class) {
    workingDir = rootDir.parentFile.resolve("svg-converter-core")
    val jniLibsDir = file("${projectDir}/src/main/jniLibs")
    val abiDirs = listOf("arm64-v8a", "armeabi-v7a", "x86_64", "x86")

    onlyIf {
        val isCi = System.getenv("CI") == "true"
        val shouldRun = isCi || abiDirs.any { abi ->
            val abiDir = file("$jniLibsDir/$abi")
            !abiDir.exists() || abiDir.listFiles()?.isEmpty() == true
        }
        if (!shouldRun) {
            println("Skipping cargo-ndk build (libs exist)")
        }
        shouldRun
    }

    commandLine(
        "cargo", "ndk",
        "-t", "arm64-v8a",
        "-t", "armeabi-v7a",
        "-t", "x86_64",
        "-t", "x86",
        "-o", "${projectDir}/src/main/jniLibs",
        "build", "--release"
    )

    doLast {
        if (executionResult.get().exitValue != 0) {
            throw GradleException("cargo-ndk build failed. Check Rust/Android targets and NDK installation.")
        }
    }
}

tasks.named("preBuild") { dependsOn(cargoNdkBuild) }