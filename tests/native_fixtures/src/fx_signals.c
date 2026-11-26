#define _GNU_SOURCE 1
#include <errno.h>
#include <pthread.h>
#include <sched.h>
#include <setjmp.h>
#include <signal.h>
#include <stdint.h>
#include <string.h>
#include <sys/resource.h>
#include <sys/ucontext.h>
#include <stdlib.h>
#include <unistd.h>

extern int tgkill(int group, int tid, int signal);
extern void fx_emit(const char *text);

static volatile int seen_signal;
static volatile int seen_code;
static volatile int seen_pid;
static volatile int seen_tid;
static volatile long seen_address;
static volatile int seen_blocked_inside;
static volatile int seen_count;
static volatile unsigned long seen_pc;
static volatile unsigned long seen_sp_in_handler;
static volatile int seen_stack_flags;

static void reset_seen(void)
{
    seen_signal = 0;
    seen_code = 0;
    seen_pid = 0;
    seen_tid = 0;
    seen_address = 0;
    seen_blocked_inside = 0;
    seen_count = 0;
    seen_pc = 0;
    seen_stack_flags = 0;
}

static void record_handler(int signal, siginfo_t *info, void *context)
{
    ucontext_t *state = context;
    sigset_t current;
    seen_signal = signal;
    seen_code = info->si_code;
    seen_pid = info->si_pid;
    seen_tid = gettid();
    seen_pc = state->uc_mcontext.pc;
    seen_sp_in_handler = (unsigned long)__builtin_frame_address(0);
    seen_stack_flags = state->uc_stack.ss_flags;
    sigprocmask(SIG_BLOCK, 0, &current);
    seen_blocked_inside = sigismember(&current, signal);
    seen_count++;
}

static void skip_instruction_handler(int signal, siginfo_t *info, void *context)
{
    ucontext_t *state = context;
    seen_signal = signal;
    seen_code = info->si_code;
    seen_address = (long)info->si_addr;
    seen_pc = state->uc_mcontext.pc;
    state->uc_mcontext.regs[0] = 77;
    state->uc_mcontext.pc += 4;
    seen_count++;
}

int fx_install(int signal, int flags, void (*handler)(int, siginfo_t *, void *))
{
    struct sigaction action;
    memset(&action, 0, sizeof(action));
    action.sa_sigaction = handler;
    action.sa_flags = flags | SA_SIGINFO;
    sigemptyset(&action.sa_mask);
    return sigaction(signal, &action, 0);
}

int fx_install_recorder(int signal, int flags)
{
    reset_seen();
    return fx_install(signal, flags, record_handler);
}

int fx_install_skipper(int signal)
{
    reset_seen();
    return fx_install(signal, 0, skip_instruction_handler);
}

int fx_raise(int signal)
{
    return raise(signal);
}

int fx_seen_signal(void) { return seen_signal; }
int fx_seen_code(void) { return seen_code; }
int fx_seen_pid(void) { return seen_pid; }
int fx_seen_tid(void) { return seen_tid; }
long fx_seen_address(void) { return seen_address; }
int fx_seen_blocked_inside(void) { return seen_blocked_inside; }
int fx_seen_count(void) { return seen_count; }
unsigned long fx_seen_pc(void) { return seen_pc; }
int fx_seen_stack_flags(void) { return seen_stack_flags; }
unsigned long fx_seen_sp(void) { return seen_sp_in_handler; }
int fx_getpid(void) { return getpid(); }

int fx_blocked_after_handler(int signal)
{
    sigset_t current;
    sigprocmask(SIG_BLOCK, 0, &current);
    return sigismember(&current, signal);
}

int fx_block_raise_unblock(int signal)
{
    sigset_t set;
    sigemptyset(&set);
    sigaddset(&set, signal);
    sigprocmask(SIG_BLOCK, &set, 0);
    raise(signal);
    int before = seen_count;
    sigprocmask(SIG_UNBLOCK, &set, 0);
    return before * 10 + seen_count;
}

long fx_fault_resume(void)
{
    register long result __asm__("x0") = 1234;
    __asm__ volatile("ldr w1, [%[address]]" : "+r"(result) : [address] "r"(0x10UL) : "x1", "memory");
    return result;
}

long fx_illegal_resume(void)
{
    register long result __asm__("x0") = 1234;
    __asm__ volatile(".inst 0x00000000" : "+r"(result)::"memory");
    return result;
}

