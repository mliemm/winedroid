package fx;

public class Native {
    public static native int registeredAdd(int left, int right);

    public static native long callStaticLong(long value);

    public static native void callStaticVoid(int value);

    public static native int unbound(String name, int[] values);

    public static native byte[] xorBytes(byte[] input, int key);

    public static native void overflowRegion(byte[] output);

    public static native void incrementInPlace(byte[] data);

    public static native int stringLength(String text);

    public static native void pinBytes(byte[] data);

    public static native void bumpPinnedAndRelease(byte[] data);

    public static int recorded;

    public static void record(int value) {
        recorded = value;
    }

    public static long scale(long value, int factor) {
        return value * factor + 1;
    }
}
