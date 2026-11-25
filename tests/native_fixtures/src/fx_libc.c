#include <errno.h>
#include <malloc.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/select.h>
#include <sys/types.h>
#include <time.h>
#include <wchar.h>
#include <unistd.h>

extern int __android_log_print(int priority, const char *tag, const char *format, ...);
extern int __android_log_write(int priority, const char *tag, const char *text);
extern int __android_log_buf_write(int buffer, int priority, const char *tag, const char *text);
extern int __android_log_is_loggable(int priority, const char *tag, int default_priority);
extern int __android_log_set_minimum_priority(int priority);
extern int __android_log_get_minimum_priority(void);
extern void __android_log_set_default_tag(const char *tag);
extern int __system_property_get(const char *name, char *value);
extern const void *__system_property_find(const char *name);
extern int __system_property_read(const void *info, char *name, char *value);
extern unsigned int __system_property_serial(const void *info);
extern size_t __strlen_chk(const char *string, size_t size);
extern void *__memcpy_chk(void *destination, const void *source, size_t count, size_t size);
extern void *__memset_chk(void *destination, int value, size_t count, size_t size);
extern char *__strcpy_chk(char *destination, const char *source, size_t size);
extern char *__strcat_chk(char *destination, const char *source, size_t size);
extern char *__strncpy_chk2(char *destination, const char *source, size_t count, size_t size, size_t source_size);
extern int __vsnprintf_chk(char *destination, size_t supplied, int flags, size_t size, const char *format, va_list list);
extern int __snprintf_chk(char *destination, size_t supplied, int flags, size_t size, const char *format, ...);
extern int __vsprintf_chk(char *destination, int flags, size_t size, const char *format, va_list list);
extern ssize_t __read_chk(int fd, void *buffer, size_t count, size_t size);
extern char *__strchr_chk(const char *string, int character, size_t size);
extern void __assert2(const char *file, int line, const char *function, const char *expression);
extern void android_set_abort_message(const char *message);
extern void __android_log_assert(const char *condition, const char *tag, const char *format, ...);
extern void __android_log_set_logger(void (*logger)(const void *message));
extern int tolower(int character);
extern int toupper(int character);
extern int isalnum(int character);
extern int ispunct(int character);
extern int isspace(int character);
extern int isxdigit(int character);
extern int isalpha(int character);
extern int isdigit(int character);
extern const char *_ctype_;
extern char *setlocale(int category, const char *name);
extern void *newlocale(int mask, const char *name, void *base);
extern void *uselocale(void *locale);
extern void freelocale(void *locale);
extern size_t __ctype_get_mb_cur_max(void);
extern void *localeconv(void);

int fx_snprintf_mix(char *buffer, size_t size)
{
    return snprintf(buffer, size, "%d %5.2f %s %c %x %lu %%|%-4d|%04d|%+d|%#o|%#X|%.3s|%10.4s|", -42, 3.14159, "str", 'z', 255u,
                    18446744073709551615ul, 7, 42, 5, 8, 255, "abcdef", "abcdef");
}

int fx_snprintf_floats(char *buffer, size_t size)
{
    return snprintf(buffer, size, "%e|%g|%G|%.0f|%10.3e|%a", 12345.678, 0.0001, 1e-10, 2.5, -0.000123, 1.0);
}

int fx_snprintf_int_precision(char *buffer, size_t size)
{
    return snprintf(buffer, size, "%.5d|%8.3d|%.0d|%-8.3x|%08.3d|%.0d", 42, 7, 0, 255, 5, 3);
}

int fx_snprintf_truncated(char *buffer)
{
    return snprintf(buffer, 4, "abcdefgh");
}

int fx_snprintf_measure(void)
{
    return snprintf(NULL, 0, "%s-%d", "measure", 12345);
}

int fx_sprintf_pointers(char *buffer)
{
    return sprintf(buffer, "%s|%p|%p", (const char *)0, (void *)0, (void *)0x1234);
}

int fx_vsnprintf_wrap(char *buffer, size_t size, const char *format, ...)
{
    va_list list;
    int result;
    va_start(list, format);
    result = vsnprintf(buffer, size, format, list);
    va_end(list);
    return result;
}

