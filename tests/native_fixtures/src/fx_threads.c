#define _GNU_SOURCE 1
#include <errno.h>
#include <linux/futex.h>
#include <pthread.h>
#include <sched.h>
#include <semaphore.h>
#include <stdint.h>
#include <string.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

extern int __cxa_thread_atexit_impl(void (*function)(void *), void *argument, void *dso);
extern long syscall(long number, ...);
extern int fx_sem_clockwait(sem_t *semaphore, clockid_t clock, const struct timespec *deadline) __asm__("sem_clockwait");

pthread_mutex_t fx_static_mutex = PTHREAD_MUTEX_INITIALIZER;
pthread_mutex_t fx_static_recursive = PTHREAD_RECURSIVE_MUTEX_INITIALIZER_NP;
pthread_mutex_t fx_static_errorcheck = PTHREAD_ERRORCHECK_MUTEX_INITIALIZER_NP;
pthread_cond_t fx_static_cond = PTHREAD_COND_INITIALIZER;
pthread_cond_t fx_static_cond_monotonic = PTHREAD_COND_INITIALIZER_MONOTONIC_NP;
pthread_rwlock_t fx_static_rwlock = PTHREAD_RWLOCK_INITIALIZER;
pthread_once_t fx_static_once = PTHREAD_ONCE_INIT;

static __thread long fx_tls_value = 100;

pthread_t fx_last_thread;

struct contention {
    pthread_mutex_t *mutex;
    volatile long *counter;
    int iterations;
};

static void *contend(void *argument)
{
    struct contention *work = argument;
    for (int index = 0; index < work->iterations; index++) {
        pthread_mutex_lock(work->mutex);
        long value = *work->counter;
        if ((index & 7) == 0)
            sched_yield();
        *work->counter = value + 1;
        pthread_mutex_unlock(work->mutex);
    }
    return 0;
}

long fx_mutex_contention(int type, int protocol, int threads, int iterations)
{
    pthread_mutexattr_t attribute;
    pthread_mutex_t mutex;
    pthread_t workers[16];
    volatile long counter = 0;
    struct contention work = {&mutex, &counter, iterations};
    pthread_mutexattr_init(&attribute);
    pthread_mutexattr_settype(&attribute, type);
    pthread_mutexattr_setprotocol(&attribute, protocol);
    pthread_mutex_init(&mutex, &attribute);
    pthread_mutexattr_destroy(&attribute);
    for (int index = 0; index < threads; index++)
        if (pthread_create(&workers[index], 0, contend, &work) != 0)
            return -1;
    for (int index = 0; index < threads; index++)
        pthread_join(workers[index], 0);
    pthread_mutex_destroy(&mutex);
    return counter;
}

long fx_static_mutex_contention(int threads, int iterations)
{
    pthread_t workers[16];
    volatile long counter = 0;
    struct contention work = {&fx_static_mutex, &counter, iterations};
    for (int index = 0; index < threads; index++)
        if (pthread_create(&workers[index], 0, contend, &work) != 0)
            return -1;
    for (int index = 0; index < threads; index++)
        pthread_join(workers[index], 0);
    return counter;
}

struct queue {
    pthread_mutex_t mutex;
    pthread_cond_t not_empty;
    pthread_cond_t not_full;
    int items[4];
    int head;
    int count;
    long sum;
    int total;
};

static void *produce(void *argument)
{
    struct queue *queue = argument;
    for (int value = 1; value <= queue->total; value++) {
        pthread_mutex_lock(&queue->mutex);
        while (queue->count == 4)
            pthread_cond_wait(&queue->not_full, &queue->mutex);
        queue->items[(queue->head + queue->count) % 4] = value;
        queue->count++;
        pthread_cond_signal(&queue->not_empty);
        pthread_mutex_unlock(&queue->mutex);
    }
    return 0;
}

