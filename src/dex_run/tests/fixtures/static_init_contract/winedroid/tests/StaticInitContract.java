package winedroid.tests;

public class StaticInitContract {
    static String log = "";

    static class Target {
        static {
            log += "clinit;";
        }

        static int code() {
            return 7;
        }
    }

    public static String staticCodeCallInitializesFirst() {
        log = "";
        int value = Target.code();
        return log + value;
    }

    static class Broken {
        static int value = fail();

        static int fail() {
            throw new IllegalStateException("boom");
        }

        static int code() {
            return value;
        }
    }

    public static String failedInitializerThenNoClassDefFound() {
        String result = "";
        try {
            Broken.code();
        } catch (ExceptionInInitializerError first) {
            result += "eiie:" + first.getCause().getMessage() + ";";
        }
        try {
            Broken.code();
        } catch (NoClassDefFoundError second) {
            result += "ncdfe:" + second.getMessage() + ";"
                    + (second.getCause() instanceof ExceptionInInitializerError);
        }
        return result;
    }
}
