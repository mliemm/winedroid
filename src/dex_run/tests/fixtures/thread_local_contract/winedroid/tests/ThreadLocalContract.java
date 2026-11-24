package winedroid.tests;

public final class ThreadLocalContract {
    static int countingCalls;
    static int throwingCalls;

    static final class Counting extends ThreadLocal<String> {
        @Override
        protected String initialValue() {
            countingCalls++;
            return "initial";
        }
    }

    static final class Throwing extends ThreadLocal<String> {
        @Override
        protected String initialValue() {
            throwingCalls++;
            throw new IllegalStateException();
        }
    }

    static final class Plain extends ThreadLocal<String> {
    }

    public static int initialValueRunsOncePerGetUntilRemove() {
        countingCalls = 0;
        Counting local = new Counting();
        if (!"initial".equals(local.get())) {
            return -1;
        }
        if (!"initial".equals(local.get())) {
            return -2;
        }
        if (countingCalls != 1) {
            return -3;
        }
        local.set("set");
        if (!"set".equals(local.get())) {
            return -4;
        }
        if (countingCalls != 1) {
            return -5;
        }
        local.remove();
        if (!"initial".equals(local.get())) {
            return -6;
        }
        return countingCalls;
    }

    public static int throwingInitialValuePropagatesAndStoresNothing() {
        throwingCalls = 0;
        Throwing local = new Throwing();
        for (int attempt = 0; attempt < 2; attempt++) {
            try {
                local.get();
                return -1;
            } catch (IllegalStateException expected) {
            }
        }
        return throwingCalls;
    }

    public static int defaultInitialValueIsNull() {
        Plain local = new Plain();
        if (local.get() != null) {
            return -1;
        }
        local.set("set");
        return "set".equals(local.get()) ? 1 : -2;
    }
}
