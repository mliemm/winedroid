package winedroid.tests;

import java.io.EOFException;
import java.io.IOException;
import java.io.InputStream;
import java.io.ObjectInputStream;
import java.io.ObjectOutputStream;
import java.io.Serializable;
import java.util.Arrays;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.Map;

public class SerialMaps {
    public static class AppInfoLike implements Serializable {
        private static final long serialVersionUID = 1L;
        public Map<String, Object> mAppExtension;
        public int mAppVersionCode;
        public String mAppVersionName;
        public long mLastUpdateTime;
        public String mPackageName;
    }

    static class ThrowingKey {
        @Override
        public int hashCode() {
            throw new IllegalStateException("hashCode");
        }
    }

    static class FailingFeeder extends InputStream {
        final byte[] data;
        final int failAt;
        int position;

        FailingFeeder(byte[] data, int failAt) {
            this.data = data;
            this.failAt = failAt;
        }

        @Override
        public int read() throws IOException {
            byte[] one = new byte[1];
            return read(one, 0, 1) < 0 ? -1 : one[0] & 0xff;
        }

        @Override
        public int read(byte[] buffer, int offset, int length) throws IOException {
            if (position >= failAt) {
                throw new IOException("socket closed");
            }
            if (position >= data.length) {
                return -1;
            }
            int count = Math.min(length, Math.min(data.length, failAt) - position);
            System.arraycopy(data, position, buffer, offset, count);
            position += count;
            return count;
        }
    }

    static final int COUNT = 11;

    static final String[] SMALL_KEYS = {"small0", "small1", "small2", "small3", "small4"};
    static final String[] SMALL_VALUES = {"v0", "v1", "v2", "v3", "v4"};
    static final String[] GROWN_KEYS = {"key0", "key7", "key14", "key21", "key28", "key35", "key42", "key49", "key56", "key63", "key70", "key77"};
    static final String[] GROWN_VALUES = {"value0", "value1", "value2", "value3", "value4", "value5", "value6", "value7", "value8", "value9", "value10", "value11"};
    static final String[] COLLIDING_KEYS = {"AaAaAaAa", "BBAaAaAa", "AaBBAaAa", "BBBBAaAa", "AaAaBBAa", "BBAaBBAa", "AaBBBBAa", "BBBBBBAa", "AaAaAaBB", "BBAaAaBB", "AaBBAaBB"};
    static final String[] COLLIDING_VALUES = {"c0", "c1", "c2", "c3", "c4", "c5", "c6", "c7", "c8", "c9", "c10"};
    static final String[] LOOSE_KEYS = {"loose0", "loose1", "loose2", "loose3", "loose4"};
    static final String[] LOOSE_VALUES = {"l0", "l1", "l2", "l3", "l4"};

    static HashMap<String, String> build(int which) {
        HashMap<String, String> map;
        switch (which) {
            case 0:
                return new HashMap<String, String>();
            case 1:
                return new HashMap<String, String>(10);
            case 2:
                map = new HashMap<String, String>(3);
                for (int i = 0; i < 5; i++) {
                    map.put(SMALL_KEYS[i], SMALL_VALUES[i]);
                }
                return map;
            case 3:
                map = new HashMap<String, String>();
                map.put("BB", "second");
                map.put("Aa", "first");
                for (int i = 0; i < 12; i++) {
                    map.put(GROWN_KEYS[i], GROWN_VALUES[i]);
                }
                return map;
            case 4:
                map = new HashMap<String, String>();
                for (int i = 0; i < 10; i++) {
                    map.put(COLLIDING_KEYS[i], COLLIDING_VALUES[i]);
                }
                return map;
            case 5:
                map = new HashMap<String, String>();
                map.put("Aa", "1");
                map.put("BB", "2");
                map.put("C", "3");
                map.remove("Aa");
                map.put("Aa", "4");
                map.put("BB", "5");
                return map;
            case 6:
                map = build(3);
                map.clear();
                map.put("after", "clear");
                return map;
            case 7:
                return new HashMap<String, String>(build(3));
            case 8:
                map = new HashMap<String, String>(4, 0.5f);
                for (int i = 0; i < 5; i++) {
                    map.put(LOOSE_KEYS[i], LOOSE_VALUES[i]);
                }
                return map;
            case 9:
                map = new HashMap<String, String>();
                map.put("k", null);
                map.put(null, "n");
                map.put("Aa", "a");
                return map;
            default:
                map = new HashMap<String, String>();
                map.put("release_build", "f218af1_20261119_102455_82221324");
                map.put("jekins_number", "null");
                map.put("jekins_name", "DONT-DELETE");
                map.put("jekins_url", "null");
                map.put("branch_name", "null");
                map.put("complete_cid", "null");
                return map;
        }
    }

