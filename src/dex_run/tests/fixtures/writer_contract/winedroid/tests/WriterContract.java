package winedroid.tests;

import java.io.Writer;

public final class WriterContract {
    static final class Buffer extends Writer {
        final StringBuilder text = new StringBuilder();

        @Override
        public void write(char[] buffer, int offset, int length) {
            text.append(buffer, offset, length);
        }

        @Override
        public void flush() {}

        @Override
        public void close() {}
    }

    public static int writeString() throws Exception {
        Buffer buffer = new Buffer();
        buffer.write("hello");
        return "hello".equals(buffer.text.toString()) ? 1 : -1;
    }
}
