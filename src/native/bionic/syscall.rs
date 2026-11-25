use super::super::futex_syscall::futex_syscall;
use super::super::host_log;
use super::current_thread;
use super::export::in_libc;

const NR_SET_TID_ADDRESS: i64 = 96;
const NR_FUTEX: i64 = 98;
const NR_SCHED_YIELD: i64 = 124;
const NR_TGKILL: i64 = 131;
const NR_GETPID: i64 = 172;
const NR_GETTID: i64 = 178;
const NR_MEMBARRIER: i64 = 283;
const NR_RSEQ: i64 = 293;
const ENOSYS: i64 = 38;
const MAX_ERRNO: i64 = 4095;

pub fn translate(number: i64, arguments: [u64; 6]) -> i64 {
    // cite: bionic unistd.h#__NR_futex, __NR_gettid, __NR_tgkill, __NR_rseq
    match number {
        NR_FUTEX => futex_syscall(arguments),
        NR_GETTID | NR_SET_TID_ADDRESS => i64::from(current_thread::tid()),
        NR_GETPID => i64::from(unsafe { libc::getpid() }),
        NR_SCHED_YIELD => i64::from(unsafe { libc::sched_yield() }),
        NR_RSEQ | NR_MEMBARRIER => -ENOSYS,
        NR_TGKILL => -ENOSYS,
        other => {
            host_log::line(&format!("FIXME: syscall {other} is not translated; next missing primitive: syscall {other}"));
            -ENOSYS
        }
    }
}

pub fn register() {
    in_libc("syscall", |context| {
        // cite: bionic syscall.S#syscall
        let arguments = [context.arg_u64(1), context.arg_u64(2), context.arg_u64(3), context.arg_u64(4), context.arg_u64(5), context.arg_u64(6)];
        let result = translate(context.arg_u64(0) as i64, arguments);
        if (-MAX_ERRNO..0).contains(&result) {
            context.set_errno((-result) as i32);
            context.ret_i64(-1);
        } else {
            context.ret_i64(result);
        }
    });
}
