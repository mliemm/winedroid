use std::sync::mpsc;
use std::time::Duration;

const LIMIT: Duration = Duration::from_secs(30);
const WORKER_STACK: usize = 2 << 20;

pub fn run(name: &'static str, work: impl FnOnce() + Send + 'static) {
    run_within(name, LIMIT, work);
}

pub fn run_within(name: &'static str, limit: Duration, work: impl FnOnce() + Send + 'static) {
    let (done, finished) = mpsc::channel();
    let worker = std::thread::Builder::new()
        .name(name.to_string())
        .stack_size(WORKER_STACK)
        .spawn(move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work));
            let _ = done.send(outcome);
        })
        .expect("deadline worker thread");
    match finished.recv_timeout(limit) {
        Ok(Ok(())) => {
            let _ = worker.join();
        }
        Ok(Err(panic)) => std::panic::resume_unwind(panic),
        Err(_) => panic!("{name} did not finish within {limit:?}"),
    }
}

macro_rules! bounded_test {
    (fn $name:ident() $body:block) => {
        #[test]
        fn $name() {
            $crate::native::test_deadline::run(stringify!($name), || $body)
        }
    };
}

pub(crate) use bounded_test;

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use super::*;

    #[test]
    fn a_body_that_outlives_the_limit_fails_the_test_instead_of_hanging_it() {
        // cite: libcore FutureTask.java#get
        let release = Arc::new(AtomicBool::new(false));
        let waiting = release.clone();
        let outcome = std::panic::catch_unwind(|| {
            run_within("stuck_body", Duration::from_millis(100), move || {
                while !waiting.load(Ordering::SeqCst) {
                    std::thread::yield_now();
                }
            })
        });
        release.store(true, Ordering::SeqCst);

        let message = *outcome
            .expect_err("a stuck body must fail")
            .downcast::<String>()
            .unwrap();
        assert!(
            message.contains("stuck_body did not finish within"),
            "{message}"
        );
    }

    #[test]
    fn a_panic_inside_the_body_reaches_the_test_with_its_own_message() {
        // cite: libcore FutureTask.java#report
        let outcome = std::panic::catch_unwind(|| {
            run_within("panicking_body", Duration::from_secs(5), || {
                panic!("the oracle disagreed")
            })
        });

        let message = *outcome
            .expect_err("the panic must propagate")
            .downcast::<&str>()
            .unwrap();
        assert_eq!(message, "the oracle disagreed");
    }

    #[test]
    fn a_body_that_finishes_in_time_returns_normally() {
        // cite: libcore FutureTask.java#get
        let finished = Arc::new(AtomicBool::new(false));
        let mark = finished.clone();

        run_within("quick_body", Duration::from_secs(5), move || {
            mark.store(true, Ordering::SeqCst)
        });

        assert!(finished.load(Ordering::SeqCst));
    }
}
