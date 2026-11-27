package fx;

public class Native {
    public static native int registeredAdd(int left, int right);

    public static native long callStaticLong(long value);

    public static native void callStaticVoid(int value);

    public static native int unbound(String name, int[] values);

    public static int recorded;

    public static void record(int value) {
        recorded = value;
    }

    public static long scale(long value, int factor) {
        return value * factor + 1;
    }
}
