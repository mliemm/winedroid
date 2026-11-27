#include <jni.h>
#include <stdarg.h>
#include <stddef.h>
#include <string.h>

static jint registered_add(JNIEnv *env, jclass type, jint left, jint right)
{
    return left + right;
}

jint JNI_OnLoad(JavaVM *vm, void *reserved)
{
    JNIEnv *env = NULL;
    jclass type;
    JNINativeMethod methods[] = {
        {"registeredAdd", "(II)I", (void *)registered_add},
    };
    if ((*vm)->GetEnv(vm, (void **)&env, JNI_VERSION_1_6) != JNI_OK) {
        return JNI_ERR;
    }
    type = (*env)->FindClass(env, "fx/Native");
    if (type == NULL) {
        return JNI_ERR;
    }
    if ((*env)->RegisterNatives(env, type, methods, 1) != JNI_OK) {
        return JNI_ERR;
    }
    return JNI_VERSION_1_6;
}

jint Java_fx_Native_stringLength(JNIEnv *env, jclass type, jstring text)
{
    const char *chars = (*env)->GetStringUTFChars(env, text, NULL);
    jint length;
    if (chars == NULL) {
        return -1;
    }
    length = (jint)strlen(chars);
    (*env)->ReleaseStringUTFChars(env, text, chars);
    return length;
}

jstring Java_fx_Native_greeting(JNIEnv *env, jclass type)
{
    return (*env)->NewStringUTF(env, "fx_jni");
}

jint Java_fx_Native_callLength(JNIEnv *env, jclass type, jstring text)
{
    jclass string_class = (*env)->FindClass(env, "java/lang/String");
    jmethodID length_method;
    if (string_class == NULL) {
        return -1;
    }
    length_method = (*env)->GetMethodID(env, string_class, "length", "()I");
    if (length_method == NULL) {
        return -2;
    }
    return (*env)->CallIntMethod(env, text, length_method);
}

void Java_fx_Native_callPing(JNIEnv *env, jclass type)
{
    jmethodID ping = (*env)->GetStaticMethodID(env, type, "ping", "()V");
    if (ping == NULL) {
        return;
    }
    (*env)->CallStaticVoidMethod(env, type, ping);
}

jint Java_fx_Native_arrayLength(JNIEnv *env, jclass type, jarray array)
{
    return (*env)->GetArrayLength(env, array);
}

void Java_fx_Native_throwState(JNIEnv *env, jclass type)
{
    jclass exception = (*env)->FindClass(env, "java/lang/IllegalStateException");
    if (exception == NULL) {
        return;
    }
    (*env)->ThrowNew(env, exception, "fx_jni");
}

static jlong call_static_long(JNIEnv *env, jclass type, jmethodID method, ...)
{
    va_list arguments;
    jlong result;
    va_start(arguments, method);
    result = (*env)->CallStaticLongMethodV(env, type, method, arguments);
    va_end(arguments);
    return result;
}

jlong Java_fx_Native_callStaticLong(JNIEnv *env, jclass type, jlong value)
{
    jmethodID scale = (*env)->GetStaticMethodID(env, type, "scale", "(JI)J");
    if (scale == NULL) {
        return -1;
    }
    return call_static_long(env, type, scale, value, (jint)3);
}

static void call_static_void(JNIEnv *env, jclass type, jmethodID method, ...)
{
    va_list arguments;
    va_start(arguments, method);
    (*env)->CallStaticVoidMethodV(env, type, method, arguments);
    va_end(arguments);
}

void Java_fx_Native_callStaticVoid(JNIEnv *env, jclass type, jint value)
{
    jmethodID record = (*env)->GetStaticMethodID(env, type, "record", "(I)V");
    if (record == NULL) {
        return;
    }
    call_static_void(env, type, record, value + 1);
}