static void *consume(void *argument)
{
    struct queue *queue = argument;
    for (int taken = 0; taken < queue->total; taken++) {
        pthread_mutex_lock(&queue->mutex);
        while (queue->count == 0)
            pthread_cond_wait(&queue->not_empty, &queue->mutex);
        queue->sum += queue->items[queue->head];
        queue->head = (queue->head + 1) % 4;
        queue->count--;
        pthread_cond_signal(&queue->not_full);
        pthread_mutex_unlock(&queue->mutex);
    }
    return 0;
}

long fx_condvar_queue(int total)
{
    struct queue queue;
    pthread_t producer, consumer;
    memset(&queue, 0, sizeof(queue));
    queue.total = total;
    pthread_mutex_init(&queue.mutex, 0);
    pthread_cond_init(&queue.not_empty, 0);
    pthread_cond_init(&queue.not_full, 0);
    pthread_create(&consumer, 0, consume, &queue);
    pthread_create(&producer, 0, produce, &queue);
    pthread_join(producer, 0);
    pthread_join(consumer, 0);
    return queue.sum;
}

struct gate {
    pthread_mutex_t mutex;
    pthread_cond_t cond;
    int open;
    int woken;
    int arrived;
};

static void *wait_at_gate(void *argument)
{
    struct gate *gate = argument;
    pthread_mutex_lock(&gate->mutex);
    gate->arrived++;
    while (!gate->open)
        pthread_cond_wait(&gate->cond, &gate->mutex);
    gate->woken++;
    pthread_mutex_unlock(&gate->mutex);
    return 0;
}

int fx_cond_broadcast_wakes_all(int waiters)
{
    struct gate gate;
    pthread_t threads[16];
    memset(&gate, 0, sizeof(gate));
    pthread_mutex_init(&gate.mutex, 0);
    pthread_cond_init(&gate.cond, 0);
    for (int index = 0; index < waiters; index++)
        pthread_create(&threads[index], 0, wait_at_gate, &gate);
    for (;;) {
        pthread_mutex_lock(&gate.mutex);
        int arrived = gate.arrived;
        pthread_mutex_unlock(&gate.mutex);
        if (arrived == waiters)
            break;
        sched_yield();
    }
    pthread_mutex_lock(&gate.mutex);
    gate.open = 1;
    pthread_cond_broadcast(&gate.cond);
    pthread_mutex_unlock(&gate.mutex);
    for (int index = 0; index < waiters; index++)
        pthread_join(threads[index], 0);
    return gate.woken;
}

int fx_cond_timedwait(int milliseconds, int monotonic)
{
    pthread_mutex_t mutex;
    pthread_cond_t cond;
    pthread_condattr_t attribute;
    struct timespec deadline;
    pthread_mutex_init(&mutex, 0);
    pthread_condattr_init(&attribute);
    if (monotonic)
        pthread_condattr_setclock(&attribute, CLOCK_MONOTONIC);
    pthread_cond_init(&cond, &attribute);
    clock_gettime(monotonic ? CLOCK_MONOTONIC : CLOCK_REALTIME, &deadline);
    deadline.tv_nsec += (long)milliseconds * 1000000L;
    deadline.tv_sec += deadline.tv_nsec / 1000000000L;
    deadline.tv_nsec %= 1000000000L;
    pthread_mutex_lock(&mutex);
    int result = pthread_cond_timedwait(&cond, &mutex, &deadline);
    int relocked = pthread_mutex_trylock(&mutex);
    return result * 1000 + (relocked == EBUSY);
}

static pthread_key_t destructor_key;
static volatile int destructor_calls;
static volatile long destructor_last;

static void count_destructor(void *value)
{
    destructor_calls++;
    destructor_last = (long)value;
}

static void rearm_destructor(void *value)
{
    destructor_calls++;
    pthread_setspecific(destructor_key, value);
}

static void *store_key_value(void *argument)
{
    pthread_setspecific(destructor_key, argument);
    return 0;
}

long fx_key_destructor_runs(void)
{
    pthread_t thread;
    destructor_calls = 0;
    destructor_last = 0;
    pthread_key_create(&destructor_key, count_destructor);
    pthread_create(&thread, 0, store_key_value, (void *)0x1234);
    pthread_join(thread, 0);
    pthread_key_delete(destructor_key);
    return destructor_calls * 100000L + destructor_last;
}