int fx_vsprintf_wrap(char *buffer, const char *format, ...)
{
    va_list list;
    int result;
    va_start(list, format);
    result = vsprintf(buffer, format, list);
    va_end(list);
    return result;
}

int fx_snprintf_chk_ok(char *buffer)
{
    return __snprintf_chk(buffer, 16, 0, 16, "%d:%s", 5, "ok");
}

int fx_snprintf_chk_overflow(char *buffer)
{
    return __snprintf_chk(buffer, 32, 0, 16, "%d:%s", 5, "overflow");
}

int fx_vsprintf_chk_overflow(char *buffer, const char *format, ...)
{
    va_list list;
    int result;
    va_start(list, format);
    result = __vsprintf_chk(buffer, 0, 4, format, list);
    va_end(list);
    return result;
}

long fx_strtol(const char *text, int base, long *consumed, int *error)
{
    char *end = NULL;
    long value;
    errno = 0;
    value = strtol(text, &end, base);
    *consumed = end - text;
    *error = errno;
    return value;
}

unsigned long fx_strtoul(const char *text, int base, long *consumed, int *error)
{
    char *end = NULL;
    unsigned long value;
    errno = 0;
    value = strtoul(text, &end, base);
    *consumed = end - text;
    *error = errno;
    return value;
}

double fx_strtod(const char *text, long *consumed, int *error)
{
    char *end = NULL;
    double value;
    errno = 0;
    value = strtod(text, &end);
    *consumed = end - text;
    *error = errno;
    return value;
}

int fx_atoi(const char *text)
{
    return atoi(text);
}

static int comparisons[512][2];
static int comparison_count;

static int compare_ints(const void *left, const void *right)
{
    int a = *(const int *)left;
    int b = *(const int *)right;
    if (comparison_count < 512) {
        comparisons[comparison_count][0] = a;
        comparisons[comparison_count][1] = b;
    }
    comparison_count++;
    return a < b ? -1 : (a > b ? 1 : 0);
}

int fx_qsort_ints(int *values, size_t count)
{
    comparison_count = 0;
    qsort(values, count, sizeof(int), compare_ints);
    return comparison_count;
}

int fx_comparison_value(int index, int side)
{
    return comparisons[index][side];
}

int *fx_bsearch_ints(int key, int *values, size_t count)
{
    return (int *)bsearch(&key, values, count, sizeof(int), compare_ints);
}

size_t fx_strlen(const char *text)
{
    return strlen(text);
}

int fx_string_ops(char *buffer)
{
    char copy[32];
    strcpy(copy, "Hello, World");
    if (strcasecmp(copy, "hello, WORLD") != 0) {
        return 1;
    }
    if (strncasecmp(copy, "HELLX", 4) != 0) {
        return 2;
    }
    if (strchr(copy, 'W') != copy + 7) {
        return 3;
    }
    if (memrchr(copy, 'o', 12) != copy + 8) {
        return 4;
    }
    if (strstr(copy, "World") != copy + 7) {
        return 5;
    }
    if (strcasestr(copy, "WORLD") != copy + 7) {
        return 6;
    }
    if (strlcpy(buffer, copy, 6) != 12 || strcmp(buffer, "Hello") != 0) {
        return 7;
    }
    if (strlcat(buffer, "!!!!!", 8) != 10 || strcmp(buffer, "Hello!!") != 0) {
        return 8;
    }
    if (strxfrm(buffer, "abc", 2) != 3 || strcmp(buffer, "a") != 0) {
        return 9;
    }
    return 0;
}

int fx_strtok_words(char *text, char *out)
{
    int count = 0;
    char *saved = NULL;
    char *word = strtok_r(text, " ,", &saved);
    out[0] = 0;
    while (word != NULL) {
        strcat(out, word);
        strcat(out, "|");
        count++;
        word = strtok_r(NULL, " ,", &saved);
    }
    return count;
}

const char *fx_strerror(int number)
{
    return strerror(number);
}

int fx_strerror_r(int number, char *buffer, size_t size)
{
    return strerror_r(number, buffer, size);
}

