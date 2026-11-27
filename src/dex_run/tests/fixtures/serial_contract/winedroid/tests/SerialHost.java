package winedroid.tests;

import java.io.Serializable;

public class SerialHost {
    public static class Outer implements Serializable {
        private static final long serialVersionUID = 4L;
        public Object ghost;
        public String label;
    }

    public static class Hooked implements Serializable {
        private static final long serialVersionUID = 5L;
        public int kept;
    }
}
