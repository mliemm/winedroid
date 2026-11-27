package winedroid.tests;

import java.util.concurrent.Callable;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.FutureTask;

public class FutureContract extends FutureTask<String> {
    public static String log = "";

    public FutureContract(Callable<String> callable) {
        super(callable);
    }

    @Override
    public void run() {
        log += "run;";
        super.run();
        log += "ran;";
    }

    @Override
    protected void set(String value) {
        log += "set:" + value + ";";
        super.set(value);
    }

    @Override
    protected void setException(Throwable failure) {
        log += "ex:" + failure.getMessage() + ";";
        super.setException(failure);
    }

    @Override
    protected void done() {
        log += "done;";
    }

    public static String runValue() throws Exception {
        log = "";
        FutureContract task = new FutureContract(new Callable<String>() {
            @Override
            public String call() {
                return "value";
            }
        });
        task.run();
        task.run();
        return log + task.get();
    }

    public static String runFailure() throws Exception {
        log = "";
        FutureContract task = new FutureContract(new Callable<String>() {
            @Override
            public String call() {
                throw new IllegalStateException("boom");
            }
        });
        task.run();
        try {
            task.get();
        } catch (ExecutionException failure) {
            return log + failure.getCause().getMessage();
        }
        return log + "no failure";
    }

    static class Plain extends FutureTask<String> {
        Plain(Callable<String> callable) {
            super(callable);
        }

        @Override
        protected void done() {
            log += "done;";
        }
    }

    static class RunnableTask extends FutureTask<String> {
        RunnableTask(Runnable runnable, String value) {
            super(runnable, value);
        }

        @Override
        protected void setException(Throwable failure) {
            log += "ex:" + failure.getMessage() + ";";
            super.setException(failure);
        }
    }

    public static String runnableFailure() throws Exception {
        log = "";
        RunnableTask task = new RunnableTask(new Runnable() {
            @Override
            public void run() {
                throw new IllegalStateException("bad");
            }
        }, "v");
        task.run();
        try {
            task.get();
        } catch (ExecutionException failure) {
            return log + failure.getCause().getMessage();
        }
        return log + "no failure";
    }

    public static String cancelRunsDoneOnce() {
        Plain task = plainTask();
        boolean first = task.cancel(false);
        boolean second = task.cancel(false);
        return log + first + ":" + second + ":" + task.isCancelled();
    }

    public static Plain plainTask() {
        log = "";
        return new Plain(new Callable<String>() {
            @Override
            public String call() {
                log += "call;";
                return "v";
            }
        });
    }

    public static String log() {
        return log;
    }
}