int fx_key_destructor_rounds(void)
{
    pthread_t thread;
    destructor_calls = 0;
    pthread_key_create(&destructor_key, rearm_destructor);
    pthread_create(&thread, 0, store_key_value, (void *)1);
    pthread_join(thread, 0);
    pthread_key_delete(destructor_key);
    return destructor_calls;
}

int fx_key_create_first(void)
{
    pthread_key_t key = 0;
    pthread_key_create(&key, 0);
    pthread_key_delete(key);
    return (int)key;
}

int fx_key_reuse_clears_old_values(void)
{
    pthread_key_t first, second;
    pthread_key_create(&first, 0);
    pthread_setspecific(first, (void *)77);
    pthread_key_delete(first);
    pthread_key_create(&second, 0);
    void *stale = pthread_getspecific(second);
    int same_slot = first == second;
    pthread_key_delete(second);
    return stale == 0 && same_slot;
}

int fx_key_invalid_codes(void)
{
    int setspecific_invalid = pthread_setspecific((pthread_key_t)5, (void *)1);
    int delete_invalid = pthread_key_delete((pthread_key_t)5);
    void *value = pthread_getspecific((pthread_key_t)5);
    return setspecific_invalid * 10000 + delete_invalid * 100 + (value != 0);
}

static pthread_key_t value_key;

static void *check_own_value(void *argument)
{
    pthread_setspecific(value_key, argument);
    for (int round = 0; round < 50; round++)
        sched_yield();
    return pthread_getspecific(value_key) == argument ? (void *)1 : (void *)0;
}

int fx_specific_per_thread(int threads)
{
    pthread_t workers[16];
    void *results[16];
    int good = 0;
    pthread_key_create(&value_key, 0);
    pthread_setspecific(value_key, (void *)999);
    for (int index = 0; index < threads; index++)
        pthread_create(&workers[index], 0, check_own_value, (void *)(long)(index + 1));
    for (int index = 0; index < threads; index++) {
        pthread_join(workers[index], &results[index]);
        good += results[index] != 0;
    }
    good += pthread_getspecific(value_key) == (void *)999 ? 100 : 0;
    pthread_key_delete(value_key);
    return good;
}

static void *bump_tls(void *argument)
{
    fx_tls_value += (long)argument;
    for (int round = 0; round < 20; round++)
        sched_yield();
    return (void *)fx_tls_value;
}

int fx_tls_variable_per_thread(int threads)
{
    pthread_t workers[16];
    void *results[16];
    int good = 0;
    fx_tls_value = 100;
    for (int index = 0; index < threads; index++)
        pthread_create(&workers[index], 0, bump_tls, (void *)(long)(index + 1));
    for (int index = 0; index < threads; index++) {
        pthread_join(workers[index], &results[index]);
        good += (long)results[index] == 101 + index;
    }
    good += fx_tls_value == 100 ? 100 : 0;
    return good;
}

static volatile int once_calls;

static void once_routine(void)
{
    once_calls++;
    for (int round = 0; round < 20; round++)
        sched_yield();
}

static void *once_thread(void *argument)
{
    pthread_once(&fx_static_once, once_routine);
    return (void *)(long)once_calls;
}

int fx_once_race(int threads)
{
    pthread_t workers[16];
    void *seen[16];
    int consistent = 1;
    for (int index = 0; index < threads; index++)
        pthread_create(&workers[index], 0, once_thread, 0);
    for (int index = 0; index < threads; index++) {
        pthread_join(workers[index], &seen[index]);
        consistent &= (long)seen[index] == 1;
    }
    return once_calls * 10 + consistent;
}

int fx_once_state(void)
{
    return *(volatile int *)&fx_static_once;
}

