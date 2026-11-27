use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::super::*;
use super::helpers::*;
use crate::external_storage::{
    ExternalStoragePolicy, ExternalStorageRuntimeAccess, READ_EXTERNAL_STORAGE,
    WRITE_EXTERNAL_STORAGE,
};

#[test]
fn libcore_file_parent_child_constructor_appends_an_absolute_child() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/io/File;".to_owned(), "Ljava/lang/String;".to_owned()],
        },
    };
    let get_parent_file = DexMethodId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        name: "getParentFile".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/io/File;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let file = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Ljava/io/File;"),
    });
    let parent = file_value("/data/user/0/com.example/cache");
    let path_field = DexFieldId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "path".to_owned(),
    };

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                file.clone(),
                parent,
                DexValue::String(std::rc::Rc::from("/room.db"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&file, &path_field),
        Some(DexValue::String(std::rc::Rc::from(
            "/data/user/0/com.example/cache/room.db"
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_parent_file, Some(&test_invocation(vec![file]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/io/File;:/data/user/0/com.example/cache")
        )))
    );
}

fn file_parent_method(name: &str, return_type: &str) -> DexMethodId {
    DexMethodId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: return_type.to_owned(),
            parameters: Vec::new(),
        },
    }
}

#[test]
fn libcore_file_get_parent_returns_the_path_before_the_last_separator_or_null() {
    // cite: libcore File.java#getParent
    let program = DexProgram::new(Vec::new());
    let get_parent = file_parent_method("getParent", "Ljava/lang/String;");
    let cases = [
        ("/a/b/c", Some("/a/b")),
        ("/a", Some("/")),
        ("/", None),
        ("a/b", Some("a")),
        ("a", None),
    ];

    for (path, expected) in cases {
        let expected = match expected {
            Some(parent) => DexValue::String(std::rc::Rc::from(parent)),
            None => DexValue::Null,
        };

        assert_eq!(
            program
                .invoke_builtin_method(&get_parent, Some(&test_invocation(vec![file_value(path)]))),
            Some(DexExecutionResult::Value(expected)),
            "{path}"
        );
    }
}

#[test]
fn libcore_file_get_parent_file_returns_null_when_there_is_no_parent() {
    // cite: libcore File.java#getParentFile
    let program = DexProgram::new(Vec::new());
    let get_parent_file = file_parent_method("getParentFile", "Ljava/io/File;");

    for path in ["/", "a"] {
        assert_eq!(
            program.invoke_builtin_method(
                &get_parent_file,
                Some(&test_invocation(vec![file_value(path)]))
            ),
            Some(DexExecutionResult::Value(DexValue::Null)),
            "{path}"
        );
    }
}

#[test]
fn libcore_file_single_string_constructor_normalizes_pathname() {
    // cite: libcore File.java#File
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let get_absolute_path = DexMethodId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        name: "getAbsolutePath".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let file = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Ljava/io/File;"),
    });
    let path_field = DexFieldId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "path".to_owned(),
    };

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                file.clone(),
                DexValue::String(std::rc::Rc::from(
                    "/storage/emulated/0/Download//report.pdf"
                )),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&file, &path_field),
        Some(DexValue::String(std::rc::Rc::from(
            "/storage/emulated/0/Download/report.pdf"
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_absolute_path, Some(&test_invocation(vec![file]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("/storage/emulated/0/Download/report.pdf")
        )))
    );
}

#[test]
fn libcore_file_is_hidden_checks_leading_dot_in_name() {
    // cite: libcore File.java#isHidden
    // cite: libcore UnixFileSystem.java#getBooleanAttributes
    let program = DexProgram::new(Vec::new());
    let is_hidden = DexMethodId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        name: "isHidden".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let hidden = file_value("/storage/emulated/0/.nomedia");
    let visible = file_value("/storage/emulated/0/Download");

    assert_eq!(
        program.invoke_builtin_method(&is_hidden, Some(&test_invocation(vec![hidden]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&is_hidden, Some(&test_invocation(vec![visible]))),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn libcore_file_get_name_returns_last_path_component() {
    let program = DexProgram::new(Vec::new());
    let get_name = DexMethodId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        name: "getName".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let reference = file_value("/data/user/0/com.example/files");
    let object = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Ljava/io/File;"),
    });
    let path_field = DexFieldId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "path".to_owned(),
    };

    assert!(program.write_stored_instance_field(
        &object,
        &path_field,
        DexValue::String(std::rc::Rc::from("/data/user/0/com.example/cache/room.db",)),
    ));
    assert_eq!(
        program.invoke_builtin_method(&get_name, Some(&test_invocation(vec![reference]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("files")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_name, Some(&test_invocation(vec![object]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("room.db")
        )))
    );
}

#[test]
fn libcore_file_get_absolute_file_uses_android_root_for_relative_paths() {
    let program = DexProgram::new(Vec::new());
    let get_absolute_file = DexMethodId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        name: "getAbsoluteFile".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/io/File;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let absolute = file_value("/data/user/0/com.example/files");
    let relative = file_value("relative.db");

    assert_eq!(
        program.invoke_builtin_method(&get_absolute_file, Some(&test_invocation(vec![absolute]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/io/File;:/data/user/0/com.example/files")
        )))
    );

    assert_eq!(
        program.invoke_builtin_method(&get_absolute_file, Some(&test_invocation(vec![relative]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/io/File;:/relative.db")
        )))
    );
}

#[test]
fn libcore_file_get_absolute_path_never_exposes_the_host_working_directory() {
    let program = DexProgram::new(Vec::new());
    let get_absolute_path = DexMethodId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        name: "getAbsolutePath".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let relative = file_value("relative.db");

    assert_eq!(
        program.invoke_builtin_method(&get_absolute_path, Some(&test_invocation(vec![relative]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("/relative.db")
        )))
    );
}

#[test]
fn libcore_file_get_canonical_file_returns_only_a_guest_path() {
    let fixture = GuestIoFixture::new("canonical");
    fs::create_dir_all(fixture.private.join("cache"))
        .expect("canonical test directory should exist");
    let program = fixture.program();
    let get_canonical_file = DexMethodId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        name: "getCanonicalFile".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/io/File;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let file = file_value("/data/user/0/com.example/files/../cache");
    let missing_file = file_value("/data/user/0/com.example/files/../cache/missing.db");

    assert_eq!(
        program.invoke_builtin_method(&get_canonical_file, Some(&test_invocation(vec![file]))),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/io/File;:/data/user/0/com.example/cache")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_canonical_file,
            Some(&test_invocation(vec![missing_file]))
        ),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Ljava/io/File;:/data/user/0/com.example/cache/missing.db")
        )))
    );
}

#[test]
fn builtin_stat_fs_reports_positive_volume_stats_for_a_guest_path() {
    // cite: frameworks-base StatFs.java#StatFs, getBlockSizeLong, getBlockCountLong
    let fixture = GuestIoFixture::new("statfs");
    let program = fixture.program();
    let constructor = DexMethodId {
        class_descriptor: "Landroid/os/StatFs;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let stat_fs = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Landroid/os/StatFs;"),
    });

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                stat_fs.clone(),
                DexValue::String(std::rc::Rc::from("/data/user/0/com.example/files")),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );

    let getters = [
        "getBlockSizeLong",
        "getBlockCountLong",
        "getFreeBlocksLong",
        "getFreeBytes",
        "getAvailableBlocksLong",
        "getAvailableBytes",
        "getTotalBytes",
    ];
    for name in getters {
        let method = DexMethodId {
            class_descriptor: "Landroid/os/StatFs;".to_owned(),
            name: name.to_owned(),
            proto: crate::dex::DexProto {
                shorty: "J".to_owned(),
                return_type: "J".to_owned(),
                parameters: Vec::new(),
            },
        };
        let Some(DexExecutionResult::Value(DexValue::Long(value))) =
            program.invoke_builtin_method(&method, Some(&test_invocation(vec![stat_fs.clone()])))
        else {
            panic!("{name} should return a long value");
        };
        assert!(value > 0, "{name} should be positive, got {value}");
    }
}

#[test]
fn builtin_stat_fs_constructor_throws_for_a_path_outside_the_guest_mount() {
    // cite: frameworks-base StatFs.java#StatFs
    let fixture = GuestIoFixture::new("statfs_invalid");
    let program = fixture.program();
    let constructor = DexMethodId {
        class_descriptor: "Landroid/os/StatFs;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let stat_fs = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("Landroid/os/StatFs;"),
    });

    let Some(DexExecutionResult::Throw(exception)) = program.invoke_builtin_method(
        &constructor,
        Some(&test_invocation(vec![
            stat_fs,
            DexValue::String(std::rc::Rc::from("/etc/passwd")),
        ])),
    ) else {
        panic!("StatFs(String) should throw for a path outside the guest mount");
    };
    assert_eq!(
        value_class_descriptor(&exception),
        Some("Ljava/lang/IllegalArgumentException;")
    );
}

