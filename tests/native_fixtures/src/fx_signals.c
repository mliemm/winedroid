#define _GNU_SOURCE 1
#include <errno.h>
#include <pthread.h>
#include <sched.h>
#include <signal.h>
#include <stdint.h>
#include <string.h>
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
