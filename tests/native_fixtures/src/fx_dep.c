#include <dlfcn.h>
#include <stddef.h>

extern int add(int a, int b);
extern int fx_data_value;

int fx_dep_add(int a, int b)
{
    return add(a, b) + fx_data_value;
}

int fx_dep_dlsym_add(int a, int b)
{
    void *handle = dlopen("libfx_add.so", RTLD_NOW);
    int (*function)(int, int);
    int result;
    if (handle == NULL) {
        return -1;
    }
    function = (int (*)(int, int))dlsym(handle, "add");
    if (function == NULL) {
        dlclose(handle);
        return -2;
    }
    result = function(a, b);
    dlclose(handle);
    return result;
}
