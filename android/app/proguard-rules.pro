# Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
# Keep native bridge symbols (JNI looks them up by name).
-keepclasseswithmembernames class * { native <methods>; }
-keep class com.watermelon.converter.jni.** { *; }
# ProgressCallback implementations live in viewmodel/* classes, outside the
# package kept above. jni.rs's call_method("onProgress", ...) swallows JNI
# failures silently, so a stripped/renamed override would degrade to "the
# progress bar just stops updating" instead of a visible crash — keep it.
-keep class * implements com.watermelon.converter.jni.ProgressCallback { *; }
