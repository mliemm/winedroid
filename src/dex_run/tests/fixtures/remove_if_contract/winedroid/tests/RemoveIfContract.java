package winedroid.tests;

import java.util.ArrayList;
import java.util.function.Predicate;

public final class RemoveIfContract {
    static final class OddPredicate implements Predicate<Integer> {
        @Override
        public boolean test(Integer value) {
            return value.intValue() % 2 != 0;
        }
    }

    public static int removeOdd() {
        ArrayList<Integer> values = new ArrayList<>();
        for (int i = 1; i <= 5; i++) {
            values.add(Integer.valueOf(i));
        }
        boolean removed = values.removeIf(new OddPredicate());
        if (!removed) {
            return -1;
        }
        if (values.size() != 2) {
            return -2;
        }
        if (values.get(0).intValue() != 2 || values.get(1).intValue() != 4) {
            return -3;
        }
        return 1;
    }
}
