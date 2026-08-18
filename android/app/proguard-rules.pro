# Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
# Keep native bridge symbols (JNI looks them up by name).
-keepclasseswithmembernames class * { native <methods>; }
-keep class com.watermelon.converter.jni.** { *; }
