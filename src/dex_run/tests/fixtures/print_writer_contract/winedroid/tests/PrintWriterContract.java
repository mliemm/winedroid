package winedroid.tests;

import java.io.PrintWriter;
import java.io.Writer;

public class PrintWriterContract {
    static class StringOverride extends Writer {
        final StringBuilder builder = new StringBuilder();
        int closes;

        @Override
        public void write(String value) {
            builder.append("s[").append(value).append("]");
        }

        @Override
        public void write(char[] buffer, int offset, int length) {
            builder.append("c[").append(buffer, offset, length).append("]");
        }

        @Override
        public void flush() {
        }

        @Override
        public void close() {
            closes++;
        }
    }

    static class CharsOnly extends Writer {
        final StringBuilder builder = new StringBuilder();

        @Override
        public void write(char[] buffer, int offset, int length) {
            builder.append(buffer, offset, length);
        }

        @Override
        public void flush() {
        }

        @Override
        public void close() {
        }
    }

    public static String printlnUsesWriteStringThenCloseOnce() {
        StringOverride target = new StringOverride();
        PrintWriter out = new PrintWriter(target);
        out.println();
        out.close();
        out.close();
        out.println();
        return target.builder.toString().replace("\n", "\\n") + ":" + target.closes;
    }

    public static String printlnFallsBackToCharArrayWrite() {
        CharsOnly target = new CharsOnly();
        PrintWriter out = new PrintWriter(target);
        out.println();
        out.close();
        return target.builder.toString().replace("\n", "\\n");
    }
}
