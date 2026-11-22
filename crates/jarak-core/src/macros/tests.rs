//! The macro file, run-command variables and targets.

use super::*;

#[test]
fn the_file_keeps_macros_and_commands_and_refuses_what_it_does_not_know() {
    let d = std::env::temp_dir().join(format!("jarak-macros-{}", std::process::id()));
    let f = d.join("macros.json");
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        UserCommands::load(&f).unwrap(),
        UserCommands::default(),
        "none yet"
    );
    let u = UserCommands {
        format: FORMAT.into(),
        macros: vec![Macro {
            name: "줄 정리".into(),
            keys: Some("Ctrl+Alt+1".into()),
            steps: vec![
                Step::Cmd("line.duplicate".into()),
                Step::Type("한글 text".into()),
                Step::Action("paste".into()),
                Step::Find(FindStep {
                    op: "replace_all".into(),
                    find: "a+".into(),
                    replace: "b".into(),
                    mode: "regex".into(),
                    match_case: true,
                    whole_word: false,
                    wrap: true,
                    backward: false,
                    in_selection: false,
                    dot_all: false,
                }),
            ],
        }],
        commands: vec![RunCommand {
            name: "Notepad".into(),
            command: "notepad.exe \"$(FULL_CURRENT_PATH)\"".into(),
            keys: None,
        }],
    };
    u.save(&f).unwrap();
    assert_eq!(UserCommands::load(&f).unwrap(), u, "a round trip");
    assert!(!d.join("macros.json.tmp").exists(), "written atomically");
    // A find step written by hand takes the defaults.
    std::fs::write(
        &f,
        r#"{"format": "jarak.macros/1", "macros": [{"name": "m", "steps": [{"find": {"op": "next", "find": "x"}}]}]}"#,
    )
    .unwrap();
    let m = &UserCommands::load(&f).unwrap().macros[0];
    let Step::Find(s) = &m.steps[0] else { panic!() };
    assert_eq!(
        (s.mode.as_str(), s.wrap, s.match_case),
        ("normal", true, false)
    );
    std::fs::write(&f, r#"{"format": "other/9"}"#).unwrap();
    assert!(
        UserCommands::load(&f)
            .unwrap_err()
            .contains("unknown format")
    );
    std::fs::write(&f, "not json").unwrap();
    assert!(
        UserCommands::load(&f).is_err(),
        "broken: an error, not an empty list"
    );
    let _ = std::fs::remove_dir_all(&d);
    // The built-in macro trims and saves.
    assert_eq!(
        builtin_macros()[0].steps[0],
        Step::Cmd("blank.trim-trailing".into())
    );
    assert!(
        crate::command::COMMANDS
            .iter()
            .any(|c| c.id == "blank.trim-trailing")
    );
}

#[test]
fn variables_expand_and_lines_split_into_targets() {
    let v = Vars {
        full_current_path: r"C:\docs\a b.md".into(),
        current_directory: r"C:\docs".into(),
        file_name: "a b.md".into(),
        name_part: "a b".into(),
        ext_part: ".md".into(),
        current_word: "자락".into(),
        current_line: 12,
        current_column: 3,
        current_line_text: "the line".into(),
        jarak_directory: r"C:\jarak".into(),
        jarak_full_file_path: r"C:\jarak\jarak-gui.exe".into(),
    };
    let env = |n: &str| (n == "USERNAME").then(|| "kim".to_string());
    assert_eq!(
        expand(
            "code \"$(FULL_CURRENT_PATH)\" -g $(CURRENT_LINE):$(CURRENT_COLUMN)",
            &v,
            &env
        ),
        r#"code "C:\docs\a b.md" -g 12:3"#
    );
    assert_eq!(
        expand("$(NAME_PART)$(EXT_PART) in $(CURRENT_DIRECTORY)", &v, &env),
        r"a b.md in C:\docs"
    );
    assert_eq!(
        expand("$(NPP_DIRECTORY) $(SYS.USERNAME) [$(SYS.NOPE)]", &v, &env),
        r"C:\jarak kim []"
    );
    assert_eq!(
        expand("$(UNKNOWN) $(CURRENT_WORD) $(", &v, &env),
        "$(UNKNOWN) 자락 $("
    );
    // Targets.
    assert_eq!(target("  "), None);
    assert_eq!(
        target("https://example.org/?q=a"),
        Some(Target::Web("https://example.org/?q=a".into()))
    );
    assert_eq!(
        target(r#""C:\Program Files\App\app.exe" "C:\docs\a b.md" -x"#),
        Some(Target::Program {
            program: r"C:\Program Files\App\app.exe".into(),
            args: r#""C:\docs\a b.md" -x"#.into()
        })
    );
    assert_eq!(
        target("notepad"),
        Some(Target::Program {
            program: "notepad".into(),
            args: String::new()
        })
    );
    assert_eq!(
        target(r"C:\docs\page.html"),
        Some(Target::File(r"C:\docs\page.html".into()))
    );
    assert_eq!(
        target("cmd /c echo hi"),
        Some(Target::Program {
            program: "cmd".into(),
            args: "/c echo hi".into()
        })
    );
}
