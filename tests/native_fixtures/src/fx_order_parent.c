#include <dlfcn.h>
#include <stddef.h>

extern void __fx_record(int value);
extern unsigned int fx_pool_word(int index);

int fx_nested_result = -9;
int fx_nested_close = -9;

__attribute__((constructor(101))) static void parent_first_constructor(void)
{
    __fx_record(3);
}

__attribute__((constructor(102))) static void parent_second_constructor(void)
{
    void *handle = dlopen("libfx_add.so", RTLD_NOW);
    int (*function)(int, int);
    __fx_record(4);
    if (handle == NULL) {
        fx_nested_result = -1;
        return;
    }
    function = (int (*)(int, int))dlsym(handle, "add");
    fx_nested_result = function == NULL ? -2 : function(2, 3);
    fx_nested_close = dlclose(handle);
    __fx_record(5);
}

__attribute__((destructor(101))) static void parent_first_destructor(void)
{
    __fx_record(-3);
}

__attribute__((destructor(102))) static void parent_second_destructor(void)
{
    __fx_record(-4);
}

unsigned int fx_parent_pool_word(void)
{
    return fx_pool_word(0);
}
