package winedroid.tests;

import android.os.FileObserver;
import java.io.BufferedWriter;
import java.io.File;
import java.io.FileOutputStream;
import java.io.FileWriter;
import java.io.IOException;
import java.io.RandomAccessFile;

public final class FileObserverContract extends FileObserver {
    static final Object lock = new Object();

    private final StringBuilder events = new StringBuilder();
    private int throwsLeft;
    private int count;
    private int depth;
    private int maxDepth;
    private String reenterPath;

    public FileObserverContract(String path, int mask, int throwsLeft) {
        super(path, mask);
        this.throwsLeft = throwsLeft;
    }

    @Override
    public void onEvent(int event, String path) {
        depth++;
        if (depth > maxDepth) {
            maxDepth = depth;
        }
        events.append(event).append(',').append(path).append(',')
                .append(Thread.currentThread().getName()).append(';');
        synchronized (lock) {
            count++;
            lock.notifyAll();
        }
        if (reenterPath != null) {
            String next = reenterPath;
            reenterPath = null;
            try {
                writeStream(next);
                synchronized (lock) {
                    lock.wait(1);
                }
            } catch (IOException | InterruptedException error) {
                throw new IllegalStateException(error);
            }
        }
        depth -= 1;
        if (throwsLeft > 0) {
            throwsLeft -= 1;
            throw new IllegalStateException("onEvent");
        }
    }

    public int maxDepth() {
        return maxDepth;
    }

    public static FileObserverContract watchReentrant(String path, int mask, String next) {
        FileObserverContract observer = new FileObserverContract(path, mask, 0);
        observer.reenterPath = next;
        observer.startWatching();
        return observer;
    }

    public String events() {
        return events.toString();
    }

    public static FileObserverContract watch(String path, int mask, int throwsLeft) {
        FileObserverContract observer = new FileObserverContract(path, mask, throwsLeft);
        observer.startWatching();
        return observer;
    }

    public static void writeStream(String path) throws IOException {
        FileOutputStream stream = new FileOutputStream(new File(path));
        stream.write(1);
        stream.close();
    }

    public static FileWriter openWriter(String path) throws IOException {
        return new FileWriter(new File(path));
    }

    public static void writeAndClose(FileWriter writer) throws IOException {
        BufferedWriter buffered = new BufferedWriter(writer);
        buffered.write("a");
        buffered.write("b");
        buffered.flush();
        buffered.close();
    }

    public static int writeAfterClose(FileWriter writer) {
        try {
            writer.write("late");
            return 0;
        } catch (IOException expected) {
            return "Stream closed".equals(expected.getMessage()) ? 1 : -1;
        }
    }

    public static RandomAccessFile openRandomAccess(String path) throws IOException {
        return new RandomAccessFile(new File(path), "rw");
    }

    public static void closeRandomAccess(RandomAccessFile file) throws IOException {
        file.close();
    }

    public static int writeAndAwait(FileObserverContract observer, String path)
            throws IOException, InterruptedException {
        writeStream(path);
        synchronized (lock) {
            while (observer.count < 2) {
                lock.wait();
            }
        }
        return observer.count;
    }
}
