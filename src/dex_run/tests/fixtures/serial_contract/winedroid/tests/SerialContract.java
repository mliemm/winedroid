package winedroid.tests;

import java.io.InputStream;
import java.io.ObjectInputStream;
import java.io.ObjectOutputStream;
import java.io.OutputStream;
import java.io.Serializable;

public class SerialContract {
    public static class Cached implements Serializable {
        private static final long serialVersionUID = 1L;
        public long mCreateTime;
        public String mValue;
    }

    public static class Base {
        public int baseValue;

        public Base() {
            baseValue = 41;
        }
    }

    public static class Child extends Base implements Serializable {
        private static final long serialVersionUID = -7L;
        public boolean flag;
        public char letter;
        public double ratio;
        public short small;
        public transient int skipped;
        public String first;
        public String second;
        public Cached nested;
        public Object nothing;
    }

    static class Collector extends OutputStream {
        byte[] data = new byte[0];

        @Override
        public void write(int value) {
            write(new byte[] {(byte) value}, 0, 1);
        }

        @Override
        public void write(byte[] buffer, int offset, int length) {
            byte[] grown = new byte[data.length + length];
            System.arraycopy(data, 0, grown, 0, data.length);
            System.arraycopy(buffer, offset, grown, data.length, length);
            data = grown;
        }
    }

    static class Feeder extends InputStream {
        final byte[] data;
        int position;

        Feeder(byte[] data) {
            this.data = data;
        }

        @Override
        public int read() {
            return position < data.length ? data[position++] & 0xff : -1;
        }

        @Override
        public int read(byte[] buffer, int offset, int length) {
            if (position >= data.length) {
                return -1;
            }
            int count = Math.min(length, data.length - position);
            System.arraycopy(data, position, buffer, offset, count);
            position += count;
            return count;
        }
    }

    public static byte[] writeGraph() throws Exception {
        Cached cached = new Cached();
        cached.mCreateTime = 1234567890123L;
        cached.mValue = "café";
        Child child = new Child();
        child.baseValue = 7;
        child.flag = true;
        child.letter = 'Z';
        child.ratio = 0.5;
        child.small = -3;
        child.skipped = 99;
        child.first = "shared";
        child.second = "shared";
        child.nested = cached;
        Collector collector = new Collector();
        ObjectOutputStream out = new ObjectOutputStream(collector);
        out.writeObject(child);
        out.writeObject(cached);
        out.writeObject(null);
        out.close();
        return collector.data;
    }

    public static String readGraph(byte[] data) throws Exception {
        ObjectInputStream in = new ObjectInputStream(new Feeder(data));
        Child child = (Child) in.readObject();
        Cached again = (Cached) in.readObject();
        Object nothing = in.readObject();
        in.close();
        return child.baseValue + "," + child.flag + "," + child.letter + "," + child.ratio + ","
                + child.small + "," + child.skipped + "," + child.first + "," + child.second + ","
                + child.nested.mCreateTime + "," + child.nested.mValue + ","
                + (again == child.nested) + "," + child.nothing + "," + nothing;
    }

    public static void main(String[] arguments) throws Exception {
        byte[] data = writeGraph();
        java.nio.file.Files.write(java.nio.file.Paths.get(arguments[0]), data);
        System.out.println(readGraph(data));
    }
}