#[test]
fn libcore_file_mkdirs_creates_only_inside_the_guest_mount() {
    let fixture = GuestIoFixture::new("mkdirs");
    let program = fixture.program();
    let mkdirs = DexMethodId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        name: "mkdirs".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let is_directory = DexMethodId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        name: "isDirectory".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let path = fixture.private.join("files/nested");
    let file = file_value("/data/user/0/com.example/files/nested");

    assert_eq!(
        program.invoke_builtin_method(&mkdirs, Some(&test_invocation(vec![file.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert!(path.is_dir());
    assert_eq!(
        program.invoke_builtin_method(&is_directory, Some(&test_invocation(vec![file]))),
        Some(DexExecutionResult::Int(1))
    );
}

#[test]
fn libcore_file_exists_reports_guest_filesystem_presence() {
    let fixture = GuestIoFixture::new("exists");
    let program = fixture.program();
    let exists = DexMethodId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        name: "exists".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "Z".to_owned(),
            return_type: "Z".to_owned(),
            parameters: Vec::new(),
        },
    };
    let path = fixture.private.join("files/preferences.preferences_pb");
    let file = file_value("/data/user/0/com.example/files/preferences.preferences_pb");

    assert_eq!(
        program.invoke_builtin_method(&exists, Some(&test_invocation(vec![file.clone()]))),
        Some(DexExecutionResult::Int(0))
    );

    fs::create_dir_all(path.parent().expect("test path should have a parent"))
        .expect("exists test directory should be created");
    fs::write(&path, b"datastore").expect("exists test file should be written");

    assert_eq!(
        program.invoke_builtin_method(&exists, Some(&test_invocation(vec![file]))),
        Some(DexExecutionResult::Int(1))
    );
}

// cite: frameworks-base AndroidManifest.xml#READ_EXTERNAL_STORAGE, WRITE_EXTERNAL_STORAGE
// cite: frameworks-base Environment.java#isExternalStorageLegacy
#[test]
fn aosp_legacy_external_storage_read_and_write_grants_remain_independent_in_java_io() {
    let fixture = GuestIoFixture::new("legacy_external_access_modes");
    let directory = fixture.shared.join("Documents");
    fs::create_dir_all(&directory).expect("public documents directory should exist");
    fs::write(directory.join("readable.txt"), b"readable").expect("public test file should exist");
    let exists = java_io_method("Ljava/io/File;", "exists", "Z", &[]);
    let input = java_io_method(
        "Ljava/io/FileInputStream;",
        "<init>",
        "V",
        &["Ljava/io/File;"],
    );
    let output = java_io_method(
        "Ljava/io/FileOutputStream;",
        "<init>",
        "V",
        &["Ljava/io/File;"],
    );
    let readable_file = file_value("/storage/emulated/0/Documents/readable.txt");
    let writable_file = file_value("/storage/emulated/0/Documents/writable.txt");

    let read_policy = ExternalStoragePolicy::new(
        28,
        28,
        vec![READ_EXTERNAL_STORAGE.to_owned()],
        ExternalStorageRuntimeAccess::with_legacy_file_access(
            vec![READ_EXTERNAL_STORAGE.to_owned()],
            true,
            true,
            false,
        ),
    );
    let read_program = fixture.program_with_external_storage_policy(read_policy);
    assert_eq!(
        read_program
            .invoke_builtin_method(&exists, Some(&test_invocation(vec![readable_file.clone()])),),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        read_program.invoke_builtin_method(
            &input,
            Some(&test_invocation(vec![
                object_value(301, "Ljava/io/FileInputStream;"),
                readable_file.clone(),
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert!(matches!(
        read_program.invoke_builtin_method(
            &output,
            Some(&test_invocation(vec![
                object_value(302, "Ljava/io/FileOutputStream;"),
                writable_file.clone(),
            ])),
        ),
        Some(DexExecutionResult::Throw(_))
    ));
    assert!(!directory.join("writable.txt").exists());

    let write_policy = ExternalStoragePolicy::new(
        28,
        28,
        vec![WRITE_EXTERNAL_STORAGE.to_owned()],
        ExternalStorageRuntimeAccess::with_legacy_file_access(
            vec![WRITE_EXTERNAL_STORAGE.to_owned()],
            true,
            false,
            true,
        ),
    );
    let write_program = fixture.program_with_external_storage_policy(write_policy);
    assert_eq!(
        write_program
            .invoke_builtin_method(&exists, Some(&test_invocation(vec![readable_file.clone()])),),
        Some(DexExecutionResult::Int(0))
    );
    assert!(matches!(
        write_program.invoke_builtin_method(
            &input,
            Some(&test_invocation(vec![
                object_value(303, "Ljava/io/FileInputStream;"),
                readable_file,
            ])),
        ),
        Some(DexExecutionResult::Throw(_))
    ));
    assert_eq!(
        write_program.invoke_builtin_method(
            &output,
            Some(&test_invocation(vec![
                object_value(304, "Ljava/io/FileOutputStream;"),
                writable_file,
            ])),
        ),
        Some(DexExecutionResult::Void)
    );
    assert!(directory.join("writable.txt").is_file());
}

#[cfg(unix)]
#[test]
fn unix_filesystem_check_access_uses_read_and_write_modes() {
    use std::os::unix::fs::PermissionsExt;

    if unsafe { libc::geteuid() } == 0 {
        return;
    }

    let fixture = GuestIoFixture::new("access_modes");
    fs::create_dir_all(fixture.private.join("files")).expect("guest files directory should exist");
    let host_path = fixture.private.join("files/mode.txt");
    fs::write(&host_path, b"mode").expect("guest file should exist");
    let file = file_value("/data/user/0/com.example/files/mode.txt");
    let program = fixture.program();
    let can_read = java_io_method("Ljava/io/File;", "canRead", "Z", &[]);
    let can_write = java_io_method("Ljava/io/File;", "canWrite", "Z", &[]);

    fs::set_permissions(&host_path, fs::Permissions::from_mode(0o200))
        .expect("write-only permissions should apply");
    assert_eq!(
        program.invoke_builtin_method(&can_read, Some(&test_invocation(vec![file.clone()]))),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(&can_write, Some(&test_invocation(vec![file.clone()]))),
        Some(DexExecutionResult::Int(1))
    );

    fs::set_permissions(&host_path, fs::Permissions::from_mode(0o400))
        .expect("read-only permissions should apply");
    assert_eq!(
        program.invoke_builtin_method(&can_read, Some(&test_invocation(vec![file.clone()]))),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.invoke_builtin_method(&can_write, Some(&test_invocation(vec![file]))),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn libcore_file_output_stream_opens_a_guest_file_and_stores_its_guest_path() {
    let fixture = GuestIoFixture::new("output_stream");
    let program = fixture.program();
    let constructor = DexMethodId {
        class_descriptor: "Ljava/io/FileOutputStream;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/io/File;".to_owned()],
        },
    };
    let get_channel = DexMethodId {
        class_descriptor: "Ljava/io/FileOutputStream;".to_owned(),
        name: "getChannel".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/nio/channels/FileChannel;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let lock = DexMethodId {
        class_descriptor: "Ljava/nio/channels/FileChannel;".to_owned(),
        name: "lock".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/nio/channels/FileLock;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let close_channel = DexMethodId {
        class_descriptor: "Ljava/nio/channels/spi/AbstractInterruptibleChannel;".to_owned(),
        name: "close".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let path = fixture.private.join("files/out.db");
    fs::create_dir_all(path.parent().expect("test path should have a parent"))
        .expect("output stream parent should exist");
    let guest_path = "/data/user/0/com.example/files/out.db";
    let stream = DexValue::Object(DexObjectRef {
        id: 2,
        class_descriptor: std::rc::Rc::from("Ljava/io/FileOutputStream;"),
    });
    let file = file_value(guest_path);
    let path_field = DexFieldId {
        class_descriptor: "Ljava/io/FileOutputStream;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "path".to_owned(),
    };

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![stream.clone(), file]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert!(path.is_file());
    assert_eq!(
        program.read_stored_instance_field(&stream, &path_field),
        Some(DexValue::String(std::rc::Rc::from(guest_path)))
    );
    let Some(DexExecutionResult::Value(channel)) =
        program.invoke_builtin_method(&get_channel, Some(&test_invocation(vec![stream.clone()])))
    else {
        panic!("getChannel should return a channel");
    };
    assert_eq!(
        program.invoke_builtin_method(&get_channel, Some(&test_invocation(vec![stream.clone()]))),
        Some(DexExecutionResult::Value(channel.clone())),
        "FileOutputStream.java getChannel returns the channel it created first"
    );
    let Some(DexExecutionResult::Value(lock)) =
        program.invoke_builtin_method(&lock, Some(&test_invocation(vec![channel.clone()])))
    else {
        panic!("a writable channel should be locked");
    };
    let is_valid = java_io_method("Ljava/nio/channels/FileLock;", "isValid", "Z", &[]);
    assert_eq!(
        program.invoke_builtin_method(&close_channel, Some(&test_invocation(vec![channel]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        invoke_value(&program, &is_valid, vec![lock]),
        DexExecutionResult::Int(0),
        "FileChannelImpl.java implCloseChannel invalidates the locks of the channel"
    );
    assert_eq!(
        program.read_stored_instance_field(
            &stream,
            &DexFieldId {
                class_descriptor: "Ljava/io/FileOutputStream;".to_owned(),
                type_descriptor: "Z".to_owned(),
                name: "closed".to_owned(),
            }
        ),
        Some(DexValue::Int(1)),
        "FileChannelImpl.java implCloseChannel closes the parent stream"
    );
}

#[test]
fn libcore_file_input_stream_opens_a_guest_file_and_stores_its_guest_path() {
    let fixture = GuestIoFixture::new("input_stream");
    let program = fixture.program();
    let constructor = DexMethodId {
        class_descriptor: "Ljava/io/FileInputStream;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/io/File;".to_owned()],
        },
    };
    let path = fixture.private.join("files/preferences.preferences_pb");
    fs::create_dir_all(path.parent().expect("test file should have a parent"))
        .expect("test input stream directory should be created");
    fs::write(&path, b"datastore").expect("test input stream file should be written");

    let stream = DexValue::Object(DexObjectRef {
        id: 42,
        class_descriptor: std::rc::Rc::from("Ljava/io/FileInputStream;"),
    });
    let guest_path = "/data/user/0/com.example/files/preferences.preferences_pb";
    let file = file_value(guest_path);
    let path_field = DexFieldId {
        class_descriptor: "Ljava/io/FileInputStream;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "path".to_owned(),
    };

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![stream.clone(), file]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&stream, &path_field),
        Some(DexValue::String(std::rc::Rc::from(guest_path)))
    );
}

#[test]
fn libcore_file_input_stream_throws_for_a_missing_guest_file() {
    let fixture = GuestIoFixture::new("missing_input");
    let program = fixture.program();
    let constructor = DexMethodId {
        class_descriptor: "Ljava/io/FileInputStream;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/io/File;".to_owned()],
        },
    };
    let guest_path = "/data/user/0/com.example/files/missing.preferences_pb";

    let stream = DexValue::Object(DexObjectRef {
        id: 43,
        class_descriptor: std::rc::Rc::from("Ljava/io/FileInputStream;"),
    });
    let file = file_value(guest_path);
    let message_field = DexFieldId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "detailMessage".to_owned(),
    };

    let Some(DexExecutionResult::Throw(exception)) =
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![stream, file])))
    else {
        panic!("missing file should throw FileNotFoundException");
    };

    assert!(program.value_matches_type(&exception, "Ljava/io/FileNotFoundException;"));
    assert!(program.value_matches_type(&exception, "Ljava/io/IOException;"));
    assert!(program.value_matches_type(&exception, "Ljava/lang/Exception;"));
    let Some(DexValue::String(message)) =
        program.read_stored_instance_field(&exception, &message_field)
    else {
        panic!("FileNotFoundException should contain a detail message");
    };
    assert!(message.starts_with(guest_path));
    assert!(message.contains("No such file or directory"));
}

#[test]
fn libcore_file_list_files_returns_guest_paths_instead_of_host_paths() {
    let fixture = GuestIoFixture::new("list_files");
    let directory = fixture.private.join("files");
    fs::create_dir_all(&directory).expect("guest directory should exist");
    fs::write(directory.join("entry.txt"), b"entry").expect("guest file should exist");
    let program = fixture.program();
    let list_files = java_io_method("Ljava/io/File;", "listFiles", "[Ljava/io/File;", &[]);

    let Some(DexExecutionResult::Value(array)) = program.invoke_builtin_method(
        &list_files,
        Some(&test_invocation(vec![file_value(
            "/data/user/0/com.example/files",
        )])),
    ) else {
        panic!("listFiles should return a guest array");
    };

    assert_eq!(
        program.read_array_value(&array, 0),
        Some(file_value("/data/user/0/com.example/files/entry.txt"))
    );
}

#[test]
fn environment_sdcard_alias_and_primary_path_share_a_rename_mount() {
    let fixture = GuestIoFixture::new("shared_rename");
    let directory = fixture.shared.join("Android/data/com.example/files");
    fs::create_dir_all(&directory).expect("app external directory should exist");
    fs::write(directory.join("source.txt"), b"source").expect("shared source should exist");
    let program = fixture.program();
    let rename = java_io_method("Ljava/io/File;", "renameTo", "Z", &["Ljava/io/File;"]);

    assert_eq!(
        program.invoke_builtin_method(
            &rename,
            Some(&test_invocation(vec![
                file_value("/sdcard/Android/data/com.example/files/source.txt"),
                file_value("/storage/emulated/0/Android/data/com.example/files/destination.txt",),
            ])),
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        fs::read(directory.join("destination.txt")).expect("renamed shared file should exist"),
        b"source"
    );
}

#[test]
fn libcore_file_dot_components_preserve_os_resolution_failures() {
    let fixture = GuestIoFixture::new("dot_resolution");
    fs::create_dir_all(fixture.private.join("files")).expect("guest files directory should exist");
    let victim = fixture.private.join("files/victim.txt");
    fs::write(&victim, b"victim").expect("victim should exist");
    fs::write(fixture.private.join("files/regular"), b"regular")
        .expect("regular component should exist");
    let program = fixture.program();
    let delete = java_io_method("Ljava/io/File;", "delete", "Z", &[]);
    let output = java_io_method(
        "Ljava/io/FileOutputStream;",
        "<init>",
        "V",
        &["Ljava/io/File;"],
    );

    for path in [
        "/data/user/0/com.example/files/missing/../victim.txt",
        "/data/user/0/com.example/files/regular/../victim.txt",
    ] {
        assert_eq!(
            program.invoke_builtin_method(&delete, Some(&test_invocation(vec![file_value(path)])),),
            Some(DexExecutionResult::Int(0))
        );
        assert!(matches!(
            program.invoke_builtin_method(
                &output,
                Some(&test_invocation(vec![
                    object_value(200, "Ljava/io/FileOutputStream;"),
                    file_value(path),
                ])),
            ),
            Some(DexExecutionResult::Throw(_))
        ));
        assert_eq!(fs::read(&victim).expect("victim should remain"), b"victim");
    }
}

#[test]
fn guest_mount_roots_cannot_be_deleted_or_renamed() {
    let fixture = GuestIoFixture::new("mount_roots");
    let program = fixture.program();
    let delete = java_io_method("Ljava/io/File;", "delete", "Z", &[]);
    let rename = java_io_method("Ljava/io/File;", "renameTo", "Z", &["Ljava/io/File;"]);

    for path in [
        "/data/user/0/com.example",
        "/data/user_de/0/com.example",
        "/storage/emulated/0",
        "/sdcard",
    ] {
        assert_eq!(
            program.invoke_builtin_method(&delete, Some(&test_invocation(vec![file_value(path)])),),
            Some(DexExecutionResult::Int(0))
        );
    }
    assert_eq!(
        program.invoke_builtin_method(
            &rename,
            Some(&test_invocation(vec![
                file_value("/data/user/0/com.example"),
                file_value("/storage/emulated/0/moved-private"),
            ])),
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert!(fixture.private.is_dir());
    assert!(fixture.device_private.is_dir());
    assert!(fixture.shared.is_dir());
    assert!(!fixture.shared.join("moved-private").exists());
}

#[test]
fn android_external_privacy_ancestor_cannot_be_renamed() {
    let fixture = GuestIoFixture::new("android_privacy_root");
    fs::create_dir_all(fixture.shared.join("Android/data/com.other/files"))
        .expect("other app directory should exist on the host");
    fs::write(
        fixture
            .shared
            .join("Android/data/com.other/files/secret.txt"),
        b"secret",
    )
    .expect("other app file should exist on the host");
    let program = fixture.program();
    let rename = java_io_method("Ljava/io/File;", "renameTo", "Z", &["Ljava/io/File;"]);
    let exists = java_io_method("Ljava/io/File;", "exists", "Z", &[]);

    assert_eq!(
        program.invoke_builtin_method(
            &rename,
            Some(&test_invocation(vec![
                file_value("/storage/emulated/0/Android"),
                file_value("/storage/emulated/0/moved"),
            ])),
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &exists,
            Some(&test_invocation(vec![file_value(
                "/storage/emulated/0/moved/data/com.other/files/secret.txt",
            )])),
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert!(fixture.shared.join("Android").is_dir());
    assert!(!fixture.shared.join("moved").exists());
}

#[cfg(unix)]
#[test]
fn guest_filesystem_boundary_blocks_host_and_symlink_entrypoints() {
    let fixture = GuestIoFixture::new("boundary");
    let outside = fixture.root.join("outside");
    fs::create_dir_all(&outside).expect("outside directory should exist");
    let secret = outside.join("secret.txt");
    fs::write(&secret, b"unchanged").expect("outside file should exist");
    fs::create_dir_all(fixture.private.join("files")).expect("guest files directory should exist");
    std::os::unix::fs::symlink(&outside, fixture.private.join("files/escape"))
        .expect("test symlink should exist");
    let source = fixture.private.join("files/source.txt");
    fs::write(&source, b"source").expect("rename source should exist");
    let program = fixture.program();
    let direct_host = file_value(&secret.to_string_lossy());
    let escaped = file_value("/data/user/0/com.example/files/escape/secret.txt");
    let other_app = file_value("/data/user/0/com.other/files/secret.txt");

    let exists = java_io_method("Ljava/io/File;", "exists", "Z", &[]);
    let length = java_io_method("Ljava/io/File;", "length", "J", &[]);
    let list_files = java_io_method("Ljava/io/File;", "listFiles", "[Ljava/io/File;", &[]);
    let delete = java_io_method("Ljava/io/File;", "delete", "Z", &[]);
    let rename = java_io_method("Ljava/io/File;", "renameTo", "Z", &["Ljava/io/File;"]);
    let input = java_io_method(
        "Ljava/io/FileInputStream;",
        "<init>",
        "V",
        &["Ljava/io/File;"],
    );
    let reader = java_io_method("Ljava/io/FileReader;", "<init>", "V", &["Ljava/io/File;"]);
    let output = java_io_method(
        "Ljava/io/FileOutputStream;",
        "<init>",
        "V",
        &["Ljava/io/File;"],
    );
    let writer = java_io_method("Ljava/io/FileWriter;", "<init>", "V", &["Ljava/io/File;"]);

    for file in [direct_host.clone(), escaped.clone(), other_app] {
        assert_eq!(
            program.invoke_builtin_method(&exists, Some(&test_invocation(vec![file.clone()]))),
            Some(DexExecutionResult::Int(0))
        );
        assert_eq!(
            program.invoke_builtin_method(&length, Some(&test_invocation(vec![file.clone()]))),
            Some(DexExecutionResult::Value(DexValue::Long(0)))
        );
        assert_eq!(
            program.invoke_builtin_method(&delete, Some(&test_invocation(vec![file.clone()]))),
            Some(DexExecutionResult::Int(0))
        );
        assert_eq!(
            program.invoke_builtin_method(&list_files, Some(&test_invocation(vec![file]))),
            Some(DexExecutionResult::Value(DexValue::Null))
        );
    }
    assert_eq!(
        program.invoke_builtin_method(
            &list_files,
            Some(&test_invocation(vec![file_value(
                &outside.to_string_lossy()
            )])),
        ),
        Some(DexExecutionResult::Value(DexValue::Null))
    );

    for (path_index, file) in [direct_host, escaped.clone()].into_iter().enumerate() {
        for (method, class, id) in [
            (&input, "Ljava/io/FileInputStream;", 100),
            (&reader, "Ljava/io/FileReader;", 101),
            (&output, "Ljava/io/FileOutputStream;", 102),
            (&writer, "Ljava/io/FileWriter;", 103),
        ] {
            assert!(matches!(
                program.invoke_builtin_method(
                    method,
                    Some(&test_invocation(vec![
                        object_value(id + path_index as u64 * 10, class),
                        file.clone()
                    ])),
                ),
                Some(DexExecutionResult::Throw(_))
            ));
        }
    }

    assert_eq!(
        program.invoke_builtin_method(
            &rename,
            Some(&test_invocation(vec![
                file_value("/data/user/0/com.example/files/source.txt"),
                file_value("/data/user/0/com.example/files/escape/moved.txt"),
            ])),
        ),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &rename,
            Some(&test_invocation(vec![
                file_value("/data/user/0/com.example/files/source.txt"),
                file_value("/storage/emulated/0/moved.txt"),
            ])),
        ),
        Some(DexExecutionResult::Int(0))
    );

    assert_eq!(
        fs::read(&secret).expect("outside file should remain"),
        b"unchanged"
    );
    assert!(source.is_file());
    assert!(!outside.join("moved.txt").exists());
    assert!(!fixture.shared.join("moved.txt").exists());
}

#[test]
fn builtin_output_stream_constructors_return_void() {
    let program = DexProgram::new(Vec::new());
    let byte_array_output_stream = DexMethodId {
        class_descriptor: "Ljava/io/ByteArrayOutputStream;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let data_output_stream = DexMethodId {
        class_descriptor: "Ljava/io/DataOutputStream;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/io/OutputStream;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&byte_array_output_stream, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&data_output_stream, None),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_string_writer_buffers_writes_and_appends() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/io/StringWriter;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let write_string = DexMethodId {
        class_descriptor: "Ljava/io/StringWriter;".to_owned(),
        name: "write".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let write_range = DexMethodId {
        class_descriptor: "Ljava/io/StringWriter;".to_owned(),
        name: "write".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLII".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Ljava/lang/String;".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
            ],
        },
    };
    let append_char = DexMethodId {
        class_descriptor: "Ljava/io/StringWriter;".to_owned(),
        name: "append".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LC".to_owned(),
            return_type: "Ljava/io/StringWriter;".to_owned(),
            parameters: vec!["C".to_owned()],
        },
    };
    let append_range = DexMethodId {
        class_descriptor: "Ljava/io/StringWriter;".to_owned(),
        name: "append".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LLII".to_owned(),
            return_type: "Ljava/io/StringWriter;".to_owned(),
            parameters: vec![
                "Ljava/lang/CharSequence;".to_owned(),
                "I".to_owned(),
                "I".to_owned(),
            ],
        },
    };
    let writer_write_char = DexMethodId {
        class_descriptor: "Ljava/io/Writer;".to_owned(),
        name: "write".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let writer_append_char = DexMethodId {
        class_descriptor: "Ljava/io/Writer;".to_owned(),
        name: "append".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LC".to_owned(),
            return_type: "Ljava/io/Writer;".to_owned(),
            parameters: vec!["C".to_owned()],
        },
    };
    let writer_close = DexMethodId {
        class_descriptor: "Ljava/io/Writer;".to_owned(),
        name: "close".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let to_string = DexMethodId {
        class_descriptor: "Ljava/io/StringWriter;".to_owned(),
        name: "toString".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let writer = DexValue::Object(DexObjectRef {
        id: 17,
        class_descriptor: std::rc::Rc::from("Ljava/io/StringWriter;"),
    });

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&test_invocation(vec![writer.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &write_string,
            Some(&test_invocation(vec![
                writer.clone(),
                DexValue::String(std::rc::Rc::from("alpha"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &write_range,
            Some(&test_invocation(vec![
                writer.clone(),
                DexValue::String(std::rc::Rc::from("abcdef")),
                DexValue::Int(2),
                DexValue::Int(3)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &append_char,
            Some(&test_invocation(vec![
                writer.clone(),
                DexValue::Int('!' as i32)
            ]))
        ),
        Some(DexExecutionResult::Value(writer.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &append_range,
            Some(&test_invocation(vec![
                writer.clone(),
                DexValue::String(std::rc::Rc::from("a😀z")),
                DexValue::Int(1),
                DexValue::Int(3)
            ]))
        ),
        Some(DexExecutionResult::Value(writer.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &writer_write_char,
            Some(&test_invocation(vec![
                writer.clone(),
                DexValue::Int('[' as i32)
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &writer_append_char,
            Some(&test_invocation(vec![
                writer.clone(),
                DexValue::Int(']' as i32)
            ]))
        ),
        Some(DexExecutionResult::Value(writer.clone()))
    );
    assert_eq!(
        program.invoke_builtin_method(&writer_close, Some(&test_invocation(vec![writer.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&to_string, Some(&test_invocation(vec![writer]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("alphacde!😀[]")
        )))
    );
}

#[test]
fn builtin_string_reader_reads_utf16_units_into_char_arrays() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Ljava/io/StringReader;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let read_char = DexMethodId {
        class_descriptor: "Ljava/io/StringReader;".to_owned(),
        name: "read".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let reader_read_buffer = DexMethodId {
        class_descriptor: "Ljava/io/Reader;".to_owned(),
        name: "read".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "ILII".to_owned(),
            return_type: "I".to_owned(),
            parameters: vec!["[C".to_owned(), "I".to_owned(), "I".to_owned()],
        },
    };
    let close = DexMethodId {
        class_descriptor: "Ljava/io/Reader;".to_owned(),
        name: "close".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let reader = DexValue::Object(DexObjectRef {
        id: 17,
        class_descriptor: std::rc::Rc::from("Ljava/io/StringReader;"),
    });
    let array = program
        .allocate_array("[C".to_owned(), 4)
        .expect("char array should allocate");
    let emoji_units: Vec<u16> = "\u{1f600}".encode_utf16().collect();

    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                reader.clone(),
                DexValue::String(std::rc::Rc::from("a\u{1f600}b"))
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&read_char, Some(&test_invocation(vec![reader.clone()]))),
        Some(DexExecutionResult::Int('a' as i32))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &reader_read_buffer,
            Some(&test_invocation(vec![
                reader.clone(),
                array.clone(),
                DexValue::Int(1),
                DexValue::Int(2)
            ]))
        ),
        Some(DexExecutionResult::Int(2))
    );
    assert_eq!(
        program.read_array_value(&array, 1),
        Some(DexValue::Int(i32::from(emoji_units[0])))
    );
    assert_eq!(
        program.read_array_value(&array, 2),
        Some(DexValue::Int(i32::from(emoji_units[1])))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &reader_read_buffer,
            Some(&test_invocation(vec![
                reader.clone(),
                array.clone(),
                DexValue::Int(0),
                DexValue::Int(4)
            ]))
        ),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        program.read_array_value(&array, 0),
        Some(DexValue::Int('b' as i32))
    );
    assert_eq!(
        program.invoke_builtin_method(&read_char, Some(&test_invocation(vec![reader.clone()]))),
        Some(DexExecutionResult::Int(-1))
    );
    assert_eq!(
        program.invoke_builtin_method(&close, Some(&test_invocation(vec![reader.clone()]))),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&read_char, Some(&test_invocation(vec![reader]))),
        None
    );
}

#[test]
fn builtin_data_output_stream_int_writes_return_void() {
    let program = DexProgram::new(Vec::new());
    let write_byte = DexMethodId {
        class_descriptor: "Ljava/io/DataOutputStream;".to_owned(),
        name: "writeByte".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let write_short = DexMethodId {
        class_descriptor: "Ljava/io/DataOutputStream;".to_owned(),
        name: "writeShort".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let write_int = DexMethodId {
        class_descriptor: "Ljava/io/DataOutputStream;".to_owned(),
        name: "writeInt".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let write_boolean = DexMethodId {
        class_descriptor: "Ljava/io/DataOutputStream;".to_owned(),
        name: "writeBoolean".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VZ".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Z".to_owned()],
        },
    };
    let write_utf = DexMethodId {
        class_descriptor: "Ljava/io/DataOutputStream;".to_owned(),
        name: "writeUTF".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let flush = DexMethodId {
        class_descriptor: "Ljava/io/DataOutputStream;".to_owned(),
        name: "flush".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let size = DexMethodId {
        class_descriptor: "Ljava/io/DataOutputStream;".to_owned(),
        name: "size".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let int_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("stream")),
        DexValue::Int(1),
    ]);
    let stream_invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from("stream"))]);
    let string_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("stream")),
        DexValue::String(std::rc::Rc::from("isCompressed")),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&write_byte, Some(&int_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&write_short, Some(&int_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&write_int, Some(&int_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&write_boolean, Some(&int_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&write_utf, Some(&string_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&flush, Some(&stream_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&size, Some(&stream_invocation)),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_byte_array_output_stream_to_byte_array_returns_empty_byte_array() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Ljava/io/ByteArrayOutputStream;".to_owned(),
        name: "toByteArray".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "[B".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![DexValue::Reference(std::rc::Rc::from("stream"))]);
    let expected = DexValue::Object(DexObjectRef {
        id: 1,
        class_descriptor: std::rc::Rc::from("[B"),
    });

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&invocation)),
        Some(DexExecutionResult::Value(expected.clone()))
    );
    assert_eq!(
        program
            .runtime
            .borrow()
            .array_entries
            .get(&map_identity_key(&expected)),
        Some(&Vec::new())
    );
}

#[test]
fn libcore_closeable_close_dispatches_to_the_receiver_class_and_releases_the_writer() {
    let fixture = GuestIoFixture::new("closeable_close");
    let program = fixture.program();
    let directory = fixture.private.join("files");
    fs::create_dir_all(&directory).expect("files directory should exist");
    let instance = crate::fs_notify::Instance::new();
    let watched = crate::capability_fs::Directory::open_ambient(&directory).unwrap();
    let wd = instance
        .add_watch(
            crate::fs_notify::InodeKey::of(watched.as_fd()).unwrap(),
            crate::fs_notify::WIRED_EVENTS,
        )
        .unwrap();
    let event = |mask: u32, name: &str| crate::fs_notify::Event {
        wd,
        mask,
        name: Some(std::ffi::CString::new(name).unwrap()),
    };
    let close = builtin_method("Ljava/io/Closeable;", "close", "V", &[]);
    let construct = |class: &str, name: &str| {
        let receiver = program.allocate_object(class.to_owned());
        let constructor = builtin_method(class, "<init>", "V", &["Ljava/io/File;"]);
        assert_eq!(
            program.invoke_builtin_method(
                &constructor,
                Some(&test_invocation(vec![
                    receiver.clone(),
                    file_value(&format!("/data/user/0/com.example/files/{name}")),
                ]))
            ),
            Some(DexExecutionResult::Void)
        );
        receiver
    };

    for (class, name) in [
        ("Ljava/io/FileOutputStream;", "stream.bin"),
        ("Ljava/io/FileWriter;", "writer.txt"),
    ] {
        let receiver = construct(class, name);
        assert_eq!(
            instance.take_events(),
            vec![event(crate::fs_notify::IN_CREATE, name)]
        );
        assert_eq!(
            program.invoke_builtin_method(&close, Some(&test_invocation(vec![receiver]))),
            Some(DexExecutionResult::Void),
            "{class}"
        );
        assert_eq!(
            instance.take_events(),
            vec![event(crate::fs_notify::IN_CLOSE_WRITE, name)],
            "Closeable.java close on a {class} receiver runs that class's close, which releases the writable descriptor"
        );
    }

    assert_eq!(
        program.invoke_builtin_method(
            &close,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("stream")
            )]))
        ),
        None,
        "a receiver class with no close implementation stays a visible missing method"
    );
}

#[test]
fn libcore_file_channel_impl_close_closes_the_parent_stream_and_releases_its_descriptor() {
    // cite: libcore AbstractInterruptibleChannel.java#close
    // cite: libcore FileChannelImpl.java#implCloseChannel, parent
    // cite: libcore FileOutputStream.java#getChannel, close
    // cite: libcore FileInputStream.java#getChannel, close
    // cite: libcore RandomAccessFile.java#getChannel, close
    use crate::fs_notify::{IN_CLOSE_WRITE, IN_DELETE_SELF, IN_IGNORED};
    let fixture = GuestIoFixture::new("channel_close_parent");
    let program = fixture.program();
    let directory = fixture.private.join("files");
    fs::create_dir_all(&directory).expect("files directory should exist");
    let instance = crate::fs_notify::Instance::new();
    let close_channel = builtin_method(
        "Ljava/nio/channels/spi/AbstractInterruptibleChannel;",
        "close",
        "V",
        &[],
    );
    let delete = java_io_method("Ljava/io/File;", "delete", "Z", &[]);
    let event = |wd: i32, mask: u32| crate::fs_notify::Event {
        wd,
        mask,
        name: None,
    };
    let channel_of = |class: &str, name: &str, mode: Option<&str>| {
        let guest = format!("/data/user/0/com.example/files/{name}");
        let receiver = program.allocate_object(class.to_owned());
        let mut arguments = vec![receiver.clone(), file_value(&guest)];
        let mut parameters = vec!["Ljava/io/File;"];
        if let Some(mode) = mode {
            arguments.push(DexValue::String(std::rc::Rc::from(mode)));
            parameters.push("Ljava/lang/String;");
        }
        let constructor = builtin_method(class, "<init>", "V", &parameters);
        assert_eq!(
            program.invoke_builtin_method(&constructor, Some(&test_invocation(arguments))),
            Some(DexExecutionResult::Void),
            "{class}"
        );
        let get_channel =
            builtin_method(class, "getChannel", "Ljava/nio/channels/FileChannel;", &[]);
        let DexExecutionResult::Value(channel) =
            invoke_value(&program, &get_channel, vec![receiver.clone()])
        else {
            panic!("{class}.getChannel should return a channel");
        };
        (guest, channel)
    };
    let watch = |name: &str, mask: u32| {
        let host = fs::File::open(directory.join(name)).expect("watched file");
        instance
            .add_watch(crate::fs_notify::InodeKey::of(&host).unwrap(), mask)
            .unwrap()
    };

    for (class, name, mode) in [
        ("Ljava/io/FileInputStream;", "in.bin", None),
        ("Ljava/io/RandomAccessFile;", "read.bin", Some("r")),
    ] {
        fs::write(directory.join(name), b"x").expect("readable file");
        let (guest, channel) = channel_of(class, name, mode);
        let wd = watch(name, IN_DELETE_SELF);
        assert_eq!(
            invoke_value(&program, &close_channel, vec![channel.clone()]),
            DexExecutionResult::Void
        );
        assert_eq!(
            invoke_value(&program, &close_channel, vec![channel]),
            DexExecutionResult::Void,
            "AbstractInterruptibleChannel.java close returns at once on a closed channel"
        );
        assert_eq!(
            invoke_value(&program, &delete, vec![file_value(&guest)]),
            DexExecutionResult::Int(1)
        );
        assert_eq!(
            instance.take_events(),
            vec![event(wd, IN_DELETE_SELF), event(wd, IN_IGNORED)],
            "FileChannelImpl.java implCloseChannel closes the {class} parent, so its descriptor no longer holds the deleted file"
        );
    }

    fs::write(directory.join("out.bin"), b"").expect("writable file");
    let wd = watch("out.bin", IN_CLOSE_WRITE);
    let (_, channel) = channel_of("Ljava/io/FileOutputStream;", "out.bin", None);
    let read = java_io_method(
        "Ljava/nio/channels/FileChannel;",
        "read",
        "I",
        &["Ljava/nio/ByteBuffer;"],
    );
    let buffer = allocate_byte_buffer(&program, 1);
    assert_eq!(
        thrown_class(invoke_value(&program, &read, vec![channel.clone(), buffer])),
        "Ljava/nio/channels/NonReadableChannelException;",
        "FileOutputStream.java getChannel opens the channel with readable false"
    );
    assert_eq!(
        invoke_value(&program, &close_channel, vec![channel]),
        DexExecutionResult::Void
    );
    assert_eq!(
        instance.take_events(),
        vec![event(wd, IN_CLOSE_WRITE)],
        "FileChannelImpl.java implCloseChannel closes the FileOutputStream parent, which releases the writable descriptor"
    );
}

fn file_path_after_constructor(
    program: &DexProgram,
    parameters: &[&str],
    arguments: Vec<DexValue>,
) -> Option<DexValue> {
    let constructor = java_io_method("Ljava/io/File;", "<init>", "V", parameters);
    let file = object_value(500, "Ljava/io/File;");
    let mut invocation_arguments = vec![file.clone()];
    invocation_arguments.extend(arguments);
    let result = program
        .invoke_builtin_method(&constructor, Some(&test_invocation(invocation_arguments)))?;
    assert_eq!(result, DexExecutionResult::Void);

    program.read_stored_instance_field(
        &file,
        &DexFieldId {
            class_descriptor: "Ljava/io/File;".to_owned(),
            type_descriptor: "Ljava/lang/String;".to_owned(),
            name: "path".to_owned(),
        },
    )
}

fn string_value(text: &str) -> DexValue {
    DexValue::String(std::rc::Rc::from(text))
}

#[test]
fn libcore_file_string_parent_child_constructor_resolves_like_unix_file_system() {
    // cite: libcore File.java#File
    // cite: libcore UnixFileSystem.java#normalize, resolve
    let program = DexProgram::new(Vec::new());
    let parameters = ["Ljava/lang/String;", "Ljava/lang/String;"];
    let cases = [
        (Some("/data/user/0/x"), "files", "/data/user/0/x/files"),
        (Some("/"), "a", "/a"),
        (Some("/a/"), "/b/", "/a/b"),
        (Some("/a"), "", "/a"),
        (Some(""), "b", "b"),
        (None, "b//c/", "b/c"),
    ];

    for (parent, child, expected) in cases {
        let parent = parent.map_or(DexValue::Null, string_value);
        assert_eq!(
            file_path_after_constructor(&program, &parameters, vec![parent, string_value(child)]),
            Some(string_value(expected)),
            "child {child}"
        );
    }
}

#[test]
fn libcore_file_constructors_throw_null_pointer_exception_for_a_null_child() {
    // cite: libcore File.java#File
    let program = DexProgram::new(Vec::new());
    let file = object_value(501, "Ljava/io/File;");
    let path_field = DexFieldId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "path".to_owned(),
    };

    for parent_type in ["Ljava/lang/String;", "Ljava/io/File;"] {
        let constructor = java_io_method(
            "Ljava/io/File;",
            "<init>",
            "V",
            &[parent_type, "Ljava/lang/String;"],
        );
        let Some(DexExecutionResult::Throw(exception)) = program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                file.clone(),
                string_value("/a"),
                DexValue::Null,
            ])),
        ) else {
            panic!("null child should throw for {parent_type}");
        };
        assert!(program.value_matches_type(&exception, "Ljava/lang/NullPointerException;"));
    }
    assert_eq!(program.read_stored_instance_field(&file, &path_field), None);
}

#[test]
fn libcore_file_parent_file_child_constructor_resolves_a_parent_built_from_two_strings() {
    // cite: libcore File.java#File
    // cite: libcore UnixFileSystem.java#resolve, getDefaultParent
    let program = DexProgram::new(Vec::new());
    let string_constructor = java_io_method(
        "Ljava/io/File;",
        "<init>",
        "V",
        &["Ljava/lang/String;", "Ljava/lang/String;"],
    );
    let file_constructor = java_io_method(
        "Ljava/io/File;",
        "<init>",
        "V",
        &["Ljava/io/File;", "Ljava/lang/String;"],
    );
    let parent = object_value(502, "Ljava/io/File;");
    let child = object_value(503, "Ljava/io/File;");
    let path_field = DexFieldId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "path".to_owned(),
    };

    assert_eq!(
        program.invoke_builtin_method(
            &string_constructor,
            Some(&test_invocation(vec![
                parent.clone(),
                string_value("/data/user/0/x"),
                string_value("cache"),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &file_constructor,
            Some(&test_invocation(vec![
                child.clone(),
                parent,
                string_value("room.db"),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&child, &path_field),
        Some(string_value("/data/user/0/x/cache/room.db"))
    );
}

#[test]
fn libcore_file_input_stream_string_constructor_opens_and_rejects_like_file_input_stream() {
    // cite: libcore FileInputStream.java#FileInputStream
    // cite: libcore File.java#isInvalid
    let fixture = GuestIoFixture::new("input_stream_string");
    let program = fixture.program();
    let constructor = java_io_method(
        "Ljava/io/FileInputStream;",
        "<init>",
        "V",
        &["Ljava/lang/String;"],
    );
    let path = fixture.private.join("files/data.bin");
    fs::create_dir_all(path.parent().expect("test file should have a parent"))
        .expect("test directory should be created");
    fs::write(&path, b"data").expect("test file should be written");
    let guest_path = "/data/user/0/com.example/files/data.bin";
    let path_field = DexFieldId {
        class_descriptor: "Ljava/io/FileInputStream;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "path".to_owned(),
    };
    let message_field = DexFieldId {
        class_descriptor: "Ljava/lang/Throwable;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "detailMessage".to_owned(),
    };

    let stream = object_value(510, "Ljava/io/FileInputStream;");
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                stream.clone(),
                string_value("/data/user/0/com.example//files/data.bin/")
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&stream, &path_field),
        Some(string_value(guest_path))
    );

    let Some(DexExecutionResult::Throw(missing)) = program.invoke_builtin_method(
        &constructor,
        Some(&test_invocation(vec![
            object_value(511, "Ljava/io/FileInputStream;"),
            string_value("/data/user/0/com.example/files/missing.bin"),
        ])),
    ) else {
        panic!("missing file should throw");
    };
    assert!(program.value_matches_type(&missing, "Ljava/io/FileNotFoundException;"));

    let Some(DexExecutionResult::Throw(directory)) = program.invoke_builtin_method(
        &constructor,
        Some(&test_invocation(vec![
            object_value(512, "Ljava/io/FileInputStream;"),
            string_value("/data/user/0/com.example/files"),
        ])),
    ) else {
        panic!("directory should throw");
    };
    assert!(program.value_matches_type(&directory, "Ljava/io/FileNotFoundException;"));

    let Some(DexExecutionResult::Throw(invalid)) = program.invoke_builtin_method(
        &constructor,
        Some(&test_invocation(vec![
            object_value(513, "Ljava/io/FileInputStream;"),
            string_value("/data/user/0/com.example/files/a\0b"),
        ])),
    ) else {
        panic!("NUL path should throw");
    };
    assert!(program.value_matches_type(&invalid, "Ljava/io/FileNotFoundException;"));
    assert_eq!(
        program.read_stored_instance_field(&invalid, &message_field),
        Some(string_value("Invalid file path"))
    );

    let Some(DexExecutionResult::Throw(null_name)) = program.invoke_builtin_method(
        &constructor,
        Some(&test_invocation(vec![
            object_value(514, "Ljava/io/FileInputStream;"),
            DexValue::Null,
        ])),
    ) else {
        panic!("null name should throw");
    };
    assert!(program.value_matches_type(&null_name, "Ljava/lang/NullPointerException;"));
}

// cite: libcore FileReader.java#FileReader
// cite: frameworks-base android_util_Process.cpp#android_os_Process_getPidsForCommands
#[test]
fn file_reader_path_reads_guest_files_and_own_proc_command_line() {
    let fixture = GuestIoFixture::new("file-reader-path");
    fs::create_dir_all(fixture.private.join("files")).unwrap();
    fs::write(fixture.private.join("files/reader.txt"), "aé").unwrap();
    let program = fixture.program();
    let constructor = java_io_method(
        "Ljava/io/FileReader;",
        "<init>",
        "V",
        &["Ljava/lang/String;"],
    );
    let reader = object_value(520, "Ljava/io/FileReader;");
    let string = |text: &str| DexValue::String(std::rc::Rc::from(text));
    let open = |path| {
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![reader.clone(), path])),
        )
    };
    assert_eq!(
        open(string("/data/user/0/com.example/files/reader.txt")),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        buffered_reader_read_all(&program, &reader, 3),
        vec![97, 233, -1]
    );

    let process_id_path = format!("/proc/{BOOTSTRAP_PROCESS_ID}/cmdline");
    for path in ["/proc/self/cmdline", process_id_path.as_str()] {
        assert_eq!(open(string(path)), Some(DexExecutionResult::Void));
        let mut expected: Vec<i32> = program
            .bootstrap_process_name()
            .encode_utf16()
            .map(i32::from)
            .collect();
        expected.extend([0, -1]);
        assert_eq!(
            buffered_reader_read_all(&program, &reader, expected.len()),
            expected
        );
    }

    for (path, class) in [
        (DexValue::Null, "Ljava/lang/NullPointerException;"),
        (string("/proc/1/cmdline"), "Ljava/io/FileNotFoundException;"),
        (
            string("/data/user/0/com.other/files/reader.txt"),
            "Ljava/io/FileNotFoundException;",
        ),
    ] {
        let Some(DexExecutionResult::Throw(error)) = open(path) else {
            panic!("expected a guest exception");
        };
        assert!(program.value_matches_type(&error, class));
    }
}

fn buffered_reader_read_all(program: &DexProgram, reader: &DexValue, count: usize) -> Vec<i32> {
    let read = java_io_method("Ljava/io/BufferedReader;", "read", "I", &[]);
    (0..count)
        .map(|_| {
            match program.invoke_builtin_method(&read, Some(&test_invocation(vec![reader.clone()])))
            {
                Some(DexExecutionResult::Int(value)) => value,
                other => panic!("unexpected read result {other:?}"),
            }
        })
        .collect()
}

fn buffered_reader_over(program: &DexProgram, bytes: &[u8]) -> DexValue {
    let reader = object_value(520, "Ljava/io/BufferedReader;");
    program.runtime.borrow_mut().input_stream_values.insert(
        map_identity_key(&reader),
        InputStreamState {
            bytes: bytes.to_vec(),
            position: 0,
            pending_low_surrogate: None,
        },
    );

    reader
}

#[test]
fn libcore_buffered_reader_read_returns_utf16_units_and_minus_one_at_end() {
    // cite: libcore BufferedReader.java#read, fill
    // cite: libcore StreamDecoder.java#read0
    let program = DexProgram::new(Vec::new());
    let cases: [(&[u8], Vec<i32>); 7] = [
        (b"ab", vec![97, 98, -1, -1]),
        ("\u{e9}".as_bytes(), vec![233, -1]),
        ("\u{20ac}".as_bytes(), vec![8364, -1]),
        ("\u{1f600}".as_bytes(), vec![0xD83D, 0xDE00, -1]),
        (&[0xFF, b'a'], vec![65533, 97, -1]),
        (&[0xE2, 0x82], vec![65533, -1]),
        (b"", vec![-1]),
    ];

    for (bytes, expected) in cases {
        let reader = buffered_reader_over(&program, bytes);
        assert_eq!(
            buffered_reader_read_all(&program, &reader, expected.len()),
            expected,
            "bytes {bytes:?}"
        );
    }
}

#[test]
fn libcore_buffered_reader_read_continues_after_read_line() {
    // cite: libcore BufferedReader.java#read, readLine
    let program = DexProgram::new(Vec::new());
    let read_line = java_io_method(
        "Ljava/io/BufferedReader;",
        "readLine",
        "Ljava/lang/String;",
        &[],
    );
    let reader = buffered_reader_over(&program, b"ab\ncd");

    assert_eq!(
        program.invoke_builtin_method(&read_line, Some(&test_invocation(vec![reader.clone()]))),
        Some(DexExecutionResult::Value(string_value("ab")))
    );
    assert_eq!(
        buffered_reader_read_all(&program, &reader, 3),
        vec![99, 100, -1]
    );
}

fn java_io_method(
    class_descriptor: &str,
    name: &str,
    return_type: &str,
    parameters: &[&str],
) -> DexMethodId {
    DexMethodId {
        class_descriptor: class_descriptor.to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: String::new(),
            return_type: return_type.to_owned(),
            parameters: parameters
                .iter()
                .map(|parameter| (*parameter).to_owned())
                .collect(),
        },
    }
}

fn file_value(path: &str) -> DexValue {
    DexValue::Reference(std::rc::Rc::from(format!("Ljava/io/File;:{path}")))
}

fn object_value(id: u64, class_descriptor: &str) -> DexValue {
    DexValue::Object(DexObjectRef {
        id,
        class_descriptor: std::rc::Rc::from(class_descriptor),
    })
}

struct GuestIoFixture {
    root: PathBuf,
    private: PathBuf,
    device_private: PathBuf,
    shared: PathBuf,
}

impl GuestIoFixture {
    fn new(name: &str) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "winedroid-java-io-{name}-{}-{id}",
            std::process::id()
        ));
        let private = root.join("apps/com.example/data");
        let device_private = root.join("apps/com.example/data_de");
        let shared = root.join("storage");
        fs::create_dir_all(&private).expect("private directory should exist");
        fs::create_dir_all(&device_private).expect("device private directory should exist");
        fs::create_dir_all(&shared).expect("shared directory should exist");

        Self {
            root,
            private,
            device_private,
            shared,
        }
    }

    fn program(&self) -> DexProgram {
        DexProgram::with_guest_filesystem(
            Vec::new(),
            "com.example",
            self.private.clone(),
            self.shared.clone(),
        )
        .expect("guest filesystem program should initialize")
    }

    fn program_with_external_storage_policy(
        &self,
        external_storage_policy: ExternalStoragePolicy,
    ) -> DexProgram {
        DexProgram::with_guest_filesystem_and_external_storage_policy(
            Vec::new(),
            "com.example",
            self.private.clone(),
            self.shared.clone(),
            external_storage_policy,
        )
        .expect("guest filesystem program should initialize")
    }
}

impl Drop for GuestIoFixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).ok();
    }
}

