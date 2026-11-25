// cite: bionic libc.map.txt#_Unwind_RaiseException, _Unwind_Resume, _Unwind_GetIP, _Unwind_GetRegionStart, _Unwind_GetLanguageSpecificData, _Unwind_GetCFA, _Unwind_Backtrace, _Unwind_SetGR, _Unwind_SetIP, _Unwind_FindEnclosingFunction

#define FX_UNWIND_CLASS 0x5744554e57494e44ull

#define FX_URC_FATAL_PHASE1_ERROR 3
#define FX_URC_END_OF_STACK 5
#define FX_URC_HANDLER_FOUND 6
#define FX_URC_INSTALL_CONTEXT 7
#define FX_URC_CONTINUE_UNWIND 8

#define FX_UA_SEARCH_PHASE 1
#define FX_UA_CLEANUP_PHASE 2
#define FX_UA_HANDLER_FRAME 4

struct _Unwind_Exception {
    unsigned long long exception_class;
    void (*exception_cleanup)(int reason, struct _Unwind_Exception *exception);
    unsigned long long private_1;
    unsigned long long private_2;
} __attribute__((aligned(16)));

struct _Unwind_Context;

struct fx_unwind_frame {
    unsigned long ip;
    unsigned long cfa;
};

struct fx_unwind_hit {
    unsigned long region;
    int count;
};

struct fx_unwind_observation {
    unsigned long ip;
    unsigned long region;
    unsigned long lsda;
    unsigned long resume_ip;
    unsigned long lsda_error;
};

struct fx_unwind_case {
    unsigned long catch_region;
    void *landing;
};

extern int _Unwind_RaiseException(struct _Unwind_Exception *exception);
extern void _Unwind_Resume(struct _Unwind_Exception *exception) __attribute__((noreturn));
extern int _Unwind_Backtrace(int (*trace)(struct _Unwind_Context *, void *), void *argument);
extern unsigned long _Unwind_GetIP(struct _Unwind_Context *context);
extern unsigned long _Unwind_GetCFA(struct _Unwind_Context *context);
extern unsigned long _Unwind_GetRegionStart(struct _Unwind_Context *context);
extern unsigned long _Unwind_GetLanguageSpecificData(struct _Unwind_Context *context);
extern unsigned long _Unwind_FindEnclosingFunction(unsigned long address);
extern void _Unwind_SetGR(struct _Unwind_Context *context, int index, unsigned long value);
extern void _Unwind_SetIP(struct _Unwind_Context *context, unsigned long value);

static volatile int catch_body_runs;
static volatile int handler_found;
static volatile int cleanup_runs;
static volatile int catch_value;
static volatile struct _Unwind_Exception *catch_exception;
static volatile struct _Unwind_Exception *cleanup_exception;
static volatile unsigned long observed_ip;
static volatile unsigned long observed_region;
static volatile unsigned long observed_lsda;
static volatile unsigned long pending_resume_ip;
static volatile unsigned long lsda_error;
static volatile struct fx_unwind_case *active_case;

static volatile struct fx_unwind_hit hits[16];
static volatile int hit_count;

static void record_hit(unsigned long region)
{
    for (int index = 0; index < hit_count; ++index) {
        if (hits[index].region == region) {
            hits[index].count += 1;
            return;
        }
    }
    if (hit_count < 16) {
        hits[hit_count].region = region;
        hits[hit_count].count = 1;
        hit_count += 1;
    }
}

static unsigned long read_uleb(const unsigned char **cursor)
{
    unsigned long value = 0;
    unsigned int shift = 0;
    for (;;) {
        unsigned char byte = *(*cursor)++;
        value |= (unsigned long)(byte & 0x7f) << shift;
        shift += 7;
        if ((byte & 0x80) == 0) {
            return value;
        }
    }
}

static unsigned long read_sleb(const unsigned char **cursor)
{
    unsigned long value = 0;
    unsigned int shift = 0;
    unsigned char byte;
    do {
        byte = *(*cursor)++;
        value |= (unsigned long)(byte & 0x7f) << shift;
        shift += 7;
    } while (byte & 0x80);
    if (shift < 64 && (byte & 0x40) != 0) {
        value |= ~0ul << shift;
    }
    return value;
}

static unsigned long read_encoded(const unsigned char **cursor, unsigned char encoding)
{
    switch (encoding & 0x0f) {
    case 0x01:
        return read_uleb(cursor);
    case 0x09:
        return read_sleb(cursor);
    case 0x03: {
        unsigned int value;
        __builtin_memcpy(&value, *cursor, 4);
        *cursor += 4;
        return value;
    }
    case 0x0b: {
        int value;
        __builtin_memcpy(&value, *cursor, 4);
        *cursor += 4;
        return (unsigned long)(long)value;
    }
    case 0x04:
    case 0x0c: {
        unsigned long value;
        __builtin_memcpy(&value, *cursor, 8);
        *cursor += 8;
        return value;
    }
    case 0x02: {
        unsigned short value;
        __builtin_memcpy(&value, *cursor, 2);
        *cursor += 2;
        return value;
    }
    case 0x0a: {
        short value;
        __builtin_memcpy(&value, *cursor, 2);
        *cursor += 2;
        return (unsigned long)(long)value;
    }
    default:
        lsda_error = 1;
        return 0;
    }
}

