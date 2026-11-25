#include <dlfcn.h>
#include <link.h>
#include <stddef.h>
#include <string.h>

static int counted;
static int stop_after;

static int count_module(struct dl_phdr_info *info, size_t size, void *data)
{
    int *limit = (int *)data;
    if (size != sizeof(struct dl_phdr_info)) {
        return -100;
    }
    counted++;
    if (*limit != 0 && counted >= *limit) {
        return 5;
    }
    return 0;
}

int fx_dl_open_call_close(int a, int b)
{
    void *handle = dlopen("libfx_add.so", RTLD_NOW);
    int (*function)(int, int);
    Dl_info info;
    int result;
    if (handle == NULL) {
        return -1;
    }
    function = (int (*)(int, int))dlsym(handle, "add");
    if (function == NULL) {
        return -2;
    }
    if (dladdr((void *)function, &info) == 0) {
        return -3;
    }
    if (info.dli_sname == NULL || strcmp(info.dli_sname, "add") != 0) {
        return -4;
    }
    if (info.dli_fname == NULL || strstr(info.dli_fname, "libfx_add.so") == NULL) {
        return -5;
    }
    result = function(a, b);
    if (dlclose(handle) != 0) {
        return -6;
    }
    return result;
}

int fx_dl_missing_library(char *message)
{
    void *handle = dlopen("libnope.so", RTLD_NOW);
    const char *first;
    const char *second;
    if (handle != NULL) {
        return -1;
    }
    first = dlerror();
    second = dlerror();
    if (first == NULL) {
        return -2;
    }
    strcpy(message, first);
    return second == NULL ? 0 : -3;
}

int fx_dl_invalid_flags(char *message)
{
    void *handle = dlopen("libfx_add.so", 0x40000000);
    const char *text;
    if (handle != NULL) {
        return -1;
    }
    text = dlerror();
    if (text == NULL) {
        return -2;
    }
    strcpy(message, text);
    return 0;
}

int fx_dl_missing_symbol(char *message)
{
    void *handle = dlopen("libfx_add.so", RTLD_NOW);
    void *symbol;
    const char *text;
    if (handle == NULL) {
        return -1;
    }
    symbol = dlsym(handle, "no_such_symbol");
    text = dlerror();
    dlclose(handle);
    if (symbol != NULL || text == NULL) {
        return -2;
    }
    strcpy(message, text);
    return 0;
}

int fx_dl_bad_handle(char *message)
{
    void *symbol = dlsym((void *)0x1234, "add");
    const char *text = dlerror();
    if (symbol != NULL || text == NULL) {
        return -1;
    }
    strcpy(message, text);
    return dlclose((void *)0x1234);
}

void *fx_dl_system_symbol(const char *library, const char *name)
{
    void *handle = dlopen(library, RTLD_NOW);
    if (handle == NULL) {
        return NULL;
    }
    return dlsym(handle, name);
}

void *fx_dl_default_symbol(const char *name)
{
    return dlsym(RTLD_DEFAULT, name);
}

int fx_dl_iterate_count(int limit)
{
    counted = 0;
    stop_after = dl_iterate_phdr(count_module, &limit);
    return stop_after == 0 ? counted : stop_after;
}

int fx_dl_counted(void)
{
    return counted;
}