#[test]
fn libcore_file_path_constructor_throws_null_pointer_exception_for_a_null_pathname() {
    // cite: libcore File.java#File
    let program = DexProgram::new(Vec::new());
    let constructor = builtin_method("Ljava/io/File;", "<init>", "V", &["Ljava/lang/String;"]);
    let path_field = DexFieldId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "path".to_owned(),
    };

    for pathname in [DexValue::Null, DexValue::Int(0)] {
        let file = program.allocate_object("Ljava/io/File;".to_owned());
        let exception = thrown_exception(program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![file.clone(), pathname])),
        ));
        assert!(program.value_matches_type(&exception, "Ljava/lang/NullPointerException;"));
        assert_eq!(thrown_detail_message(&program, &exception), None);
        assert_eq!(program.read_stored_instance_field(&file, &path_field), None);
    }

    let file = program.allocate_object("Ljava/io/File;".to_owned());
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                file.clone(),
                DexValue::String(std::rc::Rc::from("a//b/")),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.read_stored_instance_field(&file, &path_field),
        Some(DexValue::String(std::rc::Rc::from("a/b")))
    );
}

#[test]
fn libcore_file_equals_compares_paths_of_file_arguments_only() {
    // cite: libcore File.java#equals, compareTo
    // cite: libcore UnixFileSystem.java#compare
    let program = DexProgram::new(Vec::new());
    let equals = builtin_method("Ljava/io/File;", "equals", "Z", &["Ljava/lang/Object;"]);
    let path_field = DexFieldId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "path".to_owned(),
    };
    let object_file = program.allocate_object("Ljava/io/File;".to_owned());
    program.write_stored_instance_field(
        &object_file,
        &path_field,
        DexValue::String(std::rc::Rc::from("/a/b")),
    );
    let run = |receiver: &DexValue, other: DexValue| {
        program.invoke_builtin_method(
            &equals,
            Some(&test_invocation(vec![receiver.clone(), other])),
        )
    };

    assert_eq!(
        run(&file_value("/a/b"), file_value("/a/b")),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        run(&object_file, file_value("/a/b")),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        run(&file_value("/a/b"), object_file.clone()),
        Some(DexExecutionResult::Int(1))
    );
    assert_eq!(
        run(&file_value("/a/b"), file_value("/a/c")),
        Some(DexExecutionResult::Int(0))
    );
    assert_eq!(
        run(&file_value("/a/b"), file_value("/A/b")),
        Some(DexExecutionResult::Int(0))
    );
    for other in [
        DexValue::Null,
        DexValue::Int(0),
        DexValue::String(std::rc::Rc::from("/a/b")),
    ] {
        assert_eq!(
            run(&file_value("/a/b"), other),
            Some(DexExecutionResult::Int(0))
        );
    }
}