int fx_recursive_depth(int depth)
{
    pthread_mutexattr_t attribute;
    pthread_mutex_t mutex;
    int failures = 0;
    pthread_mutexattr_init(&attribute);
    pthread_mutexattr_settype(&attribute, PTHREAD_MUTEX_RECURSIVE);
    pthread_mutex_init(&mutex, &attribute);
    for (int level = 0; level < depth; level++)
        failures += pthread_mutex_lock(&mutex) != 0;
    for (int level = 0; level < depth; level++)
        failures += pthread_mutex_unlock(&mutex) != 0;
    return failures * 1000 + pthread_mutex_unlock(&mutex);
}

int fx_recursive_overflow_code(void)
{
    int code = 0;
    for (int level = 0; level < 2049; level++)
        code = pthread_mutex_lock(&fx_static_recursive);
    for (int level = 0; level < 2048; level++)
        pthread_mutex_unlock(&fx_static_recursive);
    return code;
}

static void *unlock_foreign(void *argument)
{
    return (void *)(long)pthread_mutex_unlock((pthread_mutex_t *)argument);
}

int fx_errorcheck_codes(void)
{
    void *foreign;
    pthread_t thread;
    int relock = 0, trylock = 0, foreign_unlock = 0;
    pthread_mutex_lock(&fx_static_errorcheck);
    relock = pthread_mutex_lock(&fx_static_errorcheck);
    trylock = pthread_mutex_trylock(&fx_static_errorcheck);
    pthread_create(&thread, 0, unlock_foreign, &fx_static_errorcheck);
    pthread_join(thread, &foreign);
    foreign_unlock = (long)foreign;
    pthread_mutex_unlock(&fx_static_errorcheck);
    return relock * 1000000 + trylock * 1000 + foreign_unlock;
}

int fx_static_mutex_zero_state(void)
{
    int lock = pthread_mutex_lock(&fx_static_mutex);
    int busy = pthread_mutex_trylock(&fx_static_mutex);
    int unlock = pthread_mutex_unlock(&fx_static_mutex);
    return lock * 10000 + busy * 100 + unlock;
}

static void *return_argument(void *argument)
{
    return argument;
}

static void *exit_with_argument(void *argument)
{
    pthread_exit(argument);
    return (void *)-1;
}

long fx_thread_exit_values(void)
{
    pthread_t first, second;
    void *by_return = 0, *by_exit = 0;
    pthread_create(&first, 0, return_argument, (void *)17);
    pthread_create(&second, 0, exit_with_argument, (void *)25);
    pthread_join(first, &by_return);
    pthread_join(second, &by_exit);
    return (long)by_return * 1000 + (long)by_exit;
}

static void exit_from_depth(long depth, long value)
{
    if (depth == 0)
        pthread_exit((void *)value);
    exit_from_depth(depth - 1, value);
}

static void *exit_nested(void *argument)
{
    exit_from_depth(5, (long)argument);
    return 0;
}

long fx_thread_exit_from_nested_frames(void)
{
    pthread_t thread;
    void *value = 0;
    pthread_create(&thread, 0, exit_nested, (void *)91);
    pthread_join(thread, &value);
    return (long)value;
}

static volatile int hold_detached;

static void *wait_for_release(void *argument)
{
    while (!hold_detached)
        sched_yield();
    return 0;
}

int fx_join_detached_code(void)
{
    pthread_attr_t attribute;
    hold_detached = 0;
    pthread_attr_init(&attribute);
    pthread_attr_setdetachstate(&attribute, PTHREAD_CREATE_DETACHED);
    pthread_create(&fx_last_thread, &attribute, wait_for_release, 0);
    pthread_attr_destroy(&attribute);
    int code = pthread_join(fx_last_thread, 0);
    hold_detached = 1;
    return code;
}

int fx_join_self_code(void)
{
    return pthread_join(pthread_self(), 0);
}

static volatile int detached_done;

static void *detached_thread(void *argument)
{
    detached_done = 1;
    return 0;
}

int fx_detached_thread_runs(void)
{
    pthread_attr_t attribute;
    detached_done = 0;
    pthread_attr_init(&attribute);
    pthread_attr_setdetachstate(&attribute, PTHREAD_CREATE_DETACHED);
    pthread_create(&fx_last_thread, &attribute, detached_thread, 0);
    pthread_attr_destroy(&attribute);
    for (int round = 0; round < 200000 && !detached_done; round++)
        sched_yield();
    return detached_done;
}

