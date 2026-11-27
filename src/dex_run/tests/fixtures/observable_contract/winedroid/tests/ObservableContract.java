package winedroid.tests;

import java.util.Observable;
import java.util.Observer;

public class ObservableContract {
    static class Keyed implements Observer {
        final int id;

        Keyed(int id) {
            this.id = id;
        }

        @Override
        public void update(Observable observable, Object argument) {
        }

        @Override
        public boolean equals(Object other) {
            return other instanceof Keyed && ((Keyed) other).id == id;
        }

        @Override
        public int hashCode() {
            return id;
        }
    }

    static class Plain implements Observer {
        @Override
        public void update(Observable observable, Object argument) {
        }
    }

    public static String membershipUsesEquals() {
        Observable observable = new Observable();
        observable.addObserver(new Keyed(1));
        observable.addObserver(new Keyed(1));
        observable.addObserver(new Keyed(2));
        Plain plain = new Plain();
        observable.addObserver(plain);
        observable.addObserver(plain);
        observable.addObserver(new Plain());
        String counts = observable.countObservers() + ",";
        observable.deleteObserver(new Keyed(2));
        observable.deleteObserver(plain);
        counts += observable.countObservers() + ",";
        try {
            observable.addObserver(null);
        } catch (NullPointerException expected) {
            counts += "npe,";
        }
        observable.deleteObservers();
        return counts + observable.countObservers();
    }
}
