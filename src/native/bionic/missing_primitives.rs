use super::abi_types::EINVAL;
use super::export::in_libc;
use super::fs_result::missing;

fn unavailable(context: &mut super::super::bridge::CallContext, name: &str, reason: &str) {
    missing(name, reason);
    context.set_errno(EINVAL);
    context.ret_i32(-1);
}

pub fn register() {
    for name in ["inotify_init", "inotify_init1", "inotify_add_watch", "inotify_rm_watch"] {
        in_libc(name, move |context| unavailable(context, name, "Darwin kqueue FSEvents has no inotify watch descriptor namespace"));
    }
    for name in ["signalfd", "signalfd4"] {
        in_libc(name, move |context| unavailable(context, name, "Darwin has no file descriptor that delivers queued signals"));
    }
    for name in ["fanotify_init", "fanotify_mark"] {
        in_libc(name, move |context| unavailable(context, name, "Darwin has no fanotify equivalent"));
    }
    for name in ["perf_event_open", "bpf"] {
        in_libc(name, move |context| unavailable(context, name, "Darwin exposes neither perf events nor the eBPF syscall"));
    }
}