int fx_detach_after_exit_code(void)
{
    struct timespec pause = {0, 30000000};
    pthread_create(&fx_last_thread, 0, return_argument, 0);
    nanosleep(&pause, 0);
    return pthread_detach(fx_last_thread);
}

static pthread_t *published_thread;

static void *report_published(void *argument)
{
    return (void *)(long)(*published_thread == pthread_self());
}

int fx_thread_out_is_written_before_the_routine_runs(void)
{
    pthread_t thread;
    void *seen = 0;
    published_thread = &thread;
    pthread_create(&thread, 0, report_published, 0);
    pthread_join(thread, &seen);
    return (long)seen;
}

static volatile long attribute_stack_seen;

static void *report_stack(void *argument)
{
    pthread_attr_t attribute;
    void *base;
    size_t size;
    pthread_getattr_np(pthread_self(), &attribute);
    pthread_attr_getstack(&attribute, &base, &size);
    attribute_stack_seen = (long)size;
    return base != 0 ? (void *)1 : 0;
}

long fx_attr_stack_size_seen_by_thread(size_t requested)
{
    pthread_attr_t attribute;
    pthread_t thread;
    void *base_ok = 0;
    pthread_attr_init(&attribute);
    pthread_attr_setstacksize(&attribute, requested);
    pthread_create(&thread, &attribute, report_stack, 0);
    pthread_join(thread, &base_ok);
    pthread_attr_destroy(&attribute);
    return base_ok ? attribute_stack_seen : -1;
}

static volatile int cleanup_log;

static void log_event(int code)
{
    cleanup_log = cleanup_log * 10 + code;
}

static void thread_local_destructor(void *argument)
{
    log_event(1);
}

static void cleanup_handler(void *argument)
{
    log_event(2);
}

static void key_destructor_event(void *argument)
{
    log_event(3);
}

static void *exit_with_all_cleanups(void *argument)
{
    __cxa_thread_atexit_impl(thread_local_destructor, 0, 0);
    pthread_setspecific(destructor_key, (void *)1);
    pthread_cleanup_push(cleanup_handler, 0);
    pthread_exit(0);
    pthread_cleanup_pop(0);
    return 0;
}

int fx_exit_cleanup_order(void)
{
    pthread_t thread;
    cleanup_log = 0;
    pthread_key_create(&destructor_key, key_destructor_event);
    pthread_create(&thread, 0, exit_with_all_cleanups, 0);
    pthread_join(thread, 0);
    pthread_key_delete(destructor_key);
    return cleanup_log;
}

int fx_cleanup_pop_execute_flags(void)
{
    cleanup_log = 0;
    pthread_cleanup_push(cleanup_handler, 0);
    pthread_cleanup_pop(0);
    pthread_cleanup_push(cleanup_handler, 0);
    pthread_cleanup_pop(1);
    return cleanup_log;
}

struct pingpong {
    sem_t ping;
    sem_t pong;
    int rounds;
};

static void *pong_thread(void *argument)
{
    struct pingpong *state = argument;
    for (int round = 0; round < state->rounds; round++) {
        sem_wait(&state->ping);
        sem_post(&state->pong);
    }
    return 0;
}

int fx_sem_ping_pong(int rounds)
{
    struct pingpong state;
    pthread_t thread;
    int completed = 0;
    state.rounds = rounds;
    sem_init(&state.ping, 0, 0);
    sem_init(&state.pong, 0, 0);
    pthread_create(&thread, 0, pong_thread, &state);
    for (int round = 0; round < rounds; round++) {
        sem_post(&state.ping);
        sem_wait(&state.pong);
        completed++;
    }
    pthread_join(thread, 0);
    return completed;
}