long fx_breakpoint_resume(void)
{
    register long result __asm__("x0") = 1234;
    __asm__ volatile("brk #0" : "+r"(result)::"memory");
    return result;
}

static volatile int worker_release;
static volatile int worker_started;
static volatile int worker_tid;

static void *worker_spin(void *argument)
{
    worker_tid = gettid();
    worker_started = 1;
    for (int round = 0; round < 3000 && !worker_release && seen_count == 0; round++)
        sched_yield();
    return 0;
}

int fx_signal_another_thread(int signal)
{
    pthread_t worker;
    worker_release = 0;
    worker_started = 0;
    pthread_create(&worker, 0, worker_spin, 0);
    while (!worker_started)
        sched_yield();
    int code = pthread_kill(worker, signal);
    pthread_join(worker, 0);
    return code;
}

int fx_worker_tid(void) { return worker_tid; }

int fx_send_to_thread_by_tid(int signal)
{
    pthread_t worker;
    worker_release = 0;
    worker_started = 0;
    pthread_create(&worker, 0, worker_spin, 0);
    while (!worker_started)
        sched_yield();
    int code = (int)tgkill(getpid(), worker_tid, signal);
    int saved = errno;
    pthread_join(worker, 0);
    return code == 0 ? 0 : saved;
}

int fx_pthread_kill_probe_exited(void)
{
    pthread_t worker;
    worker_release = 1;
    worker_started = 0;
    pthread_create(&worker, 0, worker_spin, 0);
    int live = pthread_kill(worker, 0);
    pthread_join(worker, 0);
    return live;
}

static void plain_handler(int signal) { seen_signal = signal; }

long fx_signal_returns_previous(void)
{
    void (*first)(int) = signal(SIGUSR2, plain_handler);
    void (*second)(int) = signal(SIGUSR2, SIG_DFL);
    return (first == SIG_DFL) * 10 + (second == plain_handler);
}

int fx_sigaction_read_back(void)
{
    struct sigaction written, read;
    memset(&written, 0, sizeof(written));
    written.sa_sigaction = record_handler;
    written.sa_flags = SA_SIGINFO | SA_RESTART | SA_NODEFER;
    sigemptyset(&written.sa_mask);
    sigaddset(&written.sa_mask, SIGUSR2);
    sigaddset(&written.sa_mask, SIGKILL);
    sigaction(SIGUSR1, &written, 0);
    sigaction(SIGUSR1, 0, &read);
    int good = read.sa_sigaction == record_handler;
    good += (read.sa_flags == (SA_SIGINFO | SA_RESTART | SA_NODEFER)) * 10;
    good += sigismember(&read.sa_mask, SIGUSR2) * 100;
    good += (sigismember(&read.sa_mask, SIGKILL) == 0) * 1000;
    return good;
}

int fx_sigaction_invalid_codes(void)
{
    struct sigaction action;
    memset(&action, 0, sizeof(action));
    int kill_code = sigaction(SIGKILL, &action, 0) == -1 ? errno : 0;
    int stop_code = sigaction(SIGSTOP, &action, 0) == -1 ? errno : 0;
    int zero_code = sigaction(0, &action, 0) == -1 ? errno : 0;
    int high_code = sigaction(65, &action, 0) == -1 ? errno : 0;
    return kill_code * 1000000 + stop_code * 10000 + zero_code * 100 + high_code;
}

static char alternate_storage[32768] __attribute__((aligned(16)));

int fx_altstack_handler_runs_on_it(void)
{
    stack_t stack;
    stack.ss_sp = alternate_storage;
    stack.ss_size = sizeof(alternate_storage);
    stack.ss_flags = 0;
    sigaltstack(&stack, 0);
    fx_install_recorder(SIGUSR1, SA_ONSTACK);
    raise(SIGUSR1);
    unsigned long frame = seen_sp_in_handler;
    int inside = frame >= (unsigned long)alternate_storage && frame < (unsigned long)alternate_storage + sizeof(alternate_storage);
    stack.ss_flags = SS_DISABLE;
    sigaltstack(&stack, 0);
    return inside * 10 + seen_stack_flags;
}

int fx_uninstall(int signal_number)
{
    signal(signal_number, SIG_DFL);
    return 0;
}

static void abort_handler(int signal)
{
    fx_emit("abort handler ran\n");
}

int fx_abort_after_handler(void)
{
    signal(SIGABRT, abort_handler);
    abort();
    return 0;
}

static sigjmp_buf recovery;
static volatile int armed;

