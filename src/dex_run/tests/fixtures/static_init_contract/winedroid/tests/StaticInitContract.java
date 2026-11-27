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
}