#[test]
fn libcore_file_get_free_space_reports_statvfs_free_bytes_or_zero() {
    // cite: libcore File.java#getFreeSpace, isInvalid
    // cite: libcore UnixFileSystem_md.c#getSpace0
    let fixture = GuestIoFixture::new("free_space");
    let program = fixture.program();
    let get_free_space = builtin_method("Ljava/io/File;", "getFreeSpace", "J", &[]);
    let free_space = |path: &str| {
        program.invoke_builtin_method(
            &get_free_space,
            Some(&test_invocation(vec![file_value(path)])),
        )
    };
    let stat_fs = object_value(1, "Landroid/os/StatFs;");
    let stat_fs_constructor = builtin_method(
        "Landroid/os/StatFs;",
        "<init>",
        "V",
        &["Ljava/lang/String;"],
    );
    let stat_fs_block_size = builtin_method("Landroid/os/StatFs;", "getBlockSizeLong", "J", &[]);
    let stat_fs_total_bytes = builtin_method("Landroid/os/StatFs;", "getTotalBytes", "J", &[]);
    assert_eq!(
        program.invoke_builtin_method(
            &stat_fs_constructor,
            Some(&test_invocation(vec![
                stat_fs.clone(),
                DexValue::String(std::rc::Rc::from("/data/user/0/com.example/files")),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    let stat_fs_long = |method: &DexMethodId| {
        let Some(DexExecutionResult::Value(DexValue::Long(value))) =
            program.invoke_builtin_method(method, Some(&test_invocation(vec![stat_fs.clone()])))
        else {
            panic!("StatFs getter should return a long value");
        };
        value
    };
    let block_size = stat_fs_long(&stat_fs_block_size);
    let total_bytes = stat_fs_long(&stat_fs_total_bytes);

    let Some(DexExecutionResult::Value(DexValue::Long(value))) =
        free_space("/data/user/0/com.example/files")
    else {
        panic!("getFreeSpace should return a long value");
    };
    assert!(value > 0 && value <= total_bytes);
    assert_eq!(value % block_size, 0);
    assert_eq!(
        free_space("/etc"),
        Some(DexExecutionResult::Value(DexValue::Long(0)))
    );
    assert_eq!(
        free_space("/data/user/0/com.example/files\0x"),
        Some(DexExecutionResult::Value(DexValue::Long(0)))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &get_free_space,
            Some(&test_invocation(vec![DexValue::Null]))
        ),
        None
    );
}

fn invoke_value(
    program: &DexProgram,
    method: &DexMethodId,
    arguments: Vec<DexValue>,
) -> DexExecutionResult {
    program
        .invoke_builtin_method(method, Some(&test_invocation(arguments)))
        .unwrap_or_else(|| panic!("{} should be a builtin", method.display_signature()))
}

fn int_array_bytes(program: &DexProgram, buffer: &DexValue, bytes: &[u8]) {
    let array_method = java_io_method("Ljava/nio/ByteBuffer;", "array", "[B", &[]);
    let DexExecutionResult::Value(array) =
        invoke_value(program, &array_method, vec![buffer.clone()])
    else {
        panic!("array() should return the backing array");
    };
    for (index, byte) in bytes.iter().enumerate() {
        program
            .write_array_value(&array, index as i32, DexValue::Int(i32::from(*byte as i8)))
            .expect("backing array write");
    }
}

fn allocate_byte_buffer(program: &DexProgram, capacity: i32) -> DexValue {
    let allocate = java_io_method(
        "Ljava/nio/ByteBuffer;",
        "allocate",
        "Ljava/nio/ByteBuffer;",
        &["I"],
    );
    let DexExecutionResult::Value(buffer) =
        invoke_value(program, &allocate, vec![DexValue::Int(capacity)])
    else {
        panic!("allocate should return a buffer");
    };
    buffer
}

fn little_endian() -> DexValue {
    DexValue::Reference(std::rc::Rc::from("Ljava/nio/ByteOrder;:LITTLE_ENDIAN"))
}

fn thrown_class(result: DexExecutionResult) -> String {
    let DexExecutionResult::Throw(exception) = result else {
        panic!("expected a thrown exception, got {result}");
    };
    value_class_descriptor(&exception)
        .expect("exception class")
        .to_owned()
}

#[test]
fn libcore_heap_byte_buffer_reads_integers_in_the_configured_byte_order() {
    // cite: libcore HeapByteBuffer.java#getShort, getInt, getLong
    let program = DexProgram::new(Vec::new());
    let buffer = allocate_byte_buffer(&program, 8);
    int_array_bytes(&program, &buffer, &[0x01, 0x02, 0x03, 0x84, 0, 0, 0, 0x80]);
    let get_int = java_io_method("Ljava/nio/ByteBuffer;", "getInt", "I", &["I"]);
    let get_short = java_io_method("Ljava/nio/ByteBuffer;", "getShort", "S", &["I"]);
    let get_long = java_io_method("Ljava/nio/ByteBuffer;", "getLong", "J", &["I"]);
    let order = java_io_method(
        "Ljava/nio/ByteBuffer;",
        "order",
        "Ljava/nio/ByteBuffer;",
        &["Ljava/nio/ByteOrder;"],
    );

    assert_eq!(
        invoke_value(&program, &get_int, vec![buffer.clone(), DexValue::Int(0)]),
        DexExecutionResult::Int(0x0102_0384_u32 as i32)
    );
    assert_eq!(
        invoke_value(&program, &get_short, vec![buffer.clone(), DexValue::Int(2)]),
        DexExecutionResult::Int(0x0384)
    );
    assert_eq!(
        invoke_value(&program, &order, vec![buffer.clone(), little_endian()]),
        DexExecutionResult::Value(buffer.clone())
    );
    assert_eq!(
        invoke_value(&program, &get_int, vec![buffer.clone(), DexValue::Int(0)]),
        DexExecutionResult::Int(0x8403_0201_u32 as i32)
    );
    assert_eq!(
        invoke_value(&program, &get_short, vec![buffer.clone(), DexValue::Int(2)]),
        DexExecutionResult::Int(0x8403_u16 as i16 as i32)
    );
    assert_eq!(
        invoke_value(&program, &get_long, vec![buffer, DexValue::Int(0)]),
        DexExecutionResult::Value(DexValue::Long(0x8000_0000_8403_0201_u64 as i64))
    );
}

#[test]
fn libcore_heap_byte_buffer_relative_and_absolute_reads_enforce_limit_and_position() {
    // cite: libcore Buffer.java#nextGetIndex, checkIndex, position, limit
    let program = DexProgram::new(Vec::new());
    let buffer = allocate_byte_buffer(&program, 6);
    int_array_bytes(&program, &buffer, &[0, 0, 0, 7, 0, 9]);
    let get_int = java_io_method("Ljava/nio/ByteBuffer;", "getInt", "I", &[]);
    let get_int_at = java_io_method("Ljava/nio/ByteBuffer;", "getInt", "I", &["I"]);
    let position = java_io_method("Ljava/nio/Buffer;", "position", "I", &[]);
    let set_position = java_io_method("Ljava/nio/Buffer;", "position", "Ljava/nio/Buffer;", &["I"]);
    let remaining = java_io_method("Ljava/nio/Buffer;", "remaining", "I", &[]);
    let has_remaining = java_io_method("Ljava/nio/Buffer;", "hasRemaining", "Z", &[]);

    assert_eq!(
        invoke_value(&program, &get_int, vec![buffer.clone()]),
        DexExecutionResult::Int(7)
    );
    assert_eq!(
        invoke_value(&program, &position, vec![buffer.clone()]),
        DexExecutionResult::Int(4)
    );
    assert_eq!(
        invoke_value(&program, &remaining, vec![buffer.clone()]),
        DexExecutionResult::Int(2)
    );
    assert_eq!(
        thrown_class(invoke_value(&program, &get_int, vec![buffer.clone()])),
        "Ljava/nio/BufferUnderflowException;"
    );
    assert_eq!(
        invoke_value(&program, &position, vec![buffer.clone()]),
        DexExecutionResult::Int(4)
    );
    assert_eq!(
        thrown_class(invoke_value(
            &program,
            &get_int_at,
            vec![buffer.clone(), DexValue::Int(3)]
        )),
        "Ljava/lang/IndexOutOfBoundsException;"
    );
    assert_eq!(
        thrown_class(invoke_value(
            &program,
            &get_int_at,
            vec![buffer.clone(), DexValue::Int(-1)]
        )),
        "Ljava/lang/IndexOutOfBoundsException;"
    );
    assert_eq!(
        thrown_class(invoke_value(
            &program,
            &set_position,
            vec![buffer.clone(), DexValue::Int(7)]
        )),
        "Ljava/lang/IllegalArgumentException;"
    );
    invoke_value(
        &program,
        &set_position,
        vec![buffer.clone(), DexValue::Int(6)],
    );
    assert_eq!(
        invoke_value(&program, &has_remaining, vec![buffer]),
        DexExecutionResult::Int(0)
    );
}

#[test]
fn libcore_heap_byte_buffer_slice_shares_the_array_and_resets_byte_order() {
    // cite: libcore HeapByteBuffer.java#slice
    // cite: libcore ByteBuffer.java#array, arrayOffset
    let program = DexProgram::new(Vec::new());
    let buffer = allocate_byte_buffer(&program, 8);
    int_array_bytes(&program, &buffer, &[0, 0, 0, 0, 0x01, 0x02, 0x03, 0x04]);
    let order = java_io_method(
        "Ljava/nio/ByteBuffer;",
        "order",
        "Ljava/nio/ByteBuffer;",
        &["Ljava/nio/ByteOrder;"],
    );
    let get_order = java_io_method(
        "Ljava/nio/ByteBuffer;",
        "order",
        "Ljava/nio/ByteOrder;",
        &[],
    );
    let set_position = java_io_method("Ljava/nio/Buffer;", "position", "Ljava/nio/Buffer;", &["I"]);
    let slice = java_io_method(
        "Ljava/nio/ByteBuffer;",
        "slice",
        "Ljava/nio/ByteBuffer;",
        &[],
    );
    let array = java_io_method("Ljava/nio/ByteBuffer;", "array", "[B", &[]);
    let array_offset = java_io_method("Ljava/nio/ByteBuffer;", "arrayOffset", "I", &[]);
    let get_int = java_io_method("Ljava/nio/ByteBuffer;", "getInt", "I", &[]);
    let capacity = java_io_method("Ljava/nio/Buffer;", "capacity", "I", &[]);

    invoke_value(&program, &order, vec![buffer.clone(), little_endian()]);
    invoke_value(
        &program,
        &set_position,
        vec![buffer.clone(), DexValue::Int(4)],
    );
    let DexExecutionResult::Value(sliced) = invoke_value(&program, &slice, vec![buffer.clone()])
    else {
        panic!("slice should return a buffer");
    };

    assert_eq!(
        invoke_value(&program, &array_offset, vec![sliced.clone()]),
        DexExecutionResult::Int(4)
    );
    assert_eq!(
        invoke_value(&program, &capacity, vec![sliced.clone()]),
        DexExecutionResult::Int(4)
    );
    assert_eq!(
        invoke_value(&program, &array, vec![sliced.clone()]),
        invoke_value(&program, &array, vec![buffer])
    );
    assert_eq!(
        invoke_value(&program, &get_order, vec![sliced.clone()]),
        DexExecutionResult::Value(DexValue::Reference(std::rc::Rc::from(
            "Ljava/nio/ByteOrder;:BIG_ENDIAN"
        )))
    );
    assert_eq!(
        invoke_value(&program, &get_int, vec![sliced]),
        DexExecutionResult::Int(0x0102_0304)
    );
}

#[test]
fn libcore_random_access_file_read_only_channel_reads_at_its_position_until_eof() {
    // cite: libcore RandomAccessFile.java#RandomAccessFile, getChannel, close
    // cite: libcore FileChannelImpl.java#size, position, read, ensureOpen
    let fixture = GuestIoFixture::new("random_access_file");
    let program = fixture.program();
    let path = fixture.private.join("files/archive.bin");
    fs::create_dir_all(path.parent().expect("parent")).expect("files directory");
    fs::write(&path, [10u8, 11, 12, 13, 14, 15, 16, 17, 250, 251]).expect("archive bytes");
    let guest_path = "/data/user/0/com.example/files/archive.bin";
    let constructor = java_io_method(
        "Ljava/io/RandomAccessFile;",
        "<init>",
        "V",
        &["Ljava/lang/String;", "Ljava/lang/String;"],
    );
    let get_channel = java_io_method(
        "Ljava/io/RandomAccessFile;",
        "getChannel",
        "Ljava/nio/channels/FileChannel;",
        &[],
    );
    let close = java_io_method("Ljava/io/RandomAccessFile;", "close", "V", &[]);
    let size = java_io_method("Ljava/nio/channels/FileChannel;", "size", "J", &[]);
    let channel_position = java_io_method("Ljava/nio/channels/FileChannel;", "position", "J", &[]);
    let set_channel_position = java_io_method(
        "Ljava/nio/channels/FileChannel;",
        "position",
        "Ljava/nio/channels/FileChannel;",
        &["J"],
    );
    let read = java_io_method(
        "Ljava/nio/channels/FileChannel;",
        "read",
        "I",
        &["Ljava/nio/ByteBuffer;"],
    );
    let buffer_position = java_io_method("Ljava/nio/Buffer;", "position", "I", &[]);
    let get_int_at = java_io_method("Ljava/nio/ByteBuffer;", "getInt", "I", &["I"]);
    let string = |value: &str| DexValue::String(std::rc::Rc::from(value));
    let file = program.allocate_object("Ljava/io/RandomAccessFile;".to_owned());

    assert_eq!(
        invoke_value(
            &program,
            &constructor,
            vec![file.clone(), string(guest_path), string("r")]
        ),
        DexExecutionResult::Void
    );
    let DexExecutionResult::Value(channel) =
        invoke_value(&program, &get_channel, vec![file.clone()])
    else {
        panic!("getChannel should return a channel");
    };
    assert_eq!(
        invoke_value(&program, &get_channel, vec![file.clone()]),
        DexExecutionResult::Value(channel.clone())
    );
    assert_eq!(
        invoke_value(&program, &size, vec![channel.clone()]),
        DexExecutionResult::Value(DexValue::Long(10))
    );
    invoke_value(
        &program,
        &set_channel_position,
        vec![channel.clone(), DexValue::Long(6), DexValue::Long(6)],
    );
    let destination = allocate_byte_buffer(&program, 8);
    assert_eq!(
        invoke_value(&program, &read, vec![channel.clone(), destination.clone()]),
        DexExecutionResult::Int(4)
    );
    assert_eq!(
        invoke_value(&program, &buffer_position, vec![destination.clone()]),
        DexExecutionResult::Int(4)
    );
    assert_eq!(
        invoke_value(
            &program,
            &get_int_at,
            vec![destination.clone(), DexValue::Int(0)]
        ),
        DexExecutionResult::Int(0x1011_FAFB_u32 as i32)
    );
    assert_eq!(
        invoke_value(&program, &channel_position, vec![channel.clone()]),
        DexExecutionResult::Value(DexValue::Long(10))
    );
    assert_eq!(
        invoke_value(&program, &read, vec![channel.clone(), destination]),
        DexExecutionResult::Int(-1)
    );
    assert_eq!(
        invoke_value(&program, &close, vec![file]),
        DexExecutionResult::Void
    );
    assert_eq!(
        thrown_class(invoke_value(&program, &size, vec![channel])),
        "Ljava/nio/channels/ClosedChannelException;"
    );
}

#[test]
fn libcore_random_access_file_constructor_rejects_bad_modes_and_missing_files() {
    // cite: libcore RandomAccessFile.java#RandomAccessFile
    let fixture = GuestIoFixture::new("random_access_file_errors");
    let program = fixture.program();
    let constructor = java_io_method(
        "Ljava/io/RandomAccessFile;",
        "<init>",
        "V",
        &["Ljava/lang/String;", "Ljava/lang/String;"],
    );
    let string = |value: &str| DexValue::String(std::rc::Rc::from(value));
    let missing = "/data/user/0/com.example/files/missing.bin";

    assert_eq!(
        thrown_class(invoke_value(
            &program,
            &constructor,
            vec![
                program.allocate_object("Ljava/io/RandomAccessFile;".to_owned()),
                string(missing),
                string("x")
            ]
        )),
        "Ljava/lang/IllegalArgumentException;"
    );
    assert_eq!(
        thrown_class(invoke_value(
            &program,
            &constructor,
            vec![
                program.allocate_object("Ljava/io/RandomAccessFile;".to_owned()),
                DexValue::Null,
                string("r")
            ]
        )),
        "Ljava/lang/NullPointerException;"
    );
    assert_eq!(
        thrown_class(invoke_value(
            &program,
            &constructor,
            vec![
                program.allocate_object("Ljava/io/RandomAccessFile;".to_owned()),
                string(missing),
                string("r")
            ]
        )),
        "Ljava/io/FileNotFoundException;"
    );
    assert_eq!(
        thrown_class(invoke_value(
            &program,
            &constructor,
            vec![
                program.allocate_object("Ljava/io/RandomAccessFile;".to_owned()),
                string(missing),
                string("rw")
            ]
        )),
        "Ljava/io/FileNotFoundException;"
    );
}

#[test]
fn libcore_random_access_file_read_write_mode_creates_and_locks_the_file() {
    // cite: libcore RandomAccessFile.java#RandomAccessFile, getChannel
    // cite: libcore FileChannelImpl.java#lock, writable
    // cite: libcore FileLockImpl.java#release, isValid
    let fixture = GuestIoFixture::new("random_access_file_lock");
    let program = fixture.program();
    let directory = fixture.private.join("files");
    fs::create_dir_all(&directory).expect("files directory");
    fs::write(directory.join("ro.lock"), [1u8]).expect("read-only lock file");
    let constructor = java_io_method(
        "Ljava/io/RandomAccessFile;",
        "<init>",
        "V",
        &["Ljava/io/File;", "Ljava/lang/String;"],
    );
    let get_channel = java_io_method(
        "Ljava/io/RandomAccessFile;",
        "getChannel",
        "Ljava/nio/channels/FileChannel;",
        &[],
    );
    let lock = java_io_method(
        "Ljava/nio/channels/FileChannel;",
        "lock",
        "Ljava/nio/channels/FileLock;",
        &[],
    );
    let release = java_io_method("Ljava/nio/channels/FileLock;", "release", "V", &[]);
    let is_valid = java_io_method("Ljava/nio/channels/FileLock;", "isValid", "Z", &[]);
    let close = java_io_method("Ljava/io/RandomAccessFile;", "close", "V", &[]);
    let channel_of = |name: &str, mode: &str| {
        let file = program.allocate_object("Ljava/io/RandomAccessFile;".to_owned());
        assert_eq!(
            invoke_value(
                &program,
                &constructor,
                vec![
                    file.clone(),
                    file_value(&format!("/data/user/0/com.example/files/{name}")),
                    DexValue::String(std::rc::Rc::from(mode)),
                ]
            ),
            DexExecutionResult::Void
        );
        let DexExecutionResult::Value(channel) =
            invoke_value(&program, &get_channel, vec![file.clone()])
        else {
            panic!("getChannel should return a channel");
        };
        (file, channel)
    };

    let (file, channel) = channel_of("process.lock", "rw");
    assert!(directory.join("process.lock").is_file());
    let DexExecutionResult::Value(file_lock) = invoke_value(&program, &lock, vec![channel]) else {
        panic!("lock should return a FileLock");
    };
    assert_eq!(
        invoke_value(&program, &is_valid, vec![file_lock.clone()]),
        DexExecutionResult::Int(1)
    );
    assert_eq!(
        invoke_value(&program, &release, vec![file_lock.clone()]),
        DexExecutionResult::Void
    );
    assert_eq!(
        invoke_value(&program, &is_valid, vec![file_lock.clone()]),
        DexExecutionResult::Int(0)
    );
    invoke_value(&program, &close, vec![file]);
    assert_eq!(
        thrown_class(invoke_value(&program, &release, vec![file_lock])),
        "Ljava/nio/channels/ClosedChannelException;"
    );

    let (_, read_only_channel) = channel_of("ro.lock", "r");
    assert_eq!(
        thrown_class(invoke_value(&program, &lock, vec![read_only_channel])),
        "Ljava/nio/channels/NonWritableChannelException;"
    );
}

#[test]
fn libcore_random_access_file_get_channel_after_close_is_a_missing_primitive() {
    // cite: libcore RandomAccessFile.java#getChannel, close
    let fixture = GuestIoFixture::new("random_access_file_closed_channel");
    let program = fixture.program();
    let path = fixture.private.join("files/archive.bin");
    fs::create_dir_all(path.parent().expect("parent")).expect("files directory");
    fs::write(&path, [1u8]).expect("archive bytes");
    let string = |value: &str| DexValue::String(std::rc::Rc::from(value));
    let constructor = java_io_method(
        "Ljava/io/RandomAccessFile;",
        "<init>",
        "V",
        &["Ljava/lang/String;", "Ljava/lang/String;"],
    );
    let close = java_io_method("Ljava/io/RandomAccessFile;", "close", "V", &[]);
    let get_channel = java_io_method(
        "Ljava/io/RandomAccessFile;",
        "getChannel",
        "Ljava/nio/channels/FileChannel;",
        &[],
    );
    let file = program.allocate_object("Ljava/io/RandomAccessFile;".to_owned());

    invoke_value(
        &program,
        &constructor,
        vec![
            file.clone(),
            string("/data/user/0/com.example/files/archive.bin"),
            string("r"),
        ],
    );
    invoke_value(&program, &close, vec![file.clone()]);

    assert_eq!(
        program.invoke_builtin_method(&get_channel, Some(&test_invocation(vec![file]))),
        None
    );
}

#[test]
fn libcore_byte_order_static_fields_are_the_two_named_constants() {
    // cite: libcore ByteOrder.java#BIG_ENDIAN, LITTLE_ENDIAN
    let field = |name: &str| DexFieldId {
        class_descriptor: "Ljava/nio/ByteOrder;".to_owned(),
        type_descriptor: "Ljava/nio/ByteOrder;".to_owned(),
        name: name.to_owned(),
    };

    assert_eq!(
        crate::dex_run::builtins::java_nio::byte_order_static_field(&field("LITTLE_ENDIAN")),
        Some(little_endian())
    );
    assert_eq!(
        crate::dex_run::builtins::java_nio::byte_order_static_field(&field("NATIVE_ORDER")),
        None
    );
}

#[test]
fn libcore_executors_services_are_running_until_a_shutdown_builtin_exists() {
    // cite: libcore ThreadPoolExecutor.java#isShutdown
    let program = DexProgram::new(Vec::new());
    let is_shutdown = java_io_method(
        "Ljava/util/concurrent/ExecutorService;",
        "isShutdown",
        "Z",
        &[],
    );
    let single = DexValue::Reference(std::rc::Rc::from(
        "Ljava/util/concurrent/ScheduledExecutorService;.single",
    ));

    assert_eq!(
        invoke_value(&program, &is_shutdown, vec![single]),
        DexExecutionResult::Int(0)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &is_shutdown,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("Ljava/util/concurrent/Executor;.main")
            )]))
        ),
        None
    );
}

