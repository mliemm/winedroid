use super::super::*;
use super::helpers::*;

#[test]
fn builtin_view_group_remove_all_views_returns_void() {
    let program = DexProgram::new(Vec::new());
    let remove_all_views = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "removeAllViews".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let add_view = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "addView".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/View;".to_owned()],
        },
    };
    let request_layout = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "requestLayout".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let set_layout_transition = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "setLayoutTransition".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/animation/LayoutTransition;".to_owned()],
        },
    };
    let get_child_count = DexMethodId {
        class_descriptor: "Landroid/view/ViewGroup;".to_owned(),
        name: "getChildCount".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&remove_all_views, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&add_view, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&request_layout, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&set_layout_transition, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&get_child_count, None),
        Some(DexExecutionResult::Int(0))
    );
}

#[test]
fn builtin_layout_transition_constructor_returns_void() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/animation/LayoutTransition;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let disable_transition_type = DexMethodId {
        class_descriptor: "Landroid/animation/LayoutTransition;".to_owned(),
        name: "disableTransitionType".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VI".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["I".to_owned()],
        },
    };
    let animator_listener = DexMethodId {
        class_descriptor: "Landroid/animation/AnimatorListenerAdapter;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from(
            "Landroid/animation/LayoutTransition;:main",
        )),
        DexValue::Int(4),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&constructor, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&disable_transition_type, Some(&invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&animator_listener, None),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_over_scroller_abort_animation_returns_void() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/widget/OverScroller;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/content/Context;".to_owned()],
        },
    };
    let interpolator_constructor = DexMethodId {
        class_descriptor: "Landroid/widget/OverScroller;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/content/Context;".to_owned(),
                "Landroid/view/animation/Interpolator;".to_owned(),
            ],
        },
    };
    let method = DexMethodId {
        class_descriptor: "Landroid/widget/OverScroller;".to_owned(),
        name: "abortAnimation".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "V".to_owned(),
            return_type: "V".to_owned(),
            parameters: Vec::new(),
        },
    };
    let constructor_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Landroid/widget/OverScroller;:main")),
        DexValue::Reference(std::rc::Rc::from("Landroid/content/Context;.bootstrap")),
    ]);
    let interpolator_constructor_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Landroid/widget/OverScroller;:main")),
        DexValue::Reference(std::rc::Rc::from("Landroid/content/Context;.bootstrap")),
        DexValue::Null,
    ]);

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&constructor_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &interpolator_constructor,
            Some(&interpolator_constructor_invocation)
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&method, None),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_edge_effect_constructors_return_void() {
    let program = DexProgram::new(Vec::new());
    let context_constructor = DexMethodId {
        class_descriptor: "Landroid/widget/EdgeEffect;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/content/Context;".to_owned()],
        },
    };
    let attrs_constructor = DexMethodId {
        class_descriptor: "Landroid/widget/EdgeEffect;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VLL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec![
                "Landroid/content/Context;".to_owned(),
                "Landroid/util/AttributeSet;".to_owned(),
            ],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(
            &context_constructor,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("Landroid/widget/EdgeEffect;:main")),
                DexValue::Reference(std::rc::Rc::from("Landroid/content/Context;.bootstrap")),
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(
            &attrs_constructor,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("Landroid/widget/EdgeEffect;:main")),
                DexValue::Reference(std::rc::Rc::from("Landroid/content/Context;.bootstrap")),
                DexValue::Null,
            ]))
        ),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_layout_inflater_from_returns_bootstrap_reference() {
    let program = DexProgram::new(Vec::new());
    let from = DexMethodId {
        class_descriptor: "Landroid/view/LayoutInflater;".to_owned(),
        name: "from".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Landroid/view/LayoutInflater;".to_owned(),
            parameters: vec!["Landroid/content/Context;".to_owned()],
        },
    };
    let inflate = DexMethodId {
        class_descriptor: "Landroid/view/LayoutInflater;".to_owned(),
        name: "inflate".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LILZ".to_owned(),
            return_type: "Landroid/view/View;".to_owned(),
            parameters: vec![
                "I".to_owned(),
                "Landroid/view/ViewGroup;".to_owned(),
                "Z".to_owned(),
            ],
        },
    };
    let inflate_without_attach = DexMethodId {
        class_descriptor: "Landroid/view/LayoutInflater;".to_owned(),
        name: "inflate".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LIL".to_owned(),
            return_type: "Landroid/view/View;".to_owned(),
            parameters: vec!["I".to_owned(), "Landroid/view/ViewGroup;".to_owned()],
        },
    };
    let get_factory = DexMethodId {
        class_descriptor: "Landroid/view/LayoutInflater;".to_owned(),
        name: "getFactory".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "L".to_owned(),
            return_type: "Landroid/view/LayoutInflater$Factory;".to_owned(),
            parameters: Vec::new(),
        },
    };
    let clone_in_context = DexMethodId {
        class_descriptor: "Landroid/view/LayoutInflater;".to_owned(),
        name: "cloneInContext".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "LL".to_owned(),
            return_type: "Landroid/view/LayoutInflater;".to_owned(),
            parameters: vec!["Landroid/content/Context;".to_owned()],
        },
    };
    let set_factory2 = DexMethodId {
        class_descriptor: "Landroid/view/LayoutInflater;".to_owned(),
        name: "setFactory2".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/LayoutInflater$Factory2;".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Landroid/view/LayoutInflater;.bootstrap")),
        DexValue::Int(2131624225),
        DexValue::Null,
        DexValue::Int(0),
    ]);
    let no_attach_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Landroid/view/LayoutInflater;.bootstrap")),
        DexValue::Int(2131624226),
        DexValue::Reference(std::rc::Rc::from("Landroid/view/ViewGroup;.parent")),
    ]);
    let content_parent = view_reference(ANDROID_CONTENT_VIEW_ID);
    let attached_content_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Landroid/view/LayoutInflater;.bootstrap")),
        DexValue::Int(2131624227),
        content_parent.clone(),
    ]);
    let factory_invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Landroid/view/LayoutInflater;.bootstrap")),
        DexValue::Reference(std::rc::Rc::from(
            "Landroid/view/LayoutInflater$Factory2;.bootstrap",
        )),
    ]);

    let context = DexValue::Reference(std::rc::Rc::from("Landroid/content/Context;.bootstrap"));
    let Some(DexExecutionResult::Value(from_inflater)) =
        program.invoke_builtin_method(&from, Some(&test_invocation(vec![context.clone()])))
    else {
        panic!("LayoutInflater.from should return an inflater");
    };
    assert!(program.value_matches_type(&from_inflater, "Landroid/view/LayoutInflater;"));
    assert_eq!(
        program.invoke_builtin_method(&from, Some(&test_invocation(vec![context]))),
        Some(DexExecutionResult::Value(from_inflater))
    );
    assert_eq!(
        program.invoke_builtin_method(&get_factory, None),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        program.invoke_builtin_method(
            &clone_in_context,
            Some(&test_invocation(vec![
                DexValue::Reference(std::rc::Rc::from("Landroid/view/LayoutInflater;.bootstrap")),
                DexValue::Reference(std::rc::Rc::from("Landroid/content/Context;.bootstrap")),
            ])),
        ),
        program.invoke_builtin_method(
            &from,
            Some(&test_invocation(vec![DexValue::Reference(
                std::rc::Rc::from("Landroid/content/Context;.bootstrap")
            )]))
        )
    );
    assert_eq!(
        program.invoke_builtin_method(&set_factory2, Some(&factory_invocation)),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&inflate, Some(&invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/view/View;:layout/2131624225")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&inflate_without_attach, Some(&no_attach_invocation)),
        Some(DexExecutionResult::Value(DexValue::Reference(
            std::rc::Rc::from("Landroid/view/View;:layout/2131624226")
        )))
    );
    assert_eq!(
        program.invoke_builtin_method(&inflate_without_attach, Some(&attached_content_invocation)),
        Some(DexExecutionResult::Value(content_parent))
    );
    // cite: frameworks-base ViewGroup.java#addView
    assert_eq!(program.view_snapshot().inflated_layouts, vec![2131624227]);
    assert_eq!(program.view_snapshot().content_layouts, vec![2131624227]);
}