int fx_errno_after_strtol_overflow(void)
{
    errno = 0;
    strtol("99999999999999999999", NULL, 10);
    return errno;
}

int fx_errno_after_read(int fd)
{
    char byte;
    errno = 0;
    if (__read_chk(fd, &byte, 1, 1) != -1) {
        return -1;
    }
    return errno;
}

int fx_realloc_zero_returns_null(void)
{
    void *block = malloc(16);
    void *result = realloc(block, 0);
    return result == NULL;
}

int fx_calloc_overflow(void)
{
    void *block;
    errno = 0;
    block = calloc((size_t)1 << 62, 16);
    return block == NULL ? errno : 0;
}

int fx_memalign_usable(size_t alignment, size_t size)
{
    void *block = memalign(alignment, size);
    int ok = block != NULL && ((uintptr_t)block % alignment) == 0 && malloc_usable_size(block) >= size;
    free(block);
    return ok;
}

int fx_reallocarray_overflow(void)
{
    errno = 0;
    return reallocarray(NULL, (size_t)1 << 62, 16) == NULL ? errno : 0;
}

int fx_reallocarray_zero_returns_null(size_t count, size_t size)
{
    void *block = malloc(16);
    return reallocarray(block, count, size) == NULL;
}

size_t fx_mallinfo_in_use_growth(void)
{
    struct mallinfo before = mallinfo();
    void *block = malloc(1 << 20);
    struct mallinfo after = mallinfo();
    memset(block, 1, 1 << 20);
    free(block);
    return after.uordblks - before.uordblks;
}

int fx_mallopt(int option, int value)
{
    return mallopt(option, value);
}

int fx_getenv_length(const char *name)
{
    const char *value = getenv(name);
    return value == NULL ? -1 : (int)strlen(value);
}

int fx_setenv(const char *name, const char *value, int overwrite)
{
    return setenv(name, value, overwrite);
}

int fx_unsetenv(const char *name)
{
    return unsetenv(name);
}

int fx_putenv_identity(char *entry, const char *name)
{
    putenv(entry);
    return getenv(name) == entry + strlen(name) + 1;
}

int fx_rand_sequence(unsigned seed, int *values, int count)
{
    srand(seed);
    for (int index = 0; index < count; index++) {
        values[index] = rand();
    }
    return count;
}

long fx_lrand48_sequence(long seed, long *values, int count)
{
    srand48(seed);
    for (int index = 0; index < count; index++) {
        values[index] = lrand48();
    }
    return count;
}

int fx_log_all(void)
{
    int total = 0;
    total += __android_log_print(4, "fxlog", "%d %s %5.2f", 7, "x", 2.5);
    total += __android_log_write(5, "fxlog", "plain");
    total += __android_log_buf_write(0, 6, "fxlog", "main buffer");
    return total;
}

int fx_log_buffer(int buffer)
{
    return __android_log_buf_write(buffer, 4, "fxbuf", "buffered");
}

int fx_log_null_tag(void)
{
    return __android_log_write(4, NULL, "no tag");
}

int fx_log_set_default_tag(void)
{
    int result;
    __android_log_set_default_tag("zz");
    result = __android_log_write(4, NULL, "default tag");
    __android_log_set_default_tag(NULL);
    return result;
}

struct fx_log_message {
    size_t struct_size;
    int buffer_id;
    int priority;
    const char *tag;
    const char *file;
    unsigned line;
    const char *message;
};

static size_t logger_size;
static int logger_priority;
static char logger_text[96];

static void guest_logger(const void *raw)
{
    const struct fx_log_message *message = raw;
    logger_size = message->struct_size;
    logger_priority = message->priority;
    snprintf(logger_text, sizeof(logger_text), "%s|%s", message->tag, message->message);
}

int fx_log_with_guest_logger(char *report)
{
    __android_log_set_logger(guest_logger);
    __android_log_write(5, "guest", "routed");
    __android_log_set_logger(NULL);
    strcpy(report, logger_text);
    return (int)(logger_size * 1000 + logger_priority);
}

void fx_log_assert_format(void)
{
    __android_log_assert("x == 1", "fxassert", "boom %d", 5);
}

