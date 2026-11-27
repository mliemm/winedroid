package winedroid.tests;

import java.io.File;
import java.io.FileFilter;
import java.io.FilenameFilter;
import java.io.IOException;

public final class ListFilesContract {
    private static String names(File[] files) {
        if (files == null) {
            return "null";
        }
        StringBuilder out = new StringBuilder();
        for (String expected : new String[] {"a.txt", "b.log", "c.txt"}) {
            for (File file : files) {
                if (file.getName().equals(expected)) {
                    out.append(out.length() == 0 ? "" : "+").append(expected);
                }
            }
        }
        return out.append("/").append(files.length).toString();
    }

    public static String run(String path) throws IOException {
        final File dir = new File(path);
        dir.mkdirs();
        new File(dir, "a.txt").createNewFile();
        new File(dir, "b.log").createNewFile();
        new File(dir, "c.txt").createNewFile();
        final StringBuilder seen = new StringBuilder();
        String byName = names(dir.listFiles(new FilenameFilter() {
            @Override
            public boolean accept(File parent, String name) {
                if (parent == dir) {
                    seen.append('=');
                }
                return name.endsWith(".txt");
            }
        }));
        String byFile = names(dir.listFiles(new FileFilter() {
            @Override
            public boolean accept(File file) {
                return file.getName().startsWith("b") && file.getParentFile().getPath().equals(dir.getPath());
            }
        }));
        String all = names(dir.listFiles((FilenameFilter) null));
        String missing = names(new File(dir, "nope").listFiles(new FilenameFilter() {
            @Override
            public boolean accept(File parent, String name) {
                return true;
            }
        }));
        String thrown;
        try {
            dir.listFiles(new FilenameFilter() {
                @Override
                public boolean accept(File parent, String name) {
                    throw new IllegalStateException("no " + name.length());
                }
            });
            thrown = "none";
        } catch (IllegalStateException e) {
            thrown = e.getMessage();
        }
        return byName + ";" + seen + ";" + byFile + ";" + all + ";" + missing + ";" + thrown;
    }
}
