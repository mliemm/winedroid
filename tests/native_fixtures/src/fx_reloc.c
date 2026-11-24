#include <pthread.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

extern int __android_log_print(int priority, const char *tag, const char *format, ...);
extern int __cxa_atexit(void (*function)(void *), void *argument, void *dso_handle);

#define FX_LOG_INFO 4

__attribute__((visibility("hidden"))) void *__dso_handle = &__dso_handle;

static int local_target = 11;
static pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;
static int ctor_marker;
static int atexit_marker;

int fx_exported_value = 7;
int fx_fini_marker;
int *fx_local_pointer = &local_target;
int *fx_exported_pointer = &fx_exported_value;
void *fx_import_table[] = {strlen, memcpy};
__thread int fx_tls_counter = 5;
__thread int fx_tls_zero;

static void atexit_handler(void *argument)
{
    atexit_marker = *(int *)argument;
}

__attribute__((constructor)) static void fx_constructor(void)
{
    ctor_marker = 0xC0DE;
    __cxa_atexit(atexit_handler, &fx_exported_value, &__dso_handle);
}

__attribute__((destructor)) static void fx_destructor(void)
{
    fx_fini_marker = 0xDEAD;
}

int fx_ctor_marker(void)
{
    return ctor_marker;
}

int fx_atexit_marker(void)
{
    return atexit_marker;
}

size_t fx_strlen(const char *text)
{
    return strlen(text);
}

void *fx_strlen_address(void)
{
    return (void *)strlen;
}

int fx_copy_sum(const uint8_t *source, size_t count)
{
    uint8_t *copy = malloc(count);
    int sum = 0;
    if (copy == NULL) {
        return -1;
    }
    memcpy(copy, source, count);
    for (size_t index = 0; index < count; index++) {
        sum += copy[index];
    }
    free(copy);
    return sum;
}

int fx_format(char *buffer, size_t capacity, int number, const char *text, double real)
{
    return snprintf(buffer, capacity, "%d %s %f", number, text, real);
}

int fx_lock_roundtrip(void)
{
    int result = pthread_mutex_lock(&lock);
    if (result != 0) {
        return result;
    }
    return pthread_mutex_unlock(&lock);
}

int fx_log(const char *message)
{
    return __android_log_print(FX_LOG_INFO, "fx_reloc", "%s %d", message, 7);
}

int fx_tls_next(void)
{
    fx_tls_zero += 1;
    return fx_tls_counter++ + fx_tls_zero;
}

int fx_guarded_sum(const char *text)
{
    char local[32];
    size_t length = strlen(text);
    int sum = 0;
    if (length >= sizeof(local)) {
        return -1;
    }
    memcpy(local, text, length + 1);
    for (size_t index = 0; index < length; index++) {
        sum += local[index];
    }
    return sum;
}
