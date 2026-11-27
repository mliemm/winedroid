package winedroid.tests;

import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;
import java.lang.reflect.TypeVariable;
import java.lang.reflect.WildcardType;
import java.util.Arrays;
import java.util.List;
import java.util.Map;

public final class GenericSignatureContract {
    static class Base<T> {}

    static class StringBase extends Base<String> {}

    static class MapBase extends Base<Map<String, Integer>> {}

    static class OtherStringBase extends Base<String> {}

    static class Token<T> {
        final Type type;

        Token() {
            Type superclass = getClass().getGenericSuperclass();
            if (superclass instanceof Class) {
                throw new RuntimeException("Missing type parameter.");
            }
            this.type = ((ParameterizedType) superclass).getActualTypeArguments()[0];
        }
    }

    static class ListToken extends Token<List<? extends Map<String, ? extends Object>>> {}

    public static int directSuperclass() {
        Type type = StringBase.class.getGenericSuperclass();
        if (!(type instanceof ParameterizedType)) {
            return -1;
        }
        ParameterizedType parameterized = (ParameterizedType) type;
        Type[] arguments = parameterized.getActualTypeArguments();
        if (arguments.length != 1) {
            return -2;
        }
        if (arguments[0] != String.class) {
            return -3;
        }
        if (parameterized.getRawType() != Base.class) {
            return -4;
        }
        if (!parameterized.toString().contains("java.lang.String")) {
            return -5;
        }
        return 1;
    }

    public static int nestedTypeArguments() {
        Type type = MapBase.class.getGenericSuperclass();
        if (!(type instanceof ParameterizedType)) {
            return -1;
        }
        Type argument = ((ParameterizedType) type).getActualTypeArguments()[0];
        if (!(argument instanceof ParameterizedType)) {
            return -2;
        }
        ParameterizedType map = (ParameterizedType) argument;
        if (map.getRawType() != Map.class) {
            return -3;
        }
        Type[] arguments = map.getActualTypeArguments();
        if (arguments.length != 2) {
            return -4;
        }
        if (arguments[0] != String.class || arguments[1] != Integer.class) {
            return -5;
        }
        return 1;
    }

    public static int arraysHashCode() {
        Type[] left = ((ParameterizedType) StringBase.class.getGenericSuperclass())
                .getActualTypeArguments();
        Type[] right = ((ParameterizedType) OtherStringBase.class.getGenericSuperclass())
                .getActualTypeArguments();
        if (Arrays.hashCode(left) != Arrays.hashCode(right)) {
            return -1;
        }
        if (!Arrays.equals(left, right)) {
            return -2;
        }
        return 1;
    }

    public static int enclosingClasses() {
        if (StringBase.class.getEnclosingClass() != GenericSignatureContract.class) {
            return -1;
        }
        if (StringBase.class.getDeclaringClass() != GenericSignatureContract.class) {
            return -2;
        }
        if (String.class.getEnclosingClass() != null) {
            return -3;
        }
        return 1;
    }

    public static int typeTokenShape() {
        Type type = new ListToken().type;
        if (!(type instanceof ParameterizedType)) {
            return -1;
        }
        ParameterizedType list = (ParameterizedType) type;
        if (list.getRawType() != List.class) {
            return -2;
        }
        Type[] listArguments = list.getActualTypeArguments();
        if (listArguments.length != 1 || !(listArguments[0] instanceof WildcardType)) {
            return -3;
        }
        WildcardType wildcard = (WildcardType) listArguments[0];
        if (wildcard.getLowerBounds().length != 0) {
            return -4;
        }
        Type[] upper = wildcard.getUpperBounds();
        if (upper.length != 1 || !(upper[0] instanceof ParameterizedType)) {
            return -5;
        }
        ParameterizedType map = (ParameterizedType) upper[0];
        if (map.getRawType() != Map.class) {
            return -6;
        }
        Type[] mapArguments = map.getActualTypeArguments();
        if (mapArguments.length != 2) {
            return -7;
        }
        if (mapArguments[0] != String.class) {
            return -8;
        }
        if (!(mapArguments[1] instanceof WildcardType)) {
            return -9;
        }
        Type[] objectBounds = ((WildcardType) mapArguments[1]).getUpperBounds();
        if (objectBounds.length != 1 || objectBounds[0] != Object.class) {
            return -10;
        }
        return 1;
    }

    public static int typeParameters() {
        Type[] parameters = Base.class.getTypeParameters();
        if (parameters.length != 1) {
            return -1;
        }
        if (!(parameters[0] instanceof TypeVariable)) {
            return -2;
        }
        TypeVariable<?> variable = (TypeVariable<?>) parameters[0];
        if (!"T".equals(variable.getName())) {
            return -3;
        }
        Type[] bounds = variable.getBounds();
        if (bounds.length != 1 || bounds[0] != Object.class) {
            return -4;
        }
        if (variable.getGenericDeclaration() != Base.class) {
            return -5;
        }
        return 1;
    }

    public static int missingTypeParameter() {
        try {
            new Token() {};
            return -1;
        } catch (RuntimeException exception) {
            return "Missing type parameter.".equals(exception.getMessage()) ? 1 : -2;
        }
    }
}