#[test]
fn libcore_concurrent_linked_deque_default_constructor_creates_an_empty_collection() {
    // cite: libcore ConcurrentLinkedDeque.java#ConcurrentLinkedDeque
    let program = DexProgram::new(Vec::new());
    let constructor = java_io_method(
        "Ljava/util/concurrent/ConcurrentLinkedDeque;",
        "<init>",
        "V",
        &[],
    );
    let deque = program.allocate_object("Ljava/util/concurrent/ConcurrentLinkedDeque;".to_owned());

    assert_eq!(
        invoke_value(&program, &constructor, vec![deque.clone()]),
        DexExecutionResult::Void
    );
    assert!(program.value_matches_type(&deque, "Ljava/util/Deque;"));
    assert!(program.value_matches_type(&deque, "Ljava/util/AbstractCollection;"));
}

// cite: libcore Charset.java#forName, name
#[test]
fn builtin_charset_for_name_returns_canonical_charset() {
    let program = DexProgram::new(Vec::new());
    let for_name = DexMethodId {
        class_descriptor: "Ljava/nio/charset/Charset;".to_owned(),
        name: "forName".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Ljava/nio/charset/Charset;".to_owned(),
            parameters: vec!["Ljava/lang/String;".to_owned()],
        },
    };
    let name = DexMethodId {
        class_descriptor: "Ljava/nio/charset/Charset;".to_owned(),
        name: "name".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let Some(DexExecutionResult::Value(charset)) = program.invoke_builtin_method(
        &for_name,
        Some(&test_invocation(vec![DexValue::String(std::rc::Rc::from(
            "utf8",
        ))])),
    ) else {
        panic!("forName should return a charset");
    };

    assert_eq!(
        program.invoke_builtin_method(&name, Some(&test_invocation(vec![charset]))),
        Some(DexExecutionResult::Value(DexValue::String(
            std::rc::Rc::from("UTF-8")
        )))
    );
}

