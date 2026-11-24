package winedroid.tests;

public final class ObjectWaitContract {
    static final Object lock = new Object();
    static boolean loaded;

    static final class Loader extends Thread {
        @Override
        public void run() {
            synchronized (lock) {
                loaded = true;
                lock.notifyAll();
            }
        }
    }

    public static int waitLoopTerminatesAfterStartedThreadRan() throws InterruptedException {
        loaded = false;
        new Loader().start();
        synchronized (lock) {
            while (!loaded) {
                lock.wait();
            }
        }
        return 1;
    }

    public static int waitWithoutRunnableThreadBlocksForever() throws InterruptedException {
        synchronized (lock) {
            lock.wait();
        }
        return 1;
    }

    public static int timedWaitReturnsWithoutRunnableThread() throws InterruptedException {
        synchronized (lock) {
            lock.wait(50);
        }
        return 1;
    }

    public static int negativeTimeoutThrowsIllegalArgument() throws InterruptedException {
        loaded = false;
        new Loader().start();
        try {
            synchronized (lock) {
                lock.wait(-1);
            }
            return -1;
        } catch (IllegalArgumentException expected) {
            return loaded ? -2 : 1;
        }
    }

    public static int outOfRangeNanosThrowsIllegalArgument() throws InterruptedException {
        try {
            synchronized (lock) {
                lock.wait(0, 1000000);
            }
            return -1;
        } catch (IllegalArgumentException expected) {
            return 1;
        }
    }
}