void fx_log_assert_condition(void)
{
    __android_log_assert("x == 1", "fxassert", NULL);
}

void fx_log_assert_nothing(void)
{
    __android_log_assert(NULL, "fxassert", NULL);
}

int fx_log_minimum(int priority, int logged_priority)
{
    int previous = __android_log_set_minimum_priority(priority);
    int result = __android_log_write(logged_priority, "fxmin", "minimum");
    __android_log_set_minimum_priority(previous);
    return result;
}

int fx_log_long_message(void)
{
    static char text[2200];
    memset(text, 'a', sizeof(text) - 1);
    text[sizeof(text) - 1] = 0;
    return __android_log_print(4, "fxlong", "%s", text);
}

int fx_log_errno_preserved(void)
{
    errno = 77;
    __android_log_print(4, "fxerr", "value %d", 1);
    return errno;
}

int fx_log_fatal(void)
{
    return __android_log_print(7, "fxfatal", "fatal %d", 1);
}

int fx_property_get(const char *name, char *value)
{
    return __system_property_get(name, value);
}

const void *fx_property_find_address(const char *name)
{
    return __system_property_find(name);
}

int fx_property_info(const char *name, char *read_name, char *read_value, unsigned *serial)
{
    const void *info = __system_property_find(name);
    if (info == NULL) {
        return -1;
    }
    *serial = __system_property_serial(info);
    return __system_property_read(info, read_name, read_value);
}

long fx_sysconf(int name)
{
    errno = 0;
    return sysconf(name);
}

int fx_getpagesize(void)
{
    return getpagesize();
}

int fx_clock_gettime(int clock, long *seconds_and_nanoseconds)
{
    struct timespec value;
    int result = clock_gettime(clock, &value);
    seconds_and_nanoseconds[0] = value.tv_sec;
    seconds_and_nanoseconds[1] = value.tv_nsec;
    return result == 0 ? 0 : errno;
}

int fx_gettimeofday(long *seconds_and_microseconds)
{
    struct timeval value;
    int result = gettimeofday(&value, NULL);
    seconds_and_microseconds[0] = value.tv_sec;
    seconds_and_microseconds[1] = value.tv_usec;
    return result;
}

int fx_gmtime(long seconds, int *fields)
{
    time_t value = seconds;
    struct tm parts;
    if (gmtime_r(&value, &parts) == NULL) {
        return -1;
    }
    fields[0] = parts.tm_sec;
    fields[1] = parts.tm_min;
    fields[2] = parts.tm_hour;
    fields[3] = parts.tm_mday;
    fields[4] = parts.tm_mon;
    fields[5] = parts.tm_year;
    fields[6] = parts.tm_wday;
    fields[7] = parts.tm_yday;
    fields[8] = (int)parts.tm_gmtoff;
    return strcmp(parts.tm_zone, "GMT");
}

long fx_timegm(int year, int month, int day, int hour, int minute, int second, int *normalized_month)
{
    struct tm parts;
    long result;
    memset(&parts, 0, sizeof(parts));
    parts.tm_year = year - 1900;
    parts.tm_mon = month;
    parts.tm_mday = day;
    parts.tm_hour = hour;
    parts.tm_min = minute;
    parts.tm_sec = second;
    result = timegm(&parts);
    *normalized_month = parts.tm_mon;
    return result;
}

int fx_asctime(long seconds, char *buffer)
{
    time_t value = seconds;
    struct tm parts;
    gmtime_r(&value, &parts);
    return asctime_r(&parts, buffer) == buffer;
}

unsigned fx_sleep_remaining(void)
{
    return sleep(0);
}

int fx_usleep(unsigned microseconds)
{
    return usleep(microseconds);
}

int fx_chk_strlen(const char *text, size_t size)
{
    return (int)__strlen_chk(text, size);
}

int fx_chk_memcpy(char *destination, const char *source, size_t count, size_t size)
{
    __memcpy_chk(destination, source, count, size);
    return destination[0];
}

int fx_chk_memset(char *destination, size_t count, size_t size)
{
    __memset_chk(destination, 7, count, size);
    return destination[0];
}

int fx_chk_strcpy(char *destination, const char *source, size_t size)
{
    __strcpy_chk(destination, source, size);
    return destination[0];
}

