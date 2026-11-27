package android.os;

public abstract class FileObserver {
    public FileObserver(String path, int mask) {}
    public void startWatching() {}
    public void stopWatching() {}
    public abstract void onEvent(int event, String path);
}
