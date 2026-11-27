package winedroid.tests;

public class CloneContract implements Cloneable {
    public int count;
    public String label;
    public int[] shared;

    static class Plain {
        Object copy() throws CloneNotSupportedException {
            return super.clone();
        }
    }

    CloneContract copy() throws CloneNotSupportedException {
        return (CloneContract) super.clone();
    }

    public static String cloneCopiesFieldsShallowly() throws Exception {
        CloneContract original = new CloneContract();
        original.count = 7;
        original.label = "seven";
        original.shared = new int[] {1};
        CloneContract copy = original.copy();
        copy.count = 8;
        copy.shared[0] = 2;
        return (copy != original) + ":" + original.count + ":" + copy.count + ":"
                + copy.label + ":" + (copy.shared == original.shared) + ":"
                + original.shared[0] + ":" + (copy.getClass() == CloneContract.class);
    }

    public static String cloneWithoutCloneableThrows() {
        try {
            new Plain().copy();
            return "no exception";
        } catch (CloneNotSupportedException expected) {
            return expected.getMessage();
        }
    }
}