int fx_chk_strcat(char *destination, const char *source, size_t size)
{
    __strcat_chk(destination, source, size);
    return (int)strlen(destination);
}

int fx_chk_strncpy2(char *destination, const char *source, size_t count, size_t size, size_t source_size)
{
    __strncpy_chk2(destination, source, count, size, source_size);
    return destination[0];
}

int fx_chk_strchr(const char *text, int character, size_t size)
{
    const char *found = __strchr_chk(text, character, size);
    return found == NULL ? -1 : (int)(found - text);
}

int fx_chk_fd_set(int fd, size_t size)
{
    unsigned long set[16];
    memset(set, 0, sizeof(set));
    __FD_SET_chk(fd, (fd_set *)set, size);
    return (int)(set[fd / 64] >> (fd % 64)) & 1;
}

void fx_assert_fail(void)
{
    __assert2("fx_libc.c", 42, "fx_assert_fail", "value > 0");
}

void fx_set_abort_message(const char *message)
{
    android_set_abort_message(message);
}

void fx_abort(void)
{
    abort();
}

void fx_exit_with_atexit(int status);

int fx_exit_marker;

static void exit_marker(void *argument)
{
    fx_exit_marker = fx_exit_marker * 10 + *(int *)argument;
}

static int first_order = 1;
static int second_order = 2;

__attribute__((visibility("hidden"))) void *__dso_handle = &__dso_handle;

extern int __cxa_atexit(void (*function)(void *), void *argument, void *dso_handle);
extern void exit(int status);

void fx_exit_with_atexit(int status)
{
    __cxa_atexit(exit_marker, &first_order, &__dso_handle);
    __cxa_atexit(exit_marker, &second_order, &__dso_handle);
    exit(status);
}

int fx_ctype_class(int character, int which)
{
    switch (which) {
    case 0: return isalpha(character) != 0;
    case 1: return isdigit(character) != 0;
    case 2: return isalnum(character) != 0;
    case 3: return isspace(character) != 0;
    case 4: return ispunct(character) != 0;
    case 5: return isxdigit(character) != 0;
    case 6: return tolower(character);
    case 7: return toupper(character);
    }
    return -1;
}

int fx_ctype_table(int character)
{
    return (unsigned char)_ctype_[character + 1];
}

int fx_setlocale(int category, const char *name, char *report)
{
    char *result = setlocale(category, name);
    if (result == NULL) {
        report[0] = 0;
        return errno;
    }
    strcpy(report, result);
    return 0;
}

long fx_mb_cur_max(void)
{
    return (long)__ctype_get_mb_cur_max();
}

long fx_locale_flow(const char *name, int *errno_out)
{
    void *locale = newlocale((1 << 13) - 1, name, NULL);
    void *previous;
    long inside;
    if (locale == NULL) {
        *errno_out = errno;
        return -1;
    }
    previous = uselocale(locale);
    inside = (long)__ctype_get_mb_cur_max();
    uselocale(previous);
    freelocale(locale);
    return inside * 10 + (previous == (void *)-1);
}

int fx_localeconv(char *decimal_point, unsigned char *fields)
{
    const char *const *strings = localeconv();
    const unsigned char *bytes = (const unsigned char *)localeconv();
    strcpy(decimal_point, strings[0]);
    for (int index = 0; index < 14; index++) {
        fields[index] = bytes[80 + index];
    }
    return (int)strlen(strings[1]) + (int)strlen(strings[9]);
}

extern char *tzname[2];
extern long timezone;
extern int daylight;
extern void tzset(void);

int fx_set_time_zone(const char *name)
{
    int result = name == NULL ? unsetenv("TZ") : setenv("TZ", name, 1);
    tzset();
    return result;
}

int fx_time_zone_globals(char *names, long *seconds_west)
{
    strcpy(names, tzname[0]);
    strcat(names, "|");
    strcat(names, tzname[1]);
    *seconds_west = timezone;
    return daylight;
}