static unsigned long landing_pad_for(unsigned long lsda, unsigned long region, unsigned long ip)
{
    if (lsda == 0) {
        return 0;
    }
    const unsigned char *cursor = (const unsigned char *)lsda;
    unsigned char lpstart_encoding = *cursor++;
    unsigned long lpstart = region;
    if (lpstart_encoding != 0xff) {
        lpstart = read_encoded(&cursor, lpstart_encoding & 0x7f);
    }
    unsigned char ttype_encoding = *cursor++;
    if (ttype_encoding != 0xff) {
        read_uleb(&cursor);
    }
    unsigned char callsite_encoding = *cursor++;
    unsigned long length = read_uleb(&cursor);
    const unsigned char *end = cursor + length;
    while (cursor < end) {
        unsigned long start = read_encoded(&cursor, callsite_encoding & 0x0f);
        unsigned long span = read_encoded(&cursor, callsite_encoding & 0x0f);
        unsigned long landing = read_encoded(&cursor, callsite_encoding & 0x0f);
        read_uleb(&cursor);
        if (ip >= start && ip < start + span) {
            return landing == 0 ? 0 : lpstart + landing;
        }
    }
    return 0;
}

static void cleanup_middle_fn(struct _Unwind_Exception **slot)
{
    cleanup_runs += 1;
    cleanup_exception = *slot;
}

static int catch_body(struct _Unwind_Exception *exception)
{
    catch_body_runs += 1;
    catch_exception = exception;
    catch_value = exception != 0 && exception->exception_class == FX_UNWIND_CLASS ? 41 : -5;
    return catch_value;
}

int __gcc_personality_v0(int version, int actions, unsigned long long exception_class,
                         struct _Unwind_Exception *exception, struct _Unwind_Context *context);

int __gcc_personality_v0(int version, int actions, unsigned long long exception_class,
                         struct _Unwind_Exception *exception, struct _Unwind_Context *context)
{
    if (version != 1) {
        return FX_URC_FATAL_PHASE1_ERROR;
    }
    unsigned long region = _Unwind_GetRegionStart(context);
    record_hit(region);
    if ((actions & FX_UA_SEARCH_PHASE) != 0) {
        unsigned long resume_ip = pending_resume_ip;
        if (active_case == 0 || region != active_case->catch_region) {
            return FX_URC_CONTINUE_UNWIND;
        }
        unsigned long ip = _Unwind_GetIP(context);
        if (ip > resume_ip || resume_ip - ip >= 64) {
            return FX_URC_CONTINUE_UNWIND;
        }
        observed_ip = ip;
        observed_region = region;
        observed_lsda = _Unwind_GetLanguageSpecificData(context);
        handler_found += 1;
        return FX_URC_HANDLER_FOUND;
    }
    if ((actions & FX_UA_CLEANUP_PHASE) != 0) {
        if (exception_class != FX_UNWIND_CLASS) {
            return FX_URC_CONTINUE_UNWIND;
        }
        if ((actions & FX_UA_HANDLER_FRAME) != 0 && active_case != 0 && region == active_case->catch_region) {
            _Unwind_SetGR(context, 0, (unsigned long)exception);
            _Unwind_SetIP(context, (unsigned long)active_case->landing);
            return FX_URC_INSTALL_CONTEXT;
        }
        unsigned long ip = _Unwind_GetIP(context);
        unsigned long pad = landing_pad_for(_Unwind_GetLanguageSpecificData(context), region, ip - region);
        if (pad != 0) {
            _Unwind_SetGR(context, 0, (unsigned long)exception);
            _Unwind_SetIP(context, pad);
            return FX_URC_INSTALL_CONTEXT;
        }
        return FX_URC_CONTINUE_UNWIND;
    }
    return FX_URC_CONTINUE_UNWIND;
}

__attribute__((noinline)) int fx_unwind_thrower_plain(struct _Unwind_Exception *exception)
{
    return (int)_Unwind_RaiseException(exception);
}

__attribute__((noinline)) int fx_unwind_thrower(struct _Unwind_Exception *exception)
{
    struct _Unwind_Exception *slot __attribute__((cleanup(cleanup_middle_fn))) = exception;
    (void)slot;
    pending_resume_ip = (unsigned long)&&after_raise;
    int value = (int)_Unwind_RaiseException(exception);
after_raise:
    return value;
}

__attribute__((noinline)) int fx_unwind_middle_cleanup(struct _Unwind_Exception *exception)
{
    struct _Unwind_Exception *slot __attribute__((cleanup(cleanup_middle_fn))) = exception;
    (void)slot;
    return fx_unwind_thrower_plain(exception);
}

__attribute__((noinline)) int fx_unwind_outer_catcher(struct _Unwind_Exception *exception)
{
    struct _Unwind_Exception *slot __attribute__((cleanup(cleanup_middle_fn))) = exception;
    (void)slot;
    pending_resume_ip = (unsigned long)&&after_call;
    int value = fx_unwind_middle_cleanup(exception);
after_call:
    return value;
}