#[test]
fn layout_element_class_descriptor_maps_android_and_custom_names() {
    assert_eq!(
        layout_element_class_descriptor("FrameLayout"),
        Some("Landroid/widget/FrameLayout;".to_owned())
    );
    assert_eq!(
        layout_element_class_descriptor("androidx.appcompat.view.menu.ListMenuItemView"),
        Some("Landroidx/appcompat/view/menu/ListMenuItemView;".to_owned())
    );
}

// cite: frameworks-base LayoutInflater.java#mConstructorSignature, createView
#[test]
fn frameworks_layout_inflater_uses_context_attribute_set_constructor() {
    let view = DexValue::Reference(std::rc::Rc::from("Lexample/View;:layout/42"));
    let context = DexValue::Reference(std::rc::Rc::from("Landroid/content/Context;.bootstrap"));
    let attrs = DexValue::Reference(std::rc::Rc::from(
        "Landroid/content/res/XmlResourceParser;:layout/42",
    ));
    let candidates =
        inflated_view_constructor_candidates("Lexample/View;", &view, &context, &attrs);

    assert_eq!(candidates.len(), 1);
    assert_eq!(
        candidates[0].0.proto.descriptor(),
        "(Landroid/content/Context;Landroid/util/AttributeSet;)V"
    );
    assert_eq!(candidates[0].1, vec![view, context, attrs,]);
}

