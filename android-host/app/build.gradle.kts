plugins {
    id("com.android.application")
}

android {
    namespace = "org.onuron.mobile"
    compileSdk = 35

    defaultConfig {
        applicationId = "org.onuron.mobile"
        minSdk = 29
        targetSdk = 35
        versionCode = 1
        versionName = "1.0.0-onuron"

        ndk {
            abiFilters.addAll(listOf("arm64-v8a"))
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    // AndroidX & Material
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.core:core:1.15.0")
}