__attribute__((noinline)) int fx_unwind_middle_plain(struct _Unwind_Exception *exception)
{
    return fx_unwind_thrower_plain(exception);
}

__attribute__((noinline)) int fx_unwind_outer_caller(struct _Unwind_Exception *exception)
{
    struct _Unwind_Exception *slot __attribute__((cleanup(cleanup_middle_fn))) = exception;
    (void)slot;
    pending_resume_ip = (unsigned long)&&after_call;
    int value = fx_unwind_middle_plain(exception);
after_call:
    return value;
}

static struct fx_unwind_case case_one;
static struct fx_unwind_case case_two;
static struct fx_unwind_case case_three;
static struct _Unwind_Exception exception_one;
static struct _Unwind_Exception exception_two;
static struct _Unwind_Exception exception_three;

static void init_exception(struct _Unwind_Exception *exception)
{
    exception->exception_class = FX_UNWIND_CLASS;
    exception->exception_cleanup = 0;
    exception->private_1 = 0;
    exception->private_2 = 0;
}

int fx_unwind_case_thrower(void)
{
    init_exception(&exception_one);
    case_one.catch_region = (unsigned long)&fx_unwind_thrower;
    case_one.landing = (void *)&catch_body;
    active_case = &case_one;
    int value = fx_unwind_thrower(&exception_one);
    active_case = 0;
    return value;
}

int fx_unwind_case_caller(void)
{
    init_exception(&exception_two);
    case_two.catch_region = (unsigned long)&fx_unwind_outer_caller;
    case_two.landing = (void *)&catch_body;
    active_case = &case_two;
    int value = fx_unwind_outer_caller(&exception_two);
    active_case = 0;
    return value;
}

int fx_unwind_case_cleanup(void)
{
    init_exception(&exception_three);
    case_three.catch_region = (unsigned long)&fx_unwind_outer_catcher;
    case_three.landing = (void *)&catch_body;
    active_case = &case_three;
    int value = fx_unwind_outer_catcher(&exception_three);
    active_case = 0;
    return value;
}

int fx_unwind_catch_val(void)
{
    return catch_value;
}

struct _Unwind_Exception *fx_unwind_catch_exception(void)
{
    return (struct _Unwind_Exception *)catch_exception;
}

struct _Unwind_Exception *fx_unwind_cleanup_exception(void)
{
    return (struct _Unwind_Exception *)cleanup_exception;
}

struct _Unwind_Exception *fx_unwind_case_exception(int index)
{
    if (index == 0) {
        return &exception_one;
    }
    if (index == 1) {
        return &exception_two;
    }
    return &exception_three;
}

int fx_unwind_catch_runs(void)
{
    return catch_body_runs;
}

int fx_unwind_cleanup_runs(void)
{
    return cleanup_runs;
}

int fx_unwind_handler_found(void)
{
    return handler_found;
}

void fx_unwind_observe(struct fx_unwind_observation *observation)
{
    observation->ip = observed_ip;
    observation->region = observed_region;
    observation->lsda = observed_lsda;
    observation->resume_ip = pending_resume_ip;
    observation->lsda_error = lsda_error;
}

int fx_unwind_hits(struct fx_unwind_hit *out, int capacity)
{
    int count = hit_count < capacity ? hit_count : capacity;
    for (int index = 0; index < count; ++index) {
        out[index].region = hits[index].region;
        out[index].count = hits[index].count;
    }
    return hit_count;
}

void fx_unwind_reset(void)
{
    catch_body_runs = 0;
    handler_found = 0;
    cleanup_runs = 0;
    catch_value = 0;
    catch_exception = 0;
    cleanup_exception = 0;
    observed_ip = 0;
    observed_region = 0;
    observed_lsda = 0;
    pending_resume_ip = 0;
    lsda_error = 0;
    hit_count = 0;
}

unsigned long fx_unwind_enclosing(unsigned long address)
{
    return _Unwind_FindEnclosingFunction(address);
}

struct fx_unwind_trace {
    struct fx_unwind_frame *frames;
    int capacity;
    int count;
};

static int trace_callback(struct _Unwind_Context *context, void *data)
{
    struct fx_unwind_trace *trace = (struct fx_unwind_trace *)data;
    if (trace->count < trace->capacity) {
        trace->frames[trace->count].ip = _Unwind_GetIP(context);
        trace->frames[trace->count].cfa = _Unwind_GetCFA(context);
    }
    trace->count += 1;
    return 0;
}

__attribute__((noinline)) int fx_unwind_backtrace(struct fx_unwind_frame *frames, int capacity)
{
    struct fx_unwind_trace trace;
    trace.frames = frames;
    trace.capacity = capacity;
    trace.count = 0;
    int reason = (int)_Unwind_Backtrace(trace_callback, &trace);
    if (reason != FX_URC_END_OF_STACK) {
        return -1;
    }
    return trace.count;
}

__attribute__((noinline)) int fx_unwind_backtrace_driver(struct fx_unwind_frame *frames, int capacity)
{
    return fx_unwind_backtrace(frames, capacity);
}
