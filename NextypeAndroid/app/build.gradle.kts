import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

val keystorePropertiesFile = rootProject.file("keystore.properties")
val keystoreProperties = Properties()
if (keystorePropertiesFile.exists()) {
    keystorePropertiesFile.inputStream().use { keystoreProperties.load(it) }
}

// Signing: 环境变量优先，keystore.properties 兜底
val resolvedStoreFile: String? = System.getenv("NEXTYPE_STORE_FILE") ?: keystoreProperties["storeFile"] as String?
val resolvedStorePassword: String? = System.getenv("NEXTYPE_STORE_PASSWORD") ?: keystoreProperties["storePassword"] as String?
val resolvedKeyAlias: String? = System.getenv("NEXTYPE_KEY_ALIAS") ?: keystoreProperties["keyAlias"] as String?
val resolvedKeyPassword: String? = System.getenv("NEXTYPE_KEY_PASSWORD") ?: keystoreProperties["keyPassword"] as String?

android {
    namespace = "com.nextype.android"
    compileSdk = 34

    buildFeatures {
        buildConfig = true
    }

    signingConfigs {
        if (resolvedStoreFile != null && resolvedStorePassword != null &&
            resolvedKeyAlias != null && resolvedKeyPassword != null) {
            create("release") {
                storeFile = file(resolvedStoreFile)
                storePassword = resolvedStorePassword
                keyAlias = resolvedKeyAlias
                keyPassword = resolvedKeyPassword
            }
        }
    }

    defaultConfig {
        applicationId = "com.nextype.app"
        minSdk = 24
        targetSdk = 34
        versionCode = 1
        versionName = "1.0.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        buildConfigField("String", "RELAY_URL", "\"wss://nextypeapi.yuanfengai.cn:8443\"")
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            if (resolvedStoreFile != null && signingConfigs.findByName("release") != null) {
                signingConfig = signingConfigs.getByName("release")
            }
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.12.0")
    implementation("androidx.appcompat:appcompat:1.6.1")
    implementation("com.google.android.material:material:1.11.0")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.6.1")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.7.3")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("androidx.security:security-crypto:1.1.0-alpha06")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.5")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.1")
}
