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
}