// cite: libcore FileOutputStream.java#FileOutputStream
// cite: libcore IoBridge.java#open
#[test]
fn libcore_file_output_stream_append_flag_keeps_or_truncates_existing_bytes() {
    let fixture = GuestIoFixture::new("output_stream_append");
    let program = fixture.program();
    let constructor = builtin_method(
        "Ljava/io/FileOutputStream;",
        "<init>",
        "V",
        &["Ljava/io/File;", "Z"],
    );
    let directory = fixture.private.join("files");
    fs::create_dir_all(&directory).expect("files directory should exist");
    let open = |name: &str, append: i32| {
        let stream = program.allocate_object("Ljava/io/FileOutputStream;".to_owned());
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                stream,
                file_value(&format!("/data/user/0/com.example/files/{name}")),
                DexValue::Int(append),
            ])),
        )
    };

    fs::write(directory.join("kept.log"), b"old").expect("seed kept.log");
    assert_eq!(open("kept.log", 1), Some(DexExecutionResult::Void));
    assert_eq!(fs::read(directory.join("kept.log")).unwrap(), b"old");

    fs::write(directory.join("cut.log"), b"old").expect("seed cut.log");
    assert_eq!(open("cut.log", 0), Some(DexExecutionResult::Void));
    assert_eq!(fs::read(directory.join("cut.log")).unwrap(), b"");

    assert_eq!(open("new.log", 1), Some(DexExecutionResult::Void));
    assert!(directory.join("new.log").is_file());
}