    static AppInfoLike appInfo() {
        AppInfoLike info = new AppInfoLike();
        HashMap<String, Object> extension = new HashMap<String, Object>();
        for (Map.Entry<String, String> entry : build(COUNT - 1).entrySet()) {
            extension.put(entry.getKey(), entry.getValue());
        }
        info.mAppExtension = extension;
        info.mAppVersionCode = 470903;
        info.mAppVersionName = "47.9.3";
        info.mLastUpdateTime = 1764300000000L;
        info.mPackageName = "com.example.app";
        return info;
    }

    static String describe(Map<?, ?> map) {
        StringBuilder out = new StringBuilder();
        for (Object key : map.keySet()) {
            out.append(key).append(',');
        }
        out.append('|');
        for (Object value : map.values()) {
            out.append(value).append(',');
        }
        out.append('|');
        for (Map.Entry<?, ?> entry : map.entrySet()) {
            out.append(entry.getKey()).append('=').append(entry.getValue()).append(',');
        }
        return out.toString();
    }

    public static String describeMaps() {
        StringBuilder out = new StringBuilder();
        for (int which = 0; which < COUNT; which++) {
            out.append(describe(build(which))).append(';');
        }
        return out.toString();
    }

    public static byte[] writeMaps() throws Exception {
        SerialContract.Collector bytes = new SerialContract.Collector();
        ObjectOutputStream out = new ObjectOutputStream(bytes);
        HashMap<String, String> shared = build(5);
        for (int which = 0; which < COUNT; which++) {
            out.writeObject(build(which));
        }
        out.writeObject(shared);
        out.writeObject(shared);
        out.writeObject(appInfo());
        out.close();
        return bytes.data;
    }

    public static String readMaps(byte[] data) throws Exception {
        ObjectInputStream in = new ObjectInputStream(new SerialContract.Feeder(data));
        StringBuilder out = new StringBuilder();
        for (int which = 0; which < COUNT; which++) {
            out.append(describe((Map<?, ?>) in.readObject())).append(';');
        }
        Object first = in.readObject();
        out.append(first == in.readObject()).append(';');
        AppInfoLike info = (AppInfoLike) in.readObject();
        out.append(describe(info.mAppExtension)).append(info.mAppVersionCode).append(',');
        out.append(info.mAppVersionName).append(',').append(info.mLastUpdateTime).append(',');
        out.append(info.mPackageName);
        in.close();
        return out.toString();
    }

    public static byte[] rewriteMaps(byte[] data) throws Exception {
        ObjectInputStream in = new ObjectInputStream(new SerialContract.Feeder(data));
        SerialContract.Collector bytes = new SerialContract.Collector();
        ObjectOutputStream out = new ObjectOutputStream(bytes);
        for (int which = 0; which < COUNT + 3; which++) {
            out.writeObject(in.readObject());
        }
        in.close();
        out.close();
        return bytes.data;
    }

    public static String treeBinKeys() {
        HashMap<String, String> map = new HashMap<String, String>();
        for (int i = 0; i < 11; i++) {
            map.put(COLLIDING_KEYS[i], COLLIDING_VALUES[i]);
        }
        return describe(map);
    }

    public static String identityKeys() {
        HashMap<Object, String> map = new HashMap<Object, String>();
        AppInfoLike first = new AppInfoLike();
        AppInfoLike second = new AppInfoLike();
        AppInfoLike third = new AppInfoLike();
        map.put(first, "first");
        map.put(second, "second");
        map.put(third, "third");
        StringBuilder out = new StringBuilder();
        for (String value : map.values()) {
            out.append(value).append(',');
        }
        out.append('#').append(first.hashCode()).append(',').append(second.hashCode());
        out.append(',').append(third.hashCode());
        return out.toString();
    }

    public static String boxedKeys() {
        HashMap<Object, String> map = new HashMap<Object, String>();
        map.put(Integer.valueOf(17), "int17");
        map.put(Integer.valueOf(-1), "intMinus1");
        map.put(Integer.valueOf(100000), "int100000");
        map.put(Long.valueOf(5L), "long5");
        map.put(Long.valueOf(1L << 33), "long2pow33");
        map.put(Boolean.TRUE, "true");
        map.put(Boolean.valueOf(false), "false");
        map.put(Character.valueOf('x'), "charX");
        map.put(Float.valueOf(1.5f), "float1.5");
        map.put(Double.valueOf(2.5), "double2.5");
        StringBuilder out = new StringBuilder();
        for (String value : map.values()) {
            out.append(value).append(',');
        }
        return out.toString();
    }

    public static String listKeys() {
        HashMap<Object, String> map = new HashMap<Object, String>();
        map.put(new java.util.ArrayList<String>(), "first");
        map.put("second", "second");
        return describe(map);
    }

    public static void writeTreeBin() throws Exception {
        HashMap<String, String> map = new HashMap<String, String>();
        for (int i = 0; i < 11; i++) {
            map.put(COLLIDING_KEYS[i], COLLIDING_VALUES[i]);
        }
        new ObjectOutputStream(new SerialContract.Collector()).writeObject(map);
    }

    public static void writeList() throws Exception {
        new ObjectOutputStream(new SerialContract.Collector())
                .writeObject(new java.util.ArrayList<String>());
    }