int fx_sem_timedwait_errno(void)
{
    sem_t semaphore;
    struct timespec deadline;
    sem_init(&semaphore, 0, 0);
    clock_gettime(CLOCK_REALTIME, &deadline);
    deadline.tv_nsec += 30000000L;
    deadline.tv_sec += deadline.tv_nsec / 1000000000L;
    deadline.tv_nsec %= 1000000000L;
    errno = 0;
    int result = sem_timedwait(&semaphore, &deadline);
    return result == -1 ? errno : 0;
}

int fx_sem_trywait_errno(void)
{
    sem_t semaphore;
    sem_init(&semaphore, 0, 0);
    errno = 0;
    return sem_trywait(&semaphore) == -1 ? errno : 0;
}

int fx_sem_init_over_max_errno(void)
{
    sem_t semaphore;
    errno = 0;
    return sem_init(&semaphore, 0, 0x40000000u) == -1 ? errno : 0;
}

int fx_sem_post_at_max_errno(void)
{
    sem_t semaphore;
    sem_init(&semaphore, 0, 0x3FFFFFFFu);
    errno = 0;
    return sem_post(&semaphore) == -1 ? errno : 0;
}

int fx_sem_clockwait_bad_clock(void)
{
    sem_t semaphore;
    struct timespec deadline = {1, 0};
    errno = 0;
    sem_init(&semaphore, 0, 0);
    return fx_sem_clockwait(&semaphore, CLOCK_BOOTTIME, &deadline) * 1000 + errno;
}

struct rendezvous {
    pthread_barrier_t barrier;
    volatile int serial;
    int cycles;
};

static void *barrier_worker(void *argument)
{
    struct rendezvous *state = argument;
    for (int cycle = 0; cycle < state->cycles; cycle++)
        if (pthread_barrier_wait(&state->barrier) == PTHREAD_BARRIER_SERIAL_THREAD)
            __atomic_add_fetch(&state->serial, 1, __ATOMIC_SEQ_CST);
    return 0;
}

int fx_barrier_serial_count(int threads, int cycles)
{
    struct rendezvous state;
    pthread_t workers[16];
    state.serial = 0;
    state.cycles = cycles;
    pthread_barrier_init(&state.barrier, 0, threads);
    for (int index = 0; index < threads; index++)
        pthread_create(&workers[index], 0, barrier_worker, &state);
    for (int index = 0; index < threads; index++)
        pthread_join(workers[index], 0);
    int destroyed = pthread_barrier_destroy(&state.barrier);
    return state.serial * 10 + destroyed;
}

int fx_barrier_init_zero_code(void)
{
    pthread_barrier_t barrier;
    return pthread_barrier_init(&barrier, 0, 0);
}

int fx_spin_codes(void)
{
    pthread_spinlock_t lock;
    pthread_spin_init(&lock, 0);
    int first = pthread_spin_lock(&lock);
    int busy = pthread_spin_trylock(&lock);
    int unlock = pthread_spin_unlock(&lock);
    int destroy = pthread_spin_destroy(&lock);
    return first * 1000 + busy * 10 + unlock + destroy * 100000;
}

int fx_rwlock_exclusion_codes(void)
{
    int checks = 0;
    checks += pthread_rwlock_rdlock(&fx_static_rwlock) == 0;
    checks += pthread_rwlock_rdlock(&fx_static_rwlock) == 0;
    checks += pthread_rwlock_trywrlock(&fx_static_rwlock) == EBUSY;
    checks += pthread_rwlock_unlock(&fx_static_rwlock) == 0;
    checks += pthread_rwlock_unlock(&fx_static_rwlock) == 0;
    checks += pthread_rwlock_trywrlock(&fx_static_rwlock) == 0;
    checks += pthread_rwlock_tryrdlock(&fx_static_rwlock) == EBUSY;
    checks += pthread_rwlock_rdlock(&fx_static_rwlock) == EDEADLK;
    checks += pthread_rwlock_unlock(&fx_static_rwlock) == 0;
    checks += pthread_rwlock_unlock(&fx_static_rwlock) == EPERM;
    return checks;
}