static void jumping_handler(int signal)
{
    if (armed) {
        armed = 0;
        seen_count++;
        siglongjmp(recovery, signal);
    }
}

static int install_plain(int signal, int flags, void (*handler)(int))
{
    return fx_install(signal, flags, (void (*)(int, siginfo_t *, void *))handler);
}

int fx_install_jumper(int signal)
{
    reset_seen();
    return install_plain(signal, 0, jumping_handler);
}

long fx_jump_values(void)
{
    jmp_buf env;
    volatile long seen = 0;
    volatile int step = 0;
    int code = setjmp(env);
    seen = seen * 100 + code;
    step++;
    if (step == 1)
        longjmp(env, 5);
    if (step == 2)
        longjmp(env, 0);
    if (_setjmp(env) == 0)
        _longjmp(env, 7);
    else
        seen = seen * 100 + 7;
    return seen;
}

__attribute__((visibility("hidden"))) long fx_register_env[33];

__asm__(".text\n"
        ".globl fx_registers_survive_longjmp\n"
        ".type fx_registers_survive_longjmp, %function\n"
        "fx_registers_survive_longjmp:\n"
        "stp x29, x30, [sp, #-160]!\n"
        "mov x29, sp\n"
        "stp x19, x20, [sp, #16]\n"
        "stp x21, x22, [sp, #32]\n"
        "stp x23, x24, [sp, #48]\n"
        "stp x25, x26, [sp, #64]\n"
        "stp x27, x28, [sp, #80]\n"
        "stp d8, d9, [sp, #96]\n"
        "stp d10, d11, [sp, #112]\n"
        "stp d12, d13, [sp, #128]\n"
        "stp d14, d15, [sp, #144]\n"
        ".irp n,19,20,21,22,23,24,25,26,27,28\n"
        "mov x\\n, #\\n\n"
        ".endr\n"
        ".irp n,8,9,10,11,12,13,14,15\n"
        "mov x9, #\\n\n"
        "fmov d\\n, x9\n"
        ".endr\n"
        "adrp x0, fx_register_env\n"
        "add x0, x0, :lo12:fx_register_env\n"
        "mov w1, #0\n"
        "bl sigsetjmp\n"
        "cbnz w0, 1f\n"
        ".irp n,19,20,21,22,23,24,25,26,27,28\n"
        "mov x\\n, xzr\n"
        ".endr\n"
        ".irp n,8,9,10,11,12,13,14,15\n"
        "fmov d\\n, xzr\n"
        ".endr\n"
        "sub sp, sp, #64\n"
        "adrp x0, fx_register_env\n"
        "add x0, x0, :lo12:fx_register_env\n"
        "mov w1, #1\n"
        "bl siglongjmp\n"
        "1:\n"
        "mov x0, xzr\n"
        "mov x9, sp\n"
        "cmp x9, x29\n"
        "cinc x0, x0, eq\n"
        ".irp n,19,20,21,22,23,24,25,26,27,28\n"
        "cmp x\\n, #\\n\n"
        "cinc x0, x0, eq\n"
        ".endr\n"
        ".irp n,8,9,10,11,12,13,14,15\n"
        "fmov x9, d\\n\n"
        "cmp x9, #\\n\n"
        "cinc x0, x0, eq\n"
        ".endr\n"
        "ldp d14, d15, [sp, #144]\n"
        "ldp d12, d13, [sp, #128]\n"
        "ldp d10, d11, [sp, #112]\n"
        "ldp d8, d9, [sp, #96]\n"
        "ldp x27, x28, [sp, #80]\n"
        "ldp x25, x26, [sp, #64]\n"
        "ldp x23, x24, [sp, #48]\n"
        "ldp x21, x22, [sp, #32]\n"
        "ldp x19, x20, [sp, #16]\n"
        "ldp x29, x30, [sp], #160\n"
        "ret\n");

static int blocked_then_unblock(int signal)
{
    sigset_t set;
    int blocked = fx_blocked_after_handler(signal);
    sigemptyset(&set);
    sigaddset(&set, signal);
    sigprocmask(SIG_UNBLOCK, &set, 0);
    return blocked;
}

int fx_recover_from_fault(int save_mask)
{
    int code = sigsetjmp(recovery, save_mask);
    if (code == 0) {
        armed = 1;
        *(volatile int *)0x10 = 1;
        return -1;
    }
    return code * 10 + blocked_then_unblock(code);
}

