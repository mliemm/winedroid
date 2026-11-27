package winedroid.tests;

import android.os.MessageQueue;

public class IdleContract implements MessageQueue.IdleHandler {
    public int calls;
    public boolean keep;

    public boolean queueIdle() {
        calls++;
        return keep;
    }
}