int fx_rwlock_timedwrlock_code(void)
{
    pthread_rwlock_t lock;
    struct timespec deadline;
    pthread_rwlock_init(&lock, 0);
    pthread_rwlock_rdlock(&lock);
    clock_gettime(CLOCK_REALTIME, &deadline);
    deadline.tv_nsec += 20000000L;
    deadline.tv_sec += deadline.tv_nsec / 1000000000L;
    deadline.tv_nsec %= 1000000000L;
    return pthread_rwlock_timedwrlock(&lock, &deadline);
}

static void *write_lock_once(void *argument)
{
    pthread_rwlock_t *lock = argument;
    pthread_rwlock_wrlock(lock);
    pthread_rwlock_unlock(lock);
    return 0;
}

int fx_rwlock_pending_writer_blocks_new_readers(void)
{
    pthread_rwlockattr_t attribute;
    pthread_rwlock_t lock;
    pthread_t writer;
    pthread_rwlockattr_init(&attribute);
    pthread_rwlockattr_setkind_np(&attribute, PTHREAD_RWLOCK_PREFER_WRITER_NONRECURSIVE_NP);
    pthread_rwlock_init(&lock, &attribute);
    pthread_rwlock_rdlock(&lock);
    pthread_create(&writer, 0, write_lock_once, &lock);
    while (!(*(volatile int *)&lock & 2))
        sched_yield();
    int blocked = pthread_rwlock_tryrdlock(&lock);
    pthread_rwlock_unlock(&lock);
    pthread_join(writer, 0);
    int free_after = pthread_rwlock_tryrdlock(&lock);
    return blocked * 100 + free_after;
}

int fx_rwlock_reader_overflow_code(void)
{
    pthread_rwlock_t lock;
    pthread_rwlock_init(&lock, 0);
    *(volatile int *)&lock = 0x7FFFFFFC;
    return pthread_rwlock_tryrdlock(&lock);
}

static volatile int futex_word;
static volatile int futex_waiter_result;

static void *futex_waiter(void *argument)
{
    futex_waiter_result = (int)syscall(__NR_futex, &futex_word, FUTEX_WAIT_PRIVATE, 0, 0, 0, 0);
    return 0;
}

int fx_futex_wait_then_wake(void)
{
    pthread_t thread;
    futex_word = 0;
    futex_waiter_result = 99;
    pthread_create(&thread, 0, futex_waiter, 0);
    int woken = 0;
    for (int round = 0; round < 1000000 && woken == 0; round++) {
        woken = (int)syscall(__NR_futex, &futex_word, FUTEX_WAKE_PRIVATE, 1, 0, 0, 0);
        sched_yield();
    }
    pthread_join(thread, 0);
    return futex_waiter_result * 10 + woken;
}

int fx_futex_wait_mismatch_errno(void)
{
    futex_word = 5;
    errno = 0;
    long result = syscall(__NR_futex, &futex_word, FUTEX_WAIT_PRIVATE, 0, 0, 0, 0);
    return result == -1 ? errno : 0;
}

int fx_futex_wait_timeout_errno(int milliseconds)
{
    struct timespec relative = {0, (long)milliseconds * 1000000L};
    futex_word = 0;
    errno = 0;
    long result = syscall(__NR_futex, &futex_word, FUTEX_WAIT_PRIVATE, 0, &relative, 0, 0);
    return result == -1 ? errno : 0;
}

long fx_gettid_via_syscall(void)
{
    return syscall(__NR_gettid) * 1000000L + gettid();
}

static void *name_thread(void *argument)
{
    char name[16];
    pthread_setname_np(pthread_self(), "fx-worker");
    pthread_getname_np(pthread_self(), name, sizeof(name));
    return (void *)(long)(strcmp(name, "fx-worker") == 0);
}

int fx_thread_name_round_trip(void)
{
    pthread_t thread;
    void *same = 0;
    pthread_create(&thread, 0, name_thread, 0);
    pthread_join(thread, &same);
    return (long)same;
}

int fx_detach_running_thread_code(void)
{
    pthread_create(&fx_last_thread, 0, return_argument, 0);
    return pthread_detach(fx_last_thread);
}
