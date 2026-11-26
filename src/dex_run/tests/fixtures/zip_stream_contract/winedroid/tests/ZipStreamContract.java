package winedroid.tests;

import java.io.FileInputStream;
import java.io.IOException;
import java.util.zip.CRC32;
import java.util.zip.CheckedInputStream;
import java.util.zip.Checksum;

public final class ZipStreamContract {
    static final class CappedStream extends FileInputStream {
        int calls;

        CappedStream(String path) throws IOException {
            super(path);
        }

        @Override
        public int read(byte[] buffer, int offset, int length) throws IOException {
            calls++;
            return super.read(buffer, offset, Math.min(length, 2));
        }

        @Override
        public void close() throws IOException {
            calls += 100;
            super.close();
        }
    }

    static final class SummingChecksum implements Checksum {
        private long sum;

        public void update(int value) {
            sum += value & 0xff;
        }

        public void update(byte[] buffer, int offset, int length) {
            for (int index = offset; index < offset + length; index++) {
                update(buffer[index]);
            }
        }

        public long getValue() {
            return sum;
        }

        public void reset() {
            sum = 0;
        }
    }

    public static long checksumThroughOverriddenStream(String path) throws IOException {
        CappedStream source = new CappedStream(path);
        CheckedInputStream checked = new CheckedInputStream(source, new CRC32());
        byte[] buffer = new byte[8];
        while (checked.read(buffer) >= 0) {
        }
        checked.close();
        return checked.getChecksum().getValue() * 1000 + source.calls;
    }

    public static long guestChecksum(String path) throws IOException {
        CheckedInputStream checked =
                new CheckedInputStream(new FileInputStream(path), new SummingChecksum());
        checked.read(new byte[4]);
        while (checked.read() != -1) {
        }
        return checked.getChecksum().getValue();
    }
}
