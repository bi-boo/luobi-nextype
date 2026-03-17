# OkHttp WebSocket
-dontwarn okhttp3.**
-keep class okhttp3.** { *; }
-keep interface okhttp3.** { *; }

# JSON data classes（保留 Gson/JSONObject 序列化字段）
-keep class com.nextype.android.PairedDevice { *; }
-keep class com.nextype.android.PairingResponse { *; }
-keep class com.nextype.android.TrustDeviceInfo { *; }
-keep class com.nextype.android.OnlineDeviceInfo { *; }

# Release 包中移除调试日志（保留 Log.e）
-assumenosideeffects class android.util.Log {
    public static *** d(...);
    public static *** v(...);
    public static *** i(...);
    public static *** w(...);
}
