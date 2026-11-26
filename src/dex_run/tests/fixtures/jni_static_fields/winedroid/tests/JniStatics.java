package winedroid.tests;

public final class JniStatics extends JniStaticsBase implements JniStaticsConstants {
    public static boolean flag = true;
    public static byte small = -2;
    public static char letter = 'w';
    public static short half = -300;
    public static int count = 7;
    public static long big = 0x100000002L;
    public static float ratio = 1.5f;
    public static double precise;
    public static String text = "dex";
    public static int[] numbers;
    public int instance;
}

class JniStaticsBase {
    static String inherited;

    static {
        inherited = "base";
    }
}

interface JniStaticsConstants {
    int LIMIT = 42;
}