// cite: libcore FileOutputStream.java#write, close, closed
// cite: libcore IoBridge.java#write
// cite: libcore ArrayUtils.java#throwsIfOutOfBounds
#[test]
fn libcore_file_output_stream_writes_bytes_in_order_until_closed() {
    let fixture = GuestIoFixture::new("output_stream_write");
    let program = fixture.program();
    let directory = fixture.private.join("files");
    fs::create_dir_all(&directory).expect("files directory should exist");
    let stream = program.allocate_object("Ljava/io/FileOutputStream;".to_owned());
    let constructor = builtin_method(
        "Ljava/io/FileOutputStream;",
        "<init>",
        "V",
        &["Ljava/io/File;"],
    );
    assert_eq!(
        program.invoke_builtin_method(
            &constructor,
            Some(&test_invocation(vec![
                stream.clone(),
                file_value("/data/user/0/com.example/files/out.bin"),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    let call = |name: &str, parameters: &[&str], mut arguments: Vec<DexValue>| {
        let method = builtin_method("Ljava/io/OutputStream;", name, "V", parameters);
        arguments.insert(0, stream.clone());
        program
            .file_output_stream_intrinsic(&method, &test_invocation(arguments), 0)
            .expect("FileOutputStream call should not fail")
    };
    let bytes = |values: &[i32]| {
        program.allocate_filled_array(
            "[B".to_owned(),
            values.iter().map(|value| DexValue::Int(*value)).collect(),
        )
    };

    assert_eq!(
        call("write", &["[B"], vec![bytes(&[0x61, 0x62])]),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        call("write", &["I"], vec![DexValue::Int(0x163)]),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        call(
            "write",
            &["[B", "I", "I"],
            vec![
                bytes(&[0x78, 0x64, 0x65, 0x79]),
                DexValue::Int(1),
                DexValue::Int(2)
            ]
        ),
        Some(DexExecutionResult::Void)
    );
    let Some(DexExecutionResult::Throw(out_of_bounds)) = call(
        "write",
        &["[B", "I", "I"],
        vec![bytes(&[0x78]), DexValue::Int(1), DexValue::Int(1)],
    ) else {
        panic!("a region past the array end should throw");
    };
    assert!(
        program.value_matches_type(&out_of_bounds, "Ljava/lang/ArrayIndexOutOfBoundsException;")
    );
    assert_eq!(fs::read(directory.join("out.bin")).unwrap(), b"abcde");

    assert_eq!(
        call("close", &[], Vec::new()),
        Some(DexExecutionResult::Void)
    );
    let Some(DexExecutionResult::Throw(closed)) = call("write", &["[B"], vec![bytes(&[0x66])])
    else {
        panic!("a write after close should throw");
    };
    assert!(program.value_matches_type(&closed, "Ljava/io/IOException;"));
    assert_eq!(
        call("write", &["[B"], vec![bytes(&[])]),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(fs::read(directory.join("out.bin")).unwrap(), b"abcde");
}

#[test]
fn libcore_file_input_stream_channel_reads_at_absolute_positions_until_closed() {
    // cite: libcore FileInputStream.java#getChannel, close, channel
    // cite: libcore FileChannelImpl.java#read, readInternal, position
    let fixture = GuestIoFixture::new("file_input_stream_channel");
    let program = fixture.program();
    let path = fixture.private.join("files/lib.so");
    fs::create_dir_all(path.parent().expect("parent")).expect("files directory");
    fs::write(&path, [0x7f, b'E', b'L', b'F', 2, 1, 1, 0]).expect("elf bytes");
    let constructor = java_io_method(
        "Ljava/io/FileInputStream;",
        "<init>",
        "V",
        &["Ljava/io/File;"],
    );
    let get_channel = java_io_method(
        "Ljava/io/FileInputStream;",
        "getChannel",
        "Ljava/nio/channels/FileChannel;",
        &[],
    );
    let close = java_io_method("Ljava/io/InputStream;", "close", "V", &[]);
    let channel_position = java_io_method("Ljava/nio/channels/FileChannel;", "position", "J", &[]);
    let read_at = java_io_method(
        "Ljava/nio/channels/FileChannel;",
        "read",
        "I",
        &["Ljava/nio/ByteBuffer;", "J"],
    );
    let get_int_at = java_io_method("Ljava/nio/ByteBuffer;", "getInt", "I", &["I"]);
    let stream = program.allocate_object("Ljava/io/FileInputStream;".to_owned());
    assert_eq!(
        invoke_value(
            &program,
            &constructor,
            vec![
                stream.clone(),
                file_value("/data/user/0/com.example/files/lib.so")
            ]
        ),
        DexExecutionResult::Void
    );
    let DexExecutionResult::Value(channel) =
        invoke_value(&program, &get_channel, vec![stream.clone()])
    else {
        panic!("getChannel should return a channel");
    };
    assert_eq!(
        invoke_value(&program, &get_channel, vec![stream.clone()]),
        DexExecutionResult::Value(channel.clone())
    );

    let destination = allocate_byte_buffer(&program, 4);
    assert_eq!(
        invoke_value(
            &program,
            &read_at,
            vec![
                channel.clone(),
                destination.clone(),
                DexValue::Long(4),
                DexValue::Long(4)
            ]
        ),
        DexExecutionResult::Int(4)
    );
    assert_eq!(
        invoke_value(&program, &get_int_at, vec![destination, DexValue::Int(0)]),
        DexExecutionResult::Int(0x0201_0100)
    );
    assert_eq!(
        invoke_value(&program, &channel_position, vec![channel.clone()]),
        DexExecutionResult::Value(DexValue::Long(0))
    );
    assert_eq!(
        thrown_class(invoke_value(
            &program,
            &read_at,
            vec![
                channel.clone(),
                allocate_byte_buffer(&program, 4),
                DexValue::Long(-1),
                DexValue::Long(-1)
            ]
        )),
        "Ljava/lang/IllegalArgumentException;"
    );
    assert_eq!(
        invoke_value(
            &program,
            &read_at,
            vec![
                channel.clone(),
                allocate_byte_buffer(&program, 4),
                DexValue::Long(8),
                DexValue::Long(8)
            ]
        ),
        DexExecutionResult::Int(-1)
    );

    assert_eq!(
        invoke_value(&program, &close, vec![stream]),
        DexExecutionResult::Void
    );
    assert_eq!(
        thrown_class(invoke_value(&program, &channel_position, vec![channel])),
        "Ljava/nio/channels/ClosedChannelException;"
    );
}

#[test]
fn libcore_heap_byte_buffer_get_reads_signed_bytes_relative_and_absolute() {
    // cite: libcore HeapByteBuffer.java#get
    // cite: libcore Buffer.java#nextGetIndex, checkIndex
    let program = DexProgram::new(Vec::new());
    let buffer = allocate_byte_buffer(&program, 2);
    let array = java_io_method("Ljava/nio/ByteBuffer;", "array", "[B", &[]);
    let get = java_io_method("Ljava/nio/ByteBuffer;", "get", "B", &[]);
    let get_at = java_io_method("Ljava/nio/ByteBuffer;", "get", "B", &["I"]);
    let position = java_io_method("Ljava/nio/Buffer;", "position", "I", &[]);
    let DexExecutionResult::Value(backing) = invoke_value(&program, &array, vec![buffer.clone()])
    else {
        panic!("array should return the backing bytes");
    };
    program.write_array_value(&backing, 0, DexValue::Int(0x7f));
    program.write_array_value(&backing, 1, DexValue::Int(-2));

    assert_eq!(
        invoke_value(&program, &get_at, vec![buffer.clone(), DexValue::Int(1)]),
        DexExecutionResult::Int(-2)
    );
    assert_eq!(
        invoke_value(&program, &position, vec![buffer.clone()]),
        DexExecutionResult::Int(0)
    );
    assert_eq!(
        invoke_value(&program, &get, vec![buffer.clone()]),
        DexExecutionResult::Int(0x7f)
    );
    assert_eq!(
        invoke_value(&program, &get, vec![buffer.clone()]),
        DexExecutionResult::Int(-2)
    );
    assert_eq!(
        thrown_class(invoke_value(&program, &get, vec![buffer.clone()])),
        "Ljava/nio/BufferUnderflowException;"
    );
    assert_eq!(
        thrown_class(invoke_value(
            &program,
            &get_at,
            vec![buffer, DexValue::Int(2)]
        )),
        "Ljava/lang/IndexOutOfBoundsException;"
    );
}

#[test]
fn libcore_file_create_temp_file_creates_a_unique_random_name_in_the_directory() {
    // cite: libcore File.java#createTempFile, generateFile
    // cite: libcore Math.java#randomLongInternal
    // cite: libcore Random.java#nextLong
    let fixture = GuestIoFixture::new("create_temp_file");
    let program = fixture.program();
    program.runtime.borrow_mut().boot_wall_clock_millis = 0;
    let directory = fixture.private.join("files");
    fs::create_dir_all(&directory).expect("files directory");
    let create = java_io_method(
        "Ljava/io/File;",
        "createTempFile",
        "Ljava/io/File;",
        &["Ljava/lang/String;", "Ljava/lang/String;", "Ljava/io/File;"],
    );
    let string = |value: &str| DexValue::String(std::rc::Rc::from(value));
    let dir = file_value("/data/user/0/com.example/files");

    assert_eq!(
        invoke_value(
            &program,
            &create,
            vec![string("lib"), string(".so"), dir.clone()]
        ),
        DexExecutionResult::Value(DexValue::Reference(std::rc::Rc::from(
            "Ljava/io/File;:/data/user/0/com.example/files/lib4962768465676381896.so"
        )))
    );
    assert!(directory.join("lib4962768465676381896.so").is_file());

    let DexExecutionResult::Value(DexValue::Reference(second)) = invoke_value(
        &program,
        &create,
        vec![string("lib"), DexValue::Null, dir.clone()],
    ) else {
        panic!("createTempFile should return a File");
    };
    assert!(second.ends_with(".tmp"));
    assert!(directory.join(second.rsplit('/').next().unwrap()).is_file());

    assert_eq!(
        thrown_class(invoke_value(
            &program,
            &create,
            vec![string("ab"), string(".so"), dir]
        )),
        "Ljava/lang/IllegalArgumentException;"
    );
}

#[test]
fn libcore_file_set_read_only_clears_every_write_bit() {
    // cite: libcore File.java#setReadOnly
    // cite: libcore UnixFileSystem_md.c#Java_java_io_UnixFileSystem_setReadOnly0
    use std::os::unix::fs::PermissionsExt;
    let fixture = GuestIoFixture::new("set_read_only");
    let program = fixture.program();
    let directory = fixture.private.join("files");
    fs::create_dir_all(&directory).expect("files directory");
    let library = directory.join("lib.so");
    fs::write(&library, b"elf").expect("library bytes");
    fs::set_permissions(&library, fs::Permissions::from_mode(0o764)).expect("initial mode");
    let set_read_only = java_io_method("Ljava/io/File;", "setReadOnly", "Z", &[]);

    assert_eq!(
        invoke_value(
            &program,
            &set_read_only,
            vec![file_value("/data/user/0/com.example/files/lib.so")]
        ),
        DexExecutionResult::Int(1)
    );
    assert_eq!(
        fs::metadata(&library).unwrap().permissions().mode() & 0o777,
        0o544
    );
    assert_eq!(
        invoke_value(
            &program,
            &set_read_only,
            vec![file_value("/data/user/0/com.example/files/missing.so")]
        ),
        DexExecutionResult::Int(0)
    );
}

#[test]
fn libcore_file_list_returns_child_names_or_null_for_a_non_directory() {
    // cite: libcore File.java#list
    let fixture = GuestIoFixture::new("file_list");
    let program = fixture.program();
    let directory = fixture.private.join("files/libs");
    fs::create_dir_all(&directory).expect("libs directory");
    fs::write(directory.join("a.so"), b"a").expect("a.so");
    fs::write(directory.join("b.so"), b"b").expect("b.so");
    let list = java_io_method("Ljava/io/File;", "list", "[Ljava/lang/String;", &[]);

    let DexExecutionResult::Value(names) = invoke_value(
        &program,
        &list,
        vec![file_value("/data/user/0/com.example/files/libs")],
    ) else {
        panic!("list should return an array");
    };
    let mut entries: Vec<String> = program.runtime.borrow().array_entries
        [&map_identity_key(&names)]
        .iter()
        .map(|name| match name {
            DexValue::String(name) => name.to_string(),
            other => panic!("list entry should be a String, got {other}"),
        })
        .collect();
    entries.sort();
    assert_eq!(entries, ["a.so", "b.so"]);
    assert_eq!(
        invoke_value(
            &program,
            &list,
            vec![file_value("/data/user/0/com.example/files/libs/a.so")]
        ),
        DexExecutionResult::Value(DexValue::Null)
    );
}

#[test]
fn libcore_file_get_canonical_path_returns_the_collapsed_guest_path_string() {
    // cite: libcore File.java#getCanonicalPath
    // cite: libcore canonicalize_md.c#canonicalize, collapse
    let fixture = GuestIoFixture::new("canonical_path");
    fs::create_dir_all(fixture.private.join("cache")).expect("cache directory");
    let program = fixture.program();
    let get_canonical_path = java_io_method(
        "Ljava/io/File;",
        "getCanonicalPath",
        "Ljava/lang/String;",
        &[],
    );

    assert_eq!(
        invoke_value(
            &program,
            &get_canonical_path,
            vec![file_value("/data/user/0/com.example/files/../cache/./x.db")]
        ),
        DexExecutionResult::Value(DexValue::String(std::rc::Rc::from(
            "/data/user/0/com.example/cache/x.db"
        )))
    );
}

#[test]
fn libcore_file_constructors_accept_string_objects_built_at_runtime() {
    // cite: libcore File.java#File
    // cite: libcore String.java#String
    let program = DexProgram::new(Vec::new());
    let string_from_bytes = java_io_method("Ljava/lang/String;", "<init>", "V", &["[B"]);
    let runtime_string = |text: &str| {
        let value = program.allocate_object("Ljava/lang/String;".to_owned());
        let bytes = program.allocate_filled_array(
            "[B".to_owned(),
            text.bytes()
                .map(|byte| DexValue::Int(i32::from(byte as i8)))
                .collect(),
        );
        invoke_value(&program, &string_from_bytes, vec![value.clone(), bytes]);
        value
    };
    let path_field = DexFieldId {
        class_descriptor: "Ljava/io/File;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: "path".to_owned(),
    };
    let from_path = java_io_method("Ljava/io/File;", "<init>", "V", &["Ljava/lang/String;"]);
    let from_parent = java_io_method(
        "Ljava/io/File;",
        "<init>",
        "V",
        &["Ljava/lang/String;", "Ljava/lang/String;"],
    );
    let file = program.allocate_object("Lapp/GuestFile;".to_owned());
    let child = program.allocate_object("Ljava/io/File;".to_owned());

    assert_eq!(
        invoke_value(
            &program,
            &from_path,
            vec![file.clone(), runtime_string("/data/local//tmp/")]
        ),
        DexExecutionResult::Void
    );
    assert_eq!(
        program.read_stored_instance_field(&file, &path_field),
        Some(DexValue::String(std::rc::Rc::from("/data/local/tmp")))
    );
    assert_eq!(
        invoke_value(
            &program,
            &from_parent,
            vec![
                child.clone(),
                runtime_string("/data"),
                runtime_string("lib.so")
            ]
        ),
        DexExecutionResult::Void
    );
    assert_eq!(
        program.read_stored_instance_field(&child, &path_field),
        Some(DexValue::String(std::rc::Rc::from("/data/lib.so")))
    );
}

fn print_writer_contract_result(method: &str) -> DexExecutionResult {
    let dex_file = crate::dex::parse_dex_file(
        "print_writer_contract.dex",
        include_bytes!("fixtures/print_writer_contract.dex").to_vec(),
    )
    .expect("PrintWriter contract DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let method = DexMethodId {
        class_descriptor: "Lwinedroid/tests/PrintWriterContract;".to_owned(),
        name: method.to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Ljava/lang/String;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let resolved = program
        .resolve_exact_method(&method)
        .expect("PrintWriter contract method should resolve")
        .expect("PrintWriter contract method should exist");

    program
        .execute_method(None, &resolved, None, 0)
        .expect("PrintWriter contract method should execute")
}

#[test]
fn libcore_print_writer_println_writes_the_separator_string_and_closes_once() {
    // cite: libcore PrintWriter.java#println, newLine, close, ensureOpen
    assert_eq!(
        print_writer_contract_result("printlnUsesWriteStringThenCloseOnce"),
        DexExecutionResult::Value(DexValue::String("s[\\n]:1".into()))
    );
}

#[test]
fn libcore_print_writer_println_reaches_a_writer_that_only_writes_char_arrays() {
    // cite: libcore Writer.java#write
    assert_eq!(
        print_writer_contract_result("printlnFallsBackToCharArrayWrite"),
        DexExecutionResult::Value(DexValue::String("\\n".into()))
    );
}

#[test]
fn libcore_file_lock_table_rejects_overlaps_and_close_invalidates_locks() {
    // cite: libcore FileLockTable.java#add, checkList, removeAll
    // cite: libcore FileChannelImpl.java#lock, implCloseChannel
    // cite: libcore FileLockImpl.java#isValid, release
    let fixture = GuestIoFixture::new("file_lock_table");
    let program = fixture.program();
    fs::create_dir_all(fixture.private.join("files")).expect("files directory");
    let constructor = java_io_method(
        "Ljava/io/RandomAccessFile;",
        "<init>",
        "V",
        &["Ljava/io/File;", "Ljava/lang/String;"],
    );
    let get_channel = java_io_method(
        "Ljava/io/RandomAccessFile;",
        "getChannel",
        "Ljava/nio/channels/FileChannel;",
        &[],
    );
    let close = java_io_method("Ljava/io/RandomAccessFile;", "close", "V", &[]);
    let lock = java_io_method(
        "Ljava/nio/channels/FileChannel;",
        "lock",
        "Ljava/nio/channels/FileLock;",
        &[],
    );
    let release = java_io_method("Ljava/nio/channels/FileLock;", "release", "V", &[]);
    let is_valid = java_io_method("Ljava/nio/channels/FileLock;", "isValid", "Z", &[]);
    let open = || {
        let file = program.allocate_object("Ljava/io/RandomAccessFile;".to_owned());
        invoke_value(
            &program,
            &constructor,
            vec![
                file.clone(),
                file_value("/data/user/0/com.example/files/process.lock"),
                DexValue::String(std::rc::Rc::from("rw")),
            ],
        );
        let DexExecutionResult::Value(channel) =
            invoke_value(&program, &get_channel, vec![file.clone()])
        else {
            panic!("getChannel should return a channel");
        };
        (file, channel)
    };
    let lock_of = |channel: &DexValue| invoke_value(&program, &lock, vec![channel.clone()]);

    let (first_file, first) = open();
    let (second_file, second) = open();
    let DexExecutionResult::Value(held) = lock_of(&first) else {
        panic!("the first lock should be granted");
    };
    assert_eq!(
        thrown_class(lock_of(&second)),
        "Ljava/nio/channels/OverlappingFileLockException;"
    );
    assert_eq!(
        thrown_class(lock_of(&first)),
        "Ljava/nio/channels/OverlappingFileLockException;"
    );

    invoke_value(&program, &release, vec![held.clone()]);
    let DexExecutionResult::Value(reacquired) = lock_of(&second) else {
        panic!("a released lock should free the file");
    };
    let (_, third) = open();
    assert_eq!(
        thrown_class(lock_of(&third)),
        "Ljava/nio/channels/OverlappingFileLockException;"
    );

    let _ = first_file;
    invoke_value(&program, &close, vec![second_file]);
    assert_eq!(
        invoke_value(&program, &is_valid, vec![reacquired]),
        DexExecutionResult::Int(0)
    );
    let DexExecutionResult::Value(_) = lock_of(&third) else {
        panic!("closing the owning channel should free the file");
    };
}

#[test]
fn libcore_file_output_stream_writes_through_the_descriptor_it_opened() {
    // cite: libcore FileOutputStream.java#FileOutputStream, write
    // cite: libcore IoBridge.java#open, write
    let fixture = GuestIoFixture::new("output_stream_descriptor");
    let program = fixture.program();
    let directory = fixture.private.join("files");
    fs::create_dir_all(&directory).expect("files directory");
    let stream = program.allocate_object("Ljava/io/FileOutputStream;".to_owned());
    let constructor = builtin_method(
        "Ljava/io/FileOutputStream;",
        "<init>",
        "V",
        &["Ljava/io/File;"],
    );
    program.invoke_builtin_method(
        &constructor,
        Some(&test_invocation(vec![
            stream.clone(),
            file_value("/data/user/0/com.example/files/log.txt"),
        ])),
    );
    let write = builtin_method("Ljava/io/OutputStream;", "write", "V", &["I"]);
    let write_byte = |byte: u8| {
        program
            .file_output_stream_intrinsic(
                &write,
                &test_invocation(vec![stream.clone(), DexValue::Int(i32::from(byte))]),
                0,
            )
            .unwrap()
    };

    assert_eq!(write_byte(b'a'), Some(DexExecutionResult::Void));
    fs::rename(directory.join("log.txt"), directory.join("old.txt")).expect("rename log");
    assert_eq!(write_byte(b'b'), Some(DexExecutionResult::Void));

    assert_eq!(fs::read(directory.join("old.txt")).unwrap(), b"ab");
    assert!(!directory.join("log.txt").exists());
}

#[test]
fn libcore_file_input_stream_and_its_channel_share_one_file_position() {
    // cite: libcore FileInputStream.java#getChannel, read, close
    // cite: libcore FileChannelImpl.java#position, read
    let fixture = GuestIoFixture::new("file_input_stream_shared_position");
    let program = fixture.program();
    let path = fixture.private.join("files/data.bin");
    fs::create_dir_all(path.parent().expect("parent")).expect("files directory");
    fs::write(&path, [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10]).expect("data bytes");
    let guest = "/data/user/0/com.example/files/data.bin";
    let constructor = java_io_method(
        "Ljava/io/FileInputStream;",
        "<init>",
        "V",
        &["Ljava/io/File;"],
    );
    let get_channel = java_io_method(
        "Ljava/io/FileInputStream;",
        "getChannel",
        "Ljava/nio/channels/FileChannel;",
        &[],
    );
    let read_range = java_io_method("Ljava/io/InputStream;", "read", "I", &["[B", "I", "I"]);
    let read_byte = java_io_method("Ljava/io/InputStream;", "read", "I", &[]);
    let close = java_io_method("Ljava/io/InputStream;", "close", "V", &[]);
    let position = java_io_method("Ljava/nio/channels/FileChannel;", "position", "J", &[]);
    let channel_read = java_io_method(
        "Ljava/nio/channels/FileChannel;",
        "read",
        "I",
        &["Ljava/nio/ByteBuffer;"],
    );
    let get_at = java_io_method("Ljava/nio/ByteBuffer;", "get", "B", &["I"]);
    let open = || {
        let stream = program.allocate_object("Ljava/io/FileInputStream;".to_owned());
        invoke_value(
            &program,
            &constructor,
            vec![stream.clone(), file_value(guest)],
        );
        stream
    };

    let stream = open();
    let header = program.allocate_filled_array("[B".to_owned(), vec![DexValue::Int(0); 4]);
    assert_eq!(
        invoke_value(
            &program,
            &read_range,
            vec![stream.clone(), header, DexValue::Int(0), DexValue::Int(4)]
        ),
        DexExecutionResult::Int(4)
    );
    let DexExecutionResult::Value(channel) =
        invoke_value(&program, &get_channel, vec![stream.clone()])
    else {
        panic!("getChannel should return a channel");
    };
    assert_eq!(
        invoke_value(&program, &position, vec![channel.clone()]),
        DexExecutionResult::Value(DexValue::Long(4))
    );
    let buffer = allocate_byte_buffer(&program, 2);
    assert_eq!(
        invoke_value(
            &program,
            &channel_read,
            vec![channel.clone(), buffer.clone()]
        ),
        DexExecutionResult::Int(2)
    );
    assert_eq!(
        invoke_value(&program, &get_at, vec![buffer, DexValue::Int(0)]),
        DexExecutionResult::Int(5)
    );
    assert_eq!(
        invoke_value(&program, &read_byte, vec![stream.clone()]),
        DexExecutionResult::Int(7)
    );

    let closed = open();
    invoke_value(&program, &close, vec![closed.clone()]);
    let DexExecutionResult::Value(late_channel) =
        invoke_value(&program, &get_channel, vec![closed])
    else {
        panic!("getChannel after close should still return a channel");
    };
    assert_eq!(
        thrown_class(invoke_value(&program, &position, vec![late_channel])),
        "Ljava/nio/channels/ClosedChannelException;"
    );
}

#[test]
fn libcore_file_set_read_only_never_changes_a_symlink_target() {
    // cite: libcore UnixFileSystem_md.c#Java_java_io_UnixFileSystem_setReadOnly0
    use std::os::unix::fs::PermissionsExt;
    let fixture = GuestIoFixture::new("set_read_only_symlink");
    let program = fixture.program();
    let directory = fixture.private.join("files");
    fs::create_dir_all(&directory).expect("files directory");
    let outside = fixture.shared.join("outside.txt");
    fs::create_dir_all(outside.parent().unwrap()).expect("outside parent");
    fs::write(&outside, b"host").expect("outside file");
    fs::set_permissions(&outside, fs::Permissions::from_mode(0o644)).expect("outside mode");
    std::os::unix::fs::symlink(&outside, directory.join("link")).expect("symlink");
    let set_read_only = java_io_method("Ljava/io/File;", "setReadOnly", "Z", &[]);

    assert_eq!(
        invoke_value(
            &program,
            &set_read_only,
            vec![file_value("/data/user/0/com.example/files/link")]
        ),
        DexExecutionResult::Int(0)
    );
    assert_eq!(
        fs::metadata(&outside).unwrap().permissions().mode() & 0o777,
        0o644
    );
}

#[test]
fn libcore_file_get_canonical_path_collapses_paths_outside_the_app_mounts() {
    // cite: libcore File.java#getCanonicalPath
    // cite: libcore canonicalize_md.c#canonicalize, collapse
    let fixture = GuestIoFixture::new("canonical_path_outside");
    let program = fixture.program();
    let get_canonical_path = java_io_method(
        "Ljava/io/File;",
        "getCanonicalPath",
        "Ljava/lang/String;",
        &[],
    );

    assert_eq!(
        invoke_value(
            &program,
            &get_canonical_path,
            vec![file_value("/system/lib64/../lib64/./libc.so")]
        ),
        DexExecutionResult::Value(DexValue::String(std::rc::Rc::from("/system/lib64/libc.so")))
    );
}

#[test]
fn libcore_byte_array_input_stream_reads_its_live_buffer_within_bounds() {
    // cite: libcore ByteArrayInputStream.java#ByteArrayInputStream, read, available
    let program = DexProgram::new(Vec::new());
    let full = java_io_method("Ljava/io/ByteArrayInputStream;", "<init>", "V", &["[B"]);
    let ranged = java_io_method(
        "Ljava/io/ByteArrayInputStream;",
        "<init>",
        "V",
        &["[B", "I", "I"],
    );
    let read = java_io_method("Ljava/io/InputStream;", "read", "I", &[]);
    let available = java_io_method("Ljava/io/InputStream;", "available", "I", &[]);
    let buffer = program.allocate_filled_array(
        "[B".to_owned(),
        [10, 20, 30, -1]
            .iter()
            .map(|byte| DexValue::Int(*byte))
            .collect(),
    );
    let read_all = |stream: &DexValue| {
        let mut values = Vec::new();
        loop {
            match invoke_value(&program, &read, vec![stream.clone()]) {
                DexExecutionResult::Int(-1) => break values,
                DexExecutionResult::Int(value) => values.push(value),
                other => panic!("read returned {other}"),
            }
        }
    };

    let stream = program.allocate_object("Ljava/io/ByteArrayInputStream;".to_owned());
    invoke_value(&program, &full, vec![stream.clone(), buffer.clone()]);
    program.write_array_value(&buffer, 1, DexValue::Int(21));
    assert_eq!(
        invoke_value(&program, &available, vec![stream.clone()]),
        DexExecutionResult::Int(4)
    );
    assert_eq!(read_all(&stream), [10, 21, 30, 255]);

    let window = program.allocate_object("Ljava/io/ByteArrayInputStream;".to_owned());
    invoke_value(
        &program,
        &ranged,
        vec![window.clone(), buffer, DexValue::Int(1), DexValue::Int(9)],
    );
    assert_eq!(read_all(&window), [21, 30, 255]);
}

#[test]
fn libcore_byte_array_input_stream_close_is_a_no_op_and_empty_reads_report_eof() {
    // cite: libcore ByteArrayInputStream.java#close, read
    let program = DexProgram::new(Vec::new());
    let init = java_io_method("Ljava/io/ByteArrayInputStream;", "<init>", "V", &["[B"]);
    let read = java_io_method("Ljava/io/InputStream;", "read", "I", &[]);
    let read_into = java_io_method("Ljava/io/InputStream;", "read", "I", &["[B", "I", "I"]);
    let close = java_io_method("Ljava/io/InputStream;", "close", "V", &[]);
    let buffer = program.allocate_filled_array("[B".to_owned(), vec![DexValue::Int(7)]);
    let target = program.allocate_filled_array("[B".to_owned(), vec![DexValue::Int(0)]);
    let stream = program.allocate_object("Ljava/io/ByteArrayInputStream;".to_owned());
    invoke_value(&program, &init, vec![stream.clone(), buffer]);
    let empty_read = || {
        invoke_value(
            &program,
            &read_into,
            vec![
                stream.clone(),
                target.clone(),
                DexValue::Int(0),
                DexValue::Int(0),
            ],
        )
    };

    assert_eq!(empty_read(), DexExecutionResult::Int(0));
    invoke_value(&program, &close, vec![stream.clone()]);
    assert_eq!(
        invoke_value(&program, &read, vec![stream.clone()]),
        DexExecutionResult::Int(7),
        "libcore ByteArrayInputStream.close has an empty body"
    );
    assert_eq!(
        empty_read(),
        DexExecutionResult::Int(-1),
        "libcore read(b, off, len) returns -1 once pos >= count, before checking len"
    );
}

fn serial_contract_program() -> DexProgram {
    let dex_file = crate::dex::parse_dex_file(
        "serial_contract.dex",
        include_bytes!("fixtures/serial_contract.dex").to_vec(),
    )
    .expect("serialization contract DEX should parse");
    DexProgram::new(vec![dex_file])
}

fn serial_contract_call(
    program: &DexProgram,
    name: &str,
    return_type: &str,
    parameters: &[&str],
    arguments: &[DexValue],
) -> DexExecutionResult {
    let method = DexMethodId {
        class_descriptor: "Lwinedroid/tests/SerialContract;".to_owned(),
        name: name.to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".repeat(1 + parameters.len()),
            return_type: return_type.to_owned(),
            parameters: parameters
                .iter()
                .map(|parameter| (*parameter).to_owned())
                .collect(),
        },
    };
    let resolved = program.resolve_exact_method(&method).unwrap().unwrap();
    program
        .execute_method(None, &resolved, Some(arguments), 0)
        .unwrap()
}

#[test]
fn libcore_object_output_stream_matches_the_openjdk_wire_bytes() {
    // cite: libcore ObjectOutputStream.java#writeStreamHeader, writeObject0, writeOrdinaryObject, writeNonProxyDesc, writeString, defaultWriteFields
    // cite: libcore ObjectStreamClass.java#writeNonProxy, getDefaultSerialFields
    let program = serial_contract_program();
    let DexExecutionResult::Value(bytes) =
        serial_contract_call(&program, "writeGraph", "[B", &[], &[])
    else {
        panic!("writeGraph should return a byte[]");
    };
    let written: Vec<u8> = program.runtime.borrow().array_entries[&map_identity_key(&bytes)]
        .iter()
        .map(|value| match value {
            DexValue::Int(byte) => *byte as u8,
            other => panic!("byte entry {other}"),
        })
        .collect();

    assert_eq!(
        written,
        include_bytes!("fixtures/serial_contract.openjdk.ser").to_vec()
    );
}

#[test]
fn libcore_object_input_stream_reads_the_openjdk_wire_bytes() {
    // cite: libcore ObjectInputStream.java#readStreamHeader, readObject0, readOrdinaryObject, readNonProxyDesc, defaultReadFields
    // cite: libcore ObjectStreamClass.java#newInstance, getSerializableConstructor
    let program = serial_contract_program();
    let oracle = include_bytes!("fixtures/serial_contract.openjdk.ser");
    let data = program.allocate_filled_array(
        "[B".to_owned(),
        oracle
            .iter()
            .map(|byte| DexValue::Int(i32::from(*byte as i8)))
            .collect(),
    );

    assert_eq!(
        serial_contract_call(
            &program,
            "readGraph",
            "Ljava/lang/String;",
            &["[B"],
            &[data]
        ),
        DexExecutionResult::Value(DexValue::String(
            "41,true,Z,0.5,-3,0,shared,shared,1234567890123,caf\u{e9},true,null,null".into()
        ))
    );
}

#[test]
fn file_observer_missing_path_fails_the_descriptor_and_unwired_watches_are_missing() {
    // cite: frameworks-base FileObserver.java#startWatching, stopWatching
    // cite: frameworks-base android_util_FileObserver.cpp#android_os_fileobserver_startWatching
    let fixture = GuestIoFixture::new("file_observer");
    let program = fixture.program();
    fs::create_dir_all(fixture.private.join("files/present")).expect("present directory");
    fs::write(fixture.private.join("files/plain"), b"x").expect("plain file");
    let constructor = java_io_method(
        "Landroid/os/FileObserver;",
        "<init>",
        "V",
        &["Ljava/lang/String;", "I"],
    );
    let start = java_io_method("Landroid/os/FileObserver;", "startWatching", "V", &[]);
    let stop = java_io_method("Landroid/os/FileObserver;", "stopWatching", "V", &[]);
    let observer_with_mask = |path: &str, mask: i32| {
        let observer = program.allocate_object("Landroid/os/FileObserver;".to_owned());
        invoke_value(
            &program,
            &constructor,
            vec![
                observer.clone(),
                DexValue::String(std::rc::Rc::from(path)),
                DexValue::Int(mask),
            ],
        );
        observer
    };
    let observer_for = |path: &str| observer_with_mask(path, 0x108);
    let descriptors_field = DexFieldId {
        class_descriptor: "Landroid/os/FileObserver;".to_owned(),
        type_descriptor: "[I".to_owned(),
        name: "mDescriptors".to_owned(),
    };

    let missing = observer_for("/data/user/0/com.example/files/npth/anr/anr_info");
    assert_eq!(
        invoke_value(&program, &start, vec![missing.clone()]),
        DexExecutionResult::Void
    );
    let descriptors = program
        .read_stored_instance_field(&missing, &descriptors_field)
        .expect("startWatching should record descriptors");
    assert_eq!(
        program.runtime.borrow().array_entries[&map_identity_key(&descriptors)],
        [DexValue::Int(-1)]
    );
    assert_eq!(
        invoke_value(&program, &stop, vec![missing.clone()]),
        DexExecutionResult::Void
    );
    assert_eq!(
        program.read_stored_instance_field(&missing, &descriptors_field),
        Some(DexValue::Null)
    );

    let present = observer_for("/data/user/0/com.example/files/present");
    assert_eq!(
        invoke_value(&program, &start, vec![present.clone()]),
        DexExecutionResult::Void
    );
    let descriptors = program
        .read_stored_instance_field(&present, &descriptors_field)
        .expect("startWatching should record descriptors");
    assert_eq!(
        program.runtime.borrow().array_entries[&map_identity_key(&descriptors)],
        [DexValue::Int(1)],
        "inotify_add_watch(2) returns the first watch descriptor of the ObserverThread m_fd"
    );

    for (path, mask) in [
        ("/data/user/0/com.example/files/present", 0x102),
        ("/data/user/0/com.example/files/plain", 0x800),
        ("/proc/self", 0x108),
    ] {
        let observer = observer_with_mask(path, mask);
        assert_eq!(
            program.invoke_builtin_method(&start, Some(&test_invocation(vec![observer]))),
            None,
            "{path} with mask {mask:#x} needs an unwired inotify producer and must stay visible"
        );
    }
}

#[test]
fn libcore_object_input_stream_defers_class_not_found_and_skips_custom_data() {
    // cite: libcore ObjectInputStream.java#readObject, lookupException, readOrdinaryObject, skipCustomData
    let program = serial_contract_program();
    let oracle = include_bytes!("fixtures/serial_mixed.openjdk.ser");
    let data = program.allocate_filled_array(
        "[B".to_owned(),
        oracle
            .iter()
            .map(|byte| DexValue::Int(i32::from(*byte as i8)))
            .collect(),
    );

    assert_eq!(
        serial_contract_call(
            &program,
            "readMixed",
            "Ljava/lang/String;",
            &["[B"],
            &[data]
        ),
        DexExecutionResult::Value(DexValue::String(
            "cnfe:winedroid.tests.SerialHost$Ghost;after;77;tail".into()
        ))
    );
}

#[test]
fn libcore_object_output_stream_reports_framework_fields_as_missing_support() {
    // cite: libcore ObjectOutputStream.java#writeObject0
    let program = serial_contract_program();
    let method = DexMethodId {
        class_descriptor: "Lwinedroid/tests/SerialContract;".to_owned(),
        name: "writeWithMap".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let resolved = program.resolve_exact_method(&method).unwrap().unwrap();
    let error = program
        .execute_method(None, &resolved, None, 0)
        .expect_err("a HashMap field is not serializable yet");
    assert!(
        error
            .to_string()
            .contains("serialization of framework class java.util.HashMap"),
        "{error}"
    );
}

#[test]
fn libcore_object_input_stream_huge_string_length_ends_in_eof_without_allocating_it() {
    // cite: libcore ObjectInputStream.java#readLongUTF, readFully
    let program = serial_contract_program();
    let data = program.allocate_filled_array(
        "[B".to_owned(),
        [
            0xac, 0xed, 0x00, 0x05, 0x7c, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        ]
        .iter()
        .map(|byte: &u8| DexValue::Int(i32::from(*byte as i8)))
        .collect(),
    );

    let DexExecutionResult::Throw(exception) = serial_contract_call(
        &program,
        "readGraph",
        "Ljava/lang/String;",
        &["[B"],
        &[data],
    ) else {
        panic!("a truncated long string should throw");
    };
    assert!(program.value_matches_type(&exception, "Ljava/io/EOFException;"));
}