    public static String linkedCopy() {
        LinkedHashMap<String, String> linked = new LinkedHashMap<String, String>();
        linked.put("b", "2");
        linked.put("a", "1");
        HashMap<String, String> added = new HashMap<String, String>();
        added.put("c", "3");
        added.putAll(linked);
        return describe(new HashMap<String, String>(linked)) + ";" + describe(added);
    }

    public static String linkedCollidingCopy() {
        LinkedHashMap<String, String> linked = new LinkedHashMap<String, String>();
        linked.put("BB", "2");
        linked.put("Aa", "1");
        return describe(new HashMap<String, String>(linked));
    }

    public static String emptiedTreeBins() {
        HashMap<Object, String> cleared = new HashMap<Object, String>();
        HashMap<Object, String> removed = new HashMap<Object, String>();
        for (int i = 0; i < 11; i++) {
            cleared.put(COLLIDING_KEYS[i], COLLIDING_VALUES[i]);
            removed.put(COLLIDING_KEYS[i], COLLIDING_VALUES[i]);
        }
        cleared.clear();
        for (int i = 0; i < 11; i++) {
            removed.remove(COLLIDING_KEYS[i]);
        }
        StringBuilder out = new StringBuilder();
        for (HashMap<Object, String> map : Arrays.asList(cleared, removed)) {
            map.put(Integer.valueOf(16), "sixteen");
            map.put(Integer.valueOf(5), "five");
            out.append(describe(map)).append(';');
        }
        return out.toString();
    }

    public static String throwingHashCode() {
        HashMap<Object, String> map = new HashMap<Object, String>();
        map.put("a", "1");
        String caught = "none";
        try {
            map.put(new ThrowingKey(), "v");
        } catch (IllegalStateException e) {
            caught = e.getMessage();
        }
        return caught + ";" + map.size() + ";" + describe(map);
    }

    public static String classKeys() {
        HashMap<Object, String> map = new HashMap<Object, String>();
        Object[] keys = {String.class, Integer.class, HashMap.class};
        map.put(keys[0], "first");
        map.put(keys[1], "second");
        map.put(keys[2], "third");
        StringBuilder out = new StringBuilder();
        for (String value : map.values()) {
            out.append(value).append(',');
        }
        out.append('#').append(keys[0].hashCode()).append(',').append(keys[1].hashCode());
        out.append(',').append(keys[2].hashCode());
        return out.toString();
    }

    static byte[] splice(byte[] data, int at, int removed, int... inserted) {
        byte[] out = new byte[data.length - removed + inserted.length];
        System.arraycopy(data, 0, out, 0, at);
        for (int i = 0; i < inserted.length; i++) {
            out[at + i] = (byte) inserted[i];
        }
        System.arraycopy(data, at + removed, out, at + inserted.length, data.length - at - removed);
        return out;
    }

    static String readFailure(byte[] data, int failAt) {
        try {
            ObjectInputStream in = new ObjectInputStream(new FailingFeeder(data, failAt));
            Object first;
            try {
                first = in.readObject();
            } catch (EOFException e) {
                try {
                    return "EOFException then " + in.readObject();
                } catch (Exception again) {
                    return "EOFException then " + again.getClass().getName() + ": " + again.getMessage();
                }
            }
            return "read " + describe((Map<?, ?>) first) + " " + describe((Map<?, ?>) in.readObject());
        } catch (Exception e) {
            return e.getClass().getName() + ": " + e.getMessage();
        }
    }

    public static String blockDataFailures(byte[] data) {
        int none = Integer.MAX_VALUE;
        String[] results = {
            readFailure(splice(data, 71, 1, 0x90), none),
            readFailure(splice(data, 71, 1, 0x70), none),
            readFailure(splice(data, 71, data.length - 71), none),
            readFailure(splice(data, 72, data.length - 72), none),
            readFailure(splice(data, 76, data.length - 76), none),
            readFailure(data, 71),
            readFailure(data, 72),
            readFailure(data, 75),
            readFailure(splice(data, 71, 0, 0x79), none),
            readFailure(splice(data, 81, 0, 0x79), none),
            readFailure(splice(data, 82, 0, 0x79), none),
            readFailure(splice(data, 0, 0), none),
        };
        StringBuilder out = new StringBuilder();
        for (String result : results) {
            out.append(result).append(';');
        }
        return out.toString();
    }

    public static void main(String[] arguments) throws Exception {
        byte[] data = writeMaps();
        java.nio.file.Files.write(java.nio.file.Paths.get(arguments[0]), data);
        java.nio.file.Files.write(java.nio.file.Paths.get(arguments[1]), rewriteMaps(data));
        System.out.println(describeMaps());
        System.out.println(readMaps(data));
        System.out.println(treeBinKeys());
        System.out.println(boxedKeys());
        System.out.println(linkedCopy());
        System.out.println(linkedCollidingCopy());
        System.out.println(emptiedTreeBins());
        System.out.println(throwingHashCode());
        System.out.println(blockDataFailures(data));
    }
}
