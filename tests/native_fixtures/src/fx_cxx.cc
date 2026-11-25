// cite: bionic libc.map.txt#_Unwind_Resume

struct E {
    int v;
};

extern "C" void *__cxa_allocate_exception(unsigned long size);
extern "C" void __cxa_throw(void *object, void *type, void (*destructor)(void *)) __attribute__((noreturn));
extern "C" void *__cxa_begin_catch(void *object);
extern "C" void __cxa_end_catch(void);

extern "C" __attribute__((noinline)) int fx_cxx_mid(int (*function)(int), int value)
{
    return function(value);
}

static int fx_cxx_raise_value(int value)
{
    E error;
    error.v = value;
    throw error;
}

extern "C" int fx_cxx_throw_and_catch(int value)
{
    try {
        return fx_cxx_mid(fx_cxx_raise_value, value) + 1000;
    } catch (E &caught) {
        return caught.v + 1;
    }
}

extern "C" __attribute__((noinline)) int fx_cxx_plain_add(int left, int right)
{
    return left + right;
}
