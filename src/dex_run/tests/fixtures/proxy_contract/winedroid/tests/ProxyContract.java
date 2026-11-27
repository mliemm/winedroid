package winedroid.tests;

import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Proxy;

public final class ProxyContract {
    public interface Counter {
        int add(int delta);

        long doubleValue(long value);

        String name();

        void record(int value);
    }

    static int recorded;

    static final class Handler implements InvocationHandler {
        int calls;

        @Override
        public Object invoke(Object proxy, Method method, Object[] args) {
            calls++;
            String name = method.getName();
            if (name.equals("add")) {
                return Integer.valueOf(((Integer) args[0]).intValue() + 2);
            }
            if (name.equals("doubleValue")) {
                return Long.valueOf(((Long) args[0]).longValue() * 2L);
            }
            if (name.equals("name")) {
                return args == null ? "noargs" : "args";
            }
            if (name.equals("record")) {
                recorded = ((Integer) args[0]).intValue();
                return null;
            }
            if (name.equals("toString")) {
                return "proxy";
            }
            if (name.equals("hashCode")) {
                return Integer.valueOf(7);
            }
            if (name.equals("equals")) {
                return Boolean.valueOf(proxy == args[0]);
            }
            return null;
        }
    }

    private static Counter newCounter(Handler handler) {
        return (Counter) Proxy.newProxyInstance(
                ProxyContract.class.getClassLoader(),
                new Class<?>[] {Counter.class},
                handler);
    }

    public static int interfaceDispatch() {
        Handler handler = new Handler();
        Counter counter = newCounter(handler);
        int sum = counter.add(5);
        long doubled = counter.doubleValue(21L);
        String name = counter.name();
        counter.record(9);
        int result = sum + (int) doubled + recorded;
        if ("noargs".equals(name)) {
            result += 100;
        }
        if (handler.calls == 4) {
            result += 200;
        }
        return result;
    }

    public static int objectDispatch() {
        Counter counter = newCounter(new Handler());
        int result = 0;
        if (counter.hashCode() == 7) {
            result |= 1;
        }
        if (counter.equals(counter)) {
            result |= 2;
        }
        if ("proxy".equals(counter.toString())) {
            result |= 4;
        }
        return result;
    }

    public static int typeIdentity() {
        Handler handler = new Handler();
        Object proxy = Proxy.newProxyInstance(
                ProxyContract.class.getClassLoader(),
                new Class<?>[] {Counter.class},
                handler);
        int result = 0;
        if (proxy instanceof Counter) {
            result |= 1;
        }
        if (Proxy.isProxyClass(proxy.getClass())) {
            result |= 2;
        }
        if (Proxy.getInvocationHandler(proxy) == handler) {
            result |= 4;
        }
        if (proxy.getClass().getSuperclass() == Proxy.class) {
            result |= 8;
        }
        if (proxy.getClass().getInterfaces().length == 1) {
            result |= 16;
        }
        return result;
    }

    public static int handlerException() {
        Counter counter = (Counter) Proxy.newProxyInstance(
                ProxyContract.class.getClassLoader(),
                new Class<?>[] {Counter.class},
                new InvocationHandler() {
                    @Override
                    public Object invoke(Object proxy, Method method, Object[] args) {
                        throw new IllegalStateException("boom");
                    }
                });
        try {
            counter.add(1);
            return -1;
        } catch (IllegalStateException exception) {
            return 1;
        }
    }

    public static int repeatedInterface() {
        try {
            Proxy.newProxyInstance(
                    ProxyContract.class.getClassLoader(),
                    new Class<?>[] {Counter.class, Counter.class},
                    new Handler());
            return -1;
        } catch (IllegalArgumentException exception) {
            return 1;
        }
    }

    public static int notAnInterface() {
        try {
            Proxy.newProxyInstance(
                    ProxyContract.class.getClassLoader(),
                    new Class<?>[] {String.class},
                    new Handler());
            return -1;
        } catch (IllegalArgumentException exception) {
            return 1;
        }
    }

    public static int nullHandler() {
        try {
            Proxy.newProxyInstance(
                    ProxyContract.class.getClassLoader(),
                    new Class<?>[] {Counter.class},
                    null);
            return -1;
        } catch (NullPointerException exception) {
            return 1;
        }
    }
}