int fx_localtime(long seconds, int *fields, char *zone)
{
    time_t value = seconds;
    struct tm parts;
    if (localtime_r(&value, &parts) == NULL) {
        return -1;
    }
    fields[0] = parts.tm_sec;
    fields[1] = parts.tm_min;
    fields[2] = parts.tm_hour;
    fields[3] = parts.tm_mday;
    fields[4] = parts.tm_mon;
    fields[5] = parts.tm_year;
    fields[6] = parts.tm_wday;
    fields[7] = parts.tm_yday;
    fields[8] = parts.tm_isdst;
    fields[9] = (int)parts.tm_gmtoff;
    strcpy(zone, parts.tm_zone);
    return 0;
}

long fx_mktime(int year, int month, int day, int hour, int minute, int second, int isdst, int *fields)
{
    struct tm parts;
    long result;
    memset(&parts, 0, sizeof(parts));
    parts.tm_year = year - 1900;
    parts.tm_mon = month;
    parts.tm_mday = day;
    parts.tm_hour = hour;
    parts.tm_min = minute;
    parts.tm_sec = second;
    parts.tm_isdst = isdst;
    result = mktime(&parts);
    fields[0] = parts.tm_sec;
    fields[1] = parts.tm_min;
    fields[2] = parts.tm_hour;
    fields[3] = parts.tm_mday;
    fields[4] = parts.tm_mon;
    fields[5] = parts.tm_year;
    fields[6] = parts.tm_wday;
    fields[7] = parts.tm_yday;
    fields[8] = parts.tm_isdst;
    return result;
}

size_t fx_strftime(char *buffer, size_t size, const char *format, long seconds, int local)
{
    time_t value = seconds;
    struct tm parts;
    if (local) {
        localtime_r(&value, &parts);
    } else {
        gmtime_r(&value, &parts);
    }
    return strftime(buffer, size, format, &parts);
}

int fx_ctime(long seconds, char *buffer)
{
    time_t value = seconds;
    return ctime_r(&value, buffer) == buffer;
}

size_t fx_mbrtowc(unsigned *out, const char *text, size_t count, mbstate_t *state)
{
    wchar_t value = 0xdeadbeef;
    size_t result = mbrtowc(&value, text, count, state);
    *out = (unsigned)value;
    return result;
}

size_t fx_wcrtomb(char *buffer, unsigned value, mbstate_t *state)
{
    return wcrtomb(buffer, (wchar_t)value, state);
}

unsigned fx_btowc(int character)
{
    return (unsigned)btowc(character);
}

int fx_wctob(unsigned value)
{
    return wctob((wint_t)value);
}

int fx_mbsinit(const mbstate_t *state)
{
    return mbsinit(state);
}

int fx_mbtowc(unsigned *out, const char *text, size_t count)
{
    wchar_t value = 0;
    int result = mbtowc(&value, text, count);
    *out = (unsigned)value;
    return result;
}

int fx_wctomb(char *buffer, unsigned value)
{
    return wctomb(buffer, (wchar_t)value);
}

int fx_wide_strings(void)
{
    wchar_t first[8], second[8], copy[16];
    first[0] = L'a'; first[1] = L'b'; first[2] = 0x1F600; first[3] = 0;
    second[0] = L'a'; second[1] = L'b'; second[2] = 0x1F601; second[3] = 0;
    if (wcslen(first) != 3) return 1;
    if (wcscmp(first, first) != 0) return 2;
    if (wcscmp(first, second) >= 0) return 3;
    if (wcsncmp(first, second, 2) != 0) return 4;
    if (wcschr(first, 0x1F600) != first + 2) return 5;
    if (wcsrchr(first, L'a') != first) return 6;
    wcscpy(copy, first);
    wcscat(copy, second);
    if (wcslen(copy) != 6 || copy[3] != L'a') return 7;
    if (wmemcmp(first, second, 3) != -1 || wmemcmp(second, first, 3) != 1 || wmemcmp(first, first, 3) != 0) return 8;
    if (wmemchr(first, 0x1F600, 4) != first + 2) return 9;
    wmemset(copy, 7, 3);
    if (copy[2] != 7 || copy[3] != L'a') return 10;
    wcsncpy(copy, first, 8);
    if (copy[3] != 0 || copy[7] != 0) return 11;
    return 0;
}