// cite: frameworks-base LayoutInflater.java#createViewFromTag, createView, mConstructorSignature, rInflate, rInflateChildren
#[test]
fn frameworks_layout_inflater_initializes_non_root_xml_children() {
    let dex_file =
        crate::dex::parse_dex_file("classes.dex", minimal_inflated_child_constructor_dex())
            .expect("inflated child constructor DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let layout_id = 42;
    let root = DexValue::Reference(std::rc::Rc::from("Landroid/widget/FrameLayout;:layout/42"));
    let root_key = map_identity_key(&root);
    let events = vec![
        XmlEvent::Start(crate::binary_xml::XmlStartElement {
            name: "FrameLayout".to_owned(),
            namespace: None,
            attributes: Vec::new(),
        }),
        XmlEvent::Start(crate::binary_xml::XmlStartElement {
            name: "example.InflatedChild".to_owned(),
            namespace: None,
            attributes: Vec::new(),
        }),
        XmlEvent::End(crate::binary_xml::XmlEndElement {
            name: "example.InflatedChild".to_owned(),
            namespace: None,
        }),
        XmlEvent::End(crate::binary_xml::XmlEndElement {
            name: "FrameLayout".to_owned(),
            namespace: None,
        }),
    ];

    program
        .register_layout_events(
            layout_id,
            events,
            Some(&root),
            &root_key,
            None,
            None,
            0,
            Some(0),
        )
        .expect("layout event registration should initialize every inflated view");

    let child = layout_node_reference(layout_id, 1, "Lexample/InflatedChild;");
    let initialized = DexFieldId {
        class_descriptor: "Lexample/InflatedChild;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "initialized".to_owned(),
    };
    assert_eq!(
        program.read_stored_instance_field(&child, &initialized),
        Some(DexValue::Int(1))
    );
}

// cite: frameworks-base LayoutInflater.java#createViewFromTag, createView, mConstructorSignature, rInflate, rInflateChildren
// cite: frameworks-base TypedArray.java#getInt
// cite: frameworks-base public-final.xml#gravity
#[test]
fn frameworks_layout_inflater_positions_attribute_set_for_each_child_constructor() {
    let dex_file =
        crate::dex::parse_dex_file("classes.dex", minimal_inflated_attribute_constructor_dex())
            .expect("inflated attribute constructor DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let layout_id = 43;
    let descriptor = "Lexample/InflatedAttributeChild;";
    let root = DexValue::Reference(std::rc::Rc::from("Landroid/widget/FrameLayout;:layout/43"));
    let root_key = map_identity_key(&root);
    let events = vec![
        XmlEvent::Start(crate::binary_xml::XmlStartElement {
            name: "FrameLayout".to_owned(),
            namespace: None,
            attributes: Vec::new(),
        }),
        XmlEvent::Start(crate::binary_xml::XmlStartElement {
            name: "example.InflatedAttributeChild".to_owned(),
            namespace: None,
            attributes: vec![crate::binary_xml::XmlAttribute {
                name: "gravity".to_owned(),
                namespace: Some("http://schemas.android.com/apk/res/android".to_owned()),
                value: crate::binary_xml::XmlValue::Hex(0x30),
            }],
        }),
        XmlEvent::End(crate::binary_xml::XmlEndElement {
            name: "example.InflatedAttributeChild".to_owned(),
            namespace: None,
        }),
        XmlEvent::Start(crate::binary_xml::XmlStartElement {
            name: "example.InflatedAttributeChild".to_owned(),
            namespace: None,
            attributes: vec![crate::binary_xml::XmlAttribute {
                name: "gravity".to_owned(),
                namespace: Some("http://schemas.android.com/apk/res/android".to_owned()),
                value: crate::binary_xml::XmlValue::Hex(0x50),
            }],
        }),
        XmlEvent::End(crate::binary_xml::XmlEndElement {
            name: "example.InflatedAttributeChild".to_owned(),
            namespace: None,
        }),
        XmlEvent::End(crate::binary_xml::XmlEndElement {
            name: "FrameLayout".to_owned(),
            namespace: None,
        }),
    ];

    program
        .register_layout_events(
            layout_id,
            events,
            Some(&root),
            &root_key,
            None,
            None,
            0,
            Some(0),
        )
        .expect("layout event registration should preserve each child AttributeSet position");

    let first_child = layout_node_reference(layout_id, 1, descriptor);
    let second_child = layout_node_reference(layout_id, 2, descriptor);
    let gravity_value = DexFieldId {
        class_descriptor: descriptor.to_owned(),
        type_descriptor: "I".to_owned(),
        name: "gravityValue".to_owned(),
    };
    assert_eq!(
        program.read_stored_instance_field(&first_child, &gravity_value),
        Some(DexValue::Int(0x30))
    );
    assert_eq!(
        program.read_stored_instance_field(&second_child, &gravity_value),
        Some(DexValue::Int(0x50))
    );
}

// cite: frameworks-base LayoutInflater.java#createView, mConstructorSignature
#[test]
fn frameworks_layout_inflater_rejects_dex_view_without_required_constructor() {
    let dex_file = crate::dex::parse_dex_file(
        "classes.dex",
        minimal_class_stub_dex("Lexample/InflatedChild;"),
    )
    .expect("inflated child class DEX should parse");
    let program = DexProgram::new(vec![dex_file]);
    let child = DexValue::Reference(std::rc::Rc::from(
        "Lexample/InflatedChild;:layout/42/node/1",
    ));
    let attrs = xml_resource_parser_reference(42);

    let error = program
        .initialize_inflated_layout_root(&child, &attrs, 0)
        .expect_err("DEX view without the XML constructor should fail inflation");

    assert!(error
        .to_string()
        .contains("inflated DEX view lacks required constructor"));
}

#[test]
fn builtin_menu_inflater_constructor_returns_void() {
    let program = DexProgram::new(Vec::new());
    let constructor = DexMethodId {
        class_descriptor: "Landroid/view/MenuInflater;".to_owned(),
        name: "<init>".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/content/Context;".to_owned()],
        },
    };
    let invocation = test_invocation(vec![
        DexValue::Reference(std::rc::Rc::from("Landroid/view/MenuInflater;.bootstrap")),
        DexValue::Reference(std::rc::Rc::from("Landroid/content/Context;.bootstrap")),
    ]);

    assert_eq!(
        program.invoke_builtin_method(&constructor, Some(&invocation)),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_appcompat_decor_content_parent_set_window_callback_returns_void() {
    let program = DexProgram::new(Vec::new());
    let set_window_callback = DexMethodId {
        class_descriptor: "Ln/n0;".to_owned(),
        name: "setWindowCallback".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Landroid/view/Window$Callback;".to_owned()],
        },
    };
    let set_window_title = DexMethodId {
        class_descriptor: "Ln/n0;".to_owned(),
        name: "setWindowTitle".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "VL".to_owned(),
            return_type: "V".to_owned(),
            parameters: vec!["Ljava/lang/CharSequence;".to_owned()],
        },
    };

    assert_eq!(
        program.invoke_builtin_method(&set_window_callback, None),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        program.invoke_builtin_method(&set_window_title, None),
        Some(DexExecutionResult::Void)
    );
}

#[test]
fn builtin_menu_item_get_item_id_decodes_synthetic_action_bar_item() {
    let program = DexProgram::new(Vec::new());
    let method = DexMethodId {
        class_descriptor: "Landroid/view/MenuItem;".to_owned(),
        name: "getItemId".to_owned(),
        proto: crate::dex::DexProto {
            shorty: "I".to_owned(),
            return_type: "I".to_owned(),
            parameters: Vec::new(),
        },
    };
    let item = DexValue::Reference(std::rc::Rc::from(
        "Landroid/view/MenuItem;:option/2131034113",
    ));

    assert_eq!(
        program.invoke_builtin_method(&method, Some(&test_invocation(vec![item]))),
        Some(DexExecutionResult::Int(2131034113))
    );
}

// cite: frameworks-base LayoutInflater.java#setFactory2, getFactory, getFactory2
#[test]
fn builtin_layout_inflater_set_factory2_is_read_back_by_both_getters() {
    let program = DexProgram::new(Vec::new());
    let class = "Landroid/view/LayoutInflater;";
    let set = builtin_method(
        class,
        "setFactory2",
        "V",
        &["Landroid/view/LayoutInflater$Factory2;"],
    );
    let get = builtin_method(
        class,
        "getFactory",
        "Landroid/view/LayoutInflater$Factory;",
        &[],
    );
    let get2 = builtin_method(
        class,
        "getFactory2",
        "Landroid/view/LayoutInflater$Factory2;",
        &[],
    );
    let inflater =
        DexValue::Reference(std::rc::Rc::from("Landroid/view/LayoutInflater;.bootstrap"));
    let factory = program.allocate_object("Lcom/example/Factory;".to_owned());
    let call = |method: &DexMethodId, mut arguments: Vec<DexValue>| {
        arguments.insert(0, inflater.clone());
        program.invoke_builtin_method(method, Some(&test_invocation(arguments)))
    };

    assert_eq!(
        call(&get2, vec![]),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert!(matches!(
        call(&set, vec![DexValue::Null]),
        Some(DexExecutionResult::Throw(_))
    ));
    assert_eq!(
        call(&set, vec![factory.clone()]),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        call(&get, vec![]),
        Some(DexExecutionResult::Value(factory.clone()))
    );
    assert_eq!(
        call(&get2, vec![]),
        Some(DexExecutionResult::Value(factory))
    );
}

// cite: frameworks-base LayoutInflater.java#getContext
#[test]
fn builtin_layout_inflater_get_context_returns_the_current_activity() {
    let program = DexProgram::new(Vec::new());
    let get_context = builtin_method(
        "Landroid/view/LayoutInflater;",
        "getContext",
        "Landroid/content/Context;",
        &[],
    );
    let inflater =
        DexValue::Reference(std::rc::Rc::from("Landroid/view/LayoutInflater;.bootstrap"));
    let activity = program.allocate_object("Lcom/example/MainActivity;".to_owned());
    program.runtime.borrow_mut().activity_receiver = Some(activity.clone());

    assert_eq!(
        program.invoke_builtin_method(&get_context, Some(&test_invocation(vec![inflater]))),
        Some(DexExecutionResult::Value(activity))
    );
}

// cite: frameworks-base LayoutInflater.java#setFactory, setFactory2, cloneInContext, LayoutInflater
#[test]
fn builtin_layout_inflater_factories_are_set_once_per_context_and_copied_by_clone() {
    let program = DexProgram::new(Vec::new());
    let class = "Landroid/view/LayoutInflater;";
    let from = builtin_method(class, "from", class, &["Landroid/content/Context;"]);
    let set = builtin_method(
        class,
        "setFactory",
        "V",
        &["Landroid/view/LayoutInflater$Factory;"],
    );
    let set2 = builtin_method(
        class,
        "setFactory2",
        "V",
        &["Landroid/view/LayoutInflater$Factory2;"],
    );
    let get = builtin_method(
        class,
        "getFactory",
        "Landroid/view/LayoutInflater$Factory;",
        &[],
    );
    let get2 = builtin_method(
        class,
        "getFactory2",
        "Landroid/view/LayoutInflater$Factory2;",
        &[],
    );
    let clone = builtin_method(
        class,
        "cloneInContext",
        class,
        &["Landroid/content/Context;"],
    );
    let call = |method: &DexMethodId, arguments: Vec<DexValue>| {
        program.invoke_builtin_method(method, Some(&test_invocation(arguments)))
    };
    let inflater_for = |context: &DexValue| match call(&from, vec![context.clone()]) {
        Some(DexExecutionResult::Value(inflater)) => inflater,
        other => panic!("from should return an inflater, got {other:?}"),
    };
    let first_activity = program.allocate_object("Lcom/example/First;".to_owned());
    let second_activity = program.allocate_object("Lcom/example/Second;".to_owned());
    let wrapper = program.allocate_object("Lcom/example/Wrapper;".to_owned());
    let factory = program.allocate_object("Lcom/example/Factory;".to_owned());
    let other = program.allocate_object("Lcom/example/OtherFactory;".to_owned());
    let first = inflater_for(&first_activity);

    assert_eq!(
        call(&set, vec![first.clone(), factory.clone()]),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        call(&get, vec![first.clone()]),
        Some(DexExecutionResult::Value(factory.clone()))
    );
    assert_eq!(
        call(&get2, vec![first.clone()]),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    let Some(DexExecutionResult::Throw(error)) = call(&set2, vec![first.clone(), other.clone()])
    else {
        panic!("a second factory on the same inflater should throw");
    };
    assert!(program.value_matches_type(&error, "Ljava/lang/IllegalStateException;"));

    let second = inflater_for(&second_activity);
    assert_eq!(
        call(&get, vec![second.clone()]),
        Some(DexExecutionResult::Value(DexValue::Null))
    );
    assert_eq!(
        call(&set2, vec![second.clone(), other.clone()]),
        Some(DexExecutionResult::Void)
    );
    assert_eq!(
        call(&get2, vec![second]),
        Some(DexExecutionResult::Value(other.clone()))
    );

    let Some(DexExecutionResult::Value(cloned)) = call(&clone, vec![first, wrapper]) else {
        panic!("cloneInContext should return an inflater");
    };
    assert_eq!(
        call(&get, vec![cloned.clone()]),
        Some(DexExecutionResult::Value(factory))
    );
    assert_eq!(
        call(&set2, vec![cloned.clone(), other.clone()]),
        Some(DexExecutionResult::Void)
    );
    let Some(DexExecutionResult::Value(merged)) = call(&get2, vec![cloned.clone()]) else {
        panic!("getFactory2 should return the merged factory");
    };
    assert!(program.value_matches_type(&merged, "Landroid/view/LayoutInflater$FactoryMerger;"));
    assert_eq!(
        call(&get, vec![cloned]),
        Some(DexExecutionResult::Value(merged))
    );
}
