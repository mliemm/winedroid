package winedroid.tests;

import java.io.IOException;
import java.io.ObjectOutputStream;
import java.io.Serializable;
import java.nio.file.Files;
import java.nio.file.Paths;

public class SerialHost {
    public static class Ghost implements Serializable {
        private static final long serialVersionUID = 3L;
        public int haunt = 9;
    }

    public static class Outer implements Serializable {
        private static final long serialVersionUID = 4L;
        public Object ghost;
        public String label;
    }

    public static class Hooked implements Serializable {
        private static final long serialVersionUID = 5L;
        public int kept;

        private void writeObject(ObjectOutputStream out) throws IOException {
            out.defaultWriteObject();
            out.writeInt(0x11223344);
            out.writeObject("extra");
        }
    }

    public static void main(String[] arguments) throws Exception {
        java.io.ByteArrayOutputStream bytes = new java.io.ByteArrayOutputStream();
        ObjectOutputStream out = new ObjectOutputStream(bytes);
        Outer outer = new Outer();
        outer.ghost = new Ghost();
        outer.label = "first";
        out.writeObject(outer);
        out.writeObject("after");
        Hooked hooked = new Hooked();
        hooked.kept = 77;
        out.writeObject(hooked);
        out.writeObject("tail");
        out.close();
        Files.write(Paths.get(arguments[0]), bytes.toByteArray());
    }
}
