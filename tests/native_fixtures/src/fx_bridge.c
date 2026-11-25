#include <errno.h>
#include <pthread.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>
#include <unistd.h>

struct fx_s16 {
    long a;
    long b;
};

struct fx_s24 {
    long a;
    long b;
    long c;
};

struct fx_dpair {
    double x;
    double y;
};

struct fx_hfa4 {
    float v[4];
};

extern int fx_probe(int first, ...);
extern long fx_vprobe(int count, va_list list);
extern struct fx_s24 fx_get_s24(void);
extern struct fx_hfa4 fx_get_hfa4(void);
extern long double fx_get_ld(void);
extern struct fx_s16 fx_get_pair(void);
extern long fx_nest_host(void *function, long depth);
extern int fx_not_registered(int value);
extern int fx_sp_check(void);
extern void fx_touch_errno(int mode);
extern void fx_clobber_probe(void);

long fx_va_sum_int(int count, ...)
{
    va_list list;
    long sum = 0;
    va_start(list, count);
    for (int index = 0; index < count; index++) {
        sum += va_arg(list, int);
    }
    va_end(list);
    return sum;
}

long fx_va_sum_long(int count, ...)
{
    va_list list;
    long sum = 0;
    va_start(list, count);
    for (int index = 0; index < count; index++) {
        sum += va_arg(list, long);
    }
    va_end(list);
    return sum;
}

double fx_va_sum_dbl(int count, ...)
{
    va_list list;
    double sum = 0;
    va_start(list, count);
    for (int index = 0; index < count; index++) {
        sum += va_arg(list, double);
    }
    va_end(list);
    return sum;
}

long double fx_va_ld_last(int count, ...)
{
    va_list list;
    long double last = 0;
    va_start(list, count);
    for (int index = 0; index < count; index++) {
        last = va_arg(list, long double);
    }
    va_end(list);
    return last;
}

long double fx_va_mixed_ld(int integers, int reals, ...)
{
    va_list list;
    long double last = 0;
    va_start(list, reals);
    for (int index = 0; index < integers; index++) {
        long value = va_arg(list, long);
        if (value != index + 1) {
            return -1;
        }
    }
    for (int index = 0; index < reals; index++) {
        last = va_arg(list, long double);
    }
    va_end(list);
    return last;
}

long fx_va_s16(int count, ...)
{
    va_list list;
    long sum = 0;
    va_start(list, count);
    for (int index = 0; index < count; index++) {
        struct fx_s16 value = va_arg(list, struct fx_s16);
        sum += value.a * 10 + value.b;
    }
    va_end(list);
    return sum;
}

double fx_va_hfa(int count, ...)
{
    va_list list;
    double sum = 0;
    va_start(list, count);
    for (int index = 0; index < count; index++) {
        struct fx_dpair value = va_arg(list, struct fx_dpair);
        sum += value.x * 10 + value.y;
    }
    va_end(list);
    return sum;
}

long fx_va_s24(int count, ...)
{
    va_list list;
    long sum = 0;
    va_start(list, count);
    for (int index = 0; index < count; index++) {
        struct fx_s24 value = va_arg(list, struct fx_s24);
        sum += value.a * 100 + value.b * 10 + value.c;
    }
    va_end(list);
    return sum;
}

long fx_va_i128(int count, ...)
{
    va_list list;
    long sum = 0;
    va_start(list, count);
    for (int index = 0; index < count; index++) {
        __int128 value = va_arg(list, __int128);
        sum += (long)value * 10 + (long)(value >> 64);
    }
    va_end(list);
    return sum;
}

int fx_call_probe_mixed(void)
{
    return fx_probe(1, 2L, 3.0, 4, 5.0f, (long double)6, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113,
                    201.0, 202.0, 203.0, 204.0, 205.0, 206.0, 207.0, 208.0, 209.0, 210.0, (long double)7);
}

long fx_call_vprobe(int count, ...)
{
    va_list list;
    va_start(list, count);
    long first = fx_vprobe(count, list);
    long second = fx_vprobe(count, list);
    va_end(list);
    return first * 100000 + second;
}

long fx_call_get_s24(void)
{
    struct fx_s24 value = fx_get_s24();
    return value.a + value.b * 10 + value.c * 100;
}

float fx_call_get_hfa4(void)
{
    struct fx_hfa4 value = fx_get_hfa4();
    return value.v[0] + value.v[1] * 10 + value.v[2] * 100 + value.v[3] * 1000;
}

long double fx_call_get_ld(void)
{
    return fx_get_ld();
}

long fx_call_get_pair(void)
{
    struct fx_s16 value = fx_get_pair();
    return value.a * 1000 + value.b;
}

__attribute__((naked)) uintptr_t fx_sp_probe(void)
{
    __asm__ volatile("mov x0, sp\n\tret");
}

int fx_call_sp_import(void)
{
    return fx_sp_check();
}

long fx_callee_saved(long seed)
{
    long v0 = seed + 1, v1 = seed + 2, v2 = seed + 3, v3 = seed + 4, v4 = seed + 5;
    long v5 = seed + 6, v6 = seed + 7, v7 = seed + 8, v8 = seed + 9, v9 = seed + 10;
    double d0 = seed * 0.5, d1 = seed * 1.5, d2 = seed * 2.5, d3 = seed * 3.5;
    double d4 = seed * 4.5, d5 = seed * 5.5, d6 = seed * 6.5, d7 = seed * 7.5;
    fx_clobber_probe();
    long integers = v0 + v1 * 2 + v2 * 3 + v3 * 4 + v4 * 5 + v5 * 6 + v6 * 7 + v7 * 8 + v8 * 9 + v9 * 10;
    double reals = d0 + d1 + d2 + d3 + d4 + d5 + d6 + d7;
    return integers * 1000 + (long)reals;
}

long fx_nest(long depth)
{
    if (depth == 0) {
        return 0;
    }
    return 1 + fx_nest_host((void *)fx_nest, depth - 1);
}

int fx_call_missing(void)
{
    return fx_not_registered(7);
}

unsigned long fx_self_record(void)
{
    return (unsigned long)pthread_self();
}

int fx_tid(void)
{
    return gettid();
}

int fx_pid(void)
{
    return getpid();
}

int fx_errno_flow(void)
{
    errno = 5;
    fx_touch_errno(0);
    int unchanged = errno;
    fx_touch_errno(1);
    return unchanged * 1000 + errno;
}

unsigned long fx_canary_slot(void)
{
    unsigned long value;
    __asm__ volatile("mrs %0, tpidr_el0\n\tldr %0, [%0, #40]" : "=r"(value));
    return value;
}

__attribute__((noinline, optnone)) int fx_canary_overflow(unsigned count)
{
    char buffer[8];
    volatile char *cursor = buffer;
    for (unsigned index = 0; index < count; index++) {
        cursor[index] = 0x41;
    }
    return cursor[0];
}