int fx_recover_from_raise(int signal)
{
    int code = sigsetjmp(recovery, 1);
    if (code == 0) {
        armed = 1;
        raise(signal);
        return -1;
    }
    return code * 10 + blocked_then_unblock(code);
}

int fx_recover_from_abort(void)
{
    int code = sigsetjmp(recovery, 1);
    if (code == 0) {
        armed = 1;
        abort();
    }
    return code;
}

static void raising_handler(int signal)
{
    raise(SIGUSR2);
    seen_count += 100;
}

static void faulting_handler(int signal)
{
    *(volatile int *)0x10 = 1;
    seen_count += 100;
}

int fx_jump_past_an_outer_handler(int fault)
{
    reset_seen();
    install_plain(SIGUSR1, 0, fault ? faulting_handler : raising_handler);
    install_plain(fault ? SIGSEGV : SIGUSR2, 0, jumping_handler);
    int result = fx_recover_from_raise(SIGUSR1);
    return result * 1000 + seen_count * 10 + fx_blocked_after_handler(SIGUSR1);
}

static void counting_handler(int signal) { seen_count += 100; }

int fx_jump_before_a_queued_handler(void)
{
    sigset_t set;
    reset_seen();
    install_plain(SIGUSR1, 0, counting_handler);
    install_plain(SIGUSR2, 0, jumping_handler);
    sigemptyset(&set);
    sigaddset(&set, SIGUSR1);
    sigaddset(&set, SIGUSR2);
    int code = sigsetjmp(recovery, 1);
    if (code == 0) {
        armed = 1;
        sigprocmask(SIG_BLOCK, &set, 0);
        raise(SIGUSR1);
        raise(SIGUSR2);
        sigprocmask(SIG_UNBLOCK, &set, 0);
        return -1;
    }
    return code * 1000 + seen_count;
}

int fx_recover_on_altstack(void)
{
    stack_t stack, current;
    stack.ss_sp = alternate_storage;
    stack.ss_size = sizeof(alternate_storage);
    stack.ss_flags = 0;
    sigaltstack(&stack, 0);
    install_plain(SIGSEGV, SA_ONSTACK, jumping_handler);
    int first = fx_recover_from_fault(1);
    int second = fx_recover_from_fault(1);
    sigaltstack(0, &current);
    stack.ss_flags = SS_DISABLE;
    sigaltstack(&stack, 0);
    return (first == 110) * 100 + (second == 110) * 10 + current.ss_flags;
}

static pthread_mutex_t held_mutex = PTHREAD_MUTEX_INITIALIZER;
static volatile int jumped_code;

static void *lock_until_signalled(void *argument)
{
    int code = sigsetjmp(recovery, 1);
    if (code == 0) {
        armed = 1;
        pthread_mutex_lock(&held_mutex);
        return 0;
    }
    jumped_code = code;
    return 0;
}

int fx_jump_out_of_a_blocked_lock(void)
{
    pthread_t worker;
    armed = 0;
    jumped_code = 0;
    pthread_mutex_lock(&held_mutex);
    pthread_create(&worker, 0, lock_until_signalled, 0);
    while (!armed && !jumped_code)
        sched_yield();
    for (int round = 0; round < 2000 && !jumped_code; round++) {
        pthread_kill(worker, SIGUSR1);
        for (int spin = 0; spin < 100 && !jumped_code; spin++)
            sched_yield();
    }
    pthread_join(worker, 0);
    pthread_mutex_unlock(&held_mutex);
    return jumped_code;
}

static int jumping_compare(const void *left, const void *right)
{
    siglongjmp(recovery, 1);
}

int fx_jump_across_qsort(void)
{
    int values[2] = {2, 1};
    if (sigsetjmp(recovery, 0) == 0)
        qsort(values, 2, sizeof(int), jumping_compare);
    return 1;
}

int fx_jump_through_a_damaged_buffer(int clear)
{
    sigjmp_buf env;
    if (sigsetjmp(env, 0) == 0) {
        if (clear)
            memset(env, 0, sizeof(env));
        else
            env[12] ^= 1;
        siglongjmp(env, 1);
    }
    return 0;
}

static void *report_priority(void *result)
{
    *(int *)result = getpriority(PRIO_PROCESS, 0);
    return 0;
}

int fx_child_priority(int nice)
{
    pthread_t worker;
    int child = 100;
    setpriority(PRIO_PROCESS, 0, nice);
    pthread_create(&worker, 0, report_priority, &child);
    pthread_join(worker, 0);
    setpriority(PRIO_PROCESS, 0, 0);
    return child;
}
