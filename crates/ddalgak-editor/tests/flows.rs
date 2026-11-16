//! Headless flow tests of the Ddalgak rule editor (egui_kittest: no window, no GPU), driven
//! through the accessibility tree like a keyboard or screen-reader user, against an isolated
//! rules folder (docs/DDALGAK_GUI_JOURNEYS.md J1–J4, J7; `DDALGAK-FR-050…053`,
//! `DDALGAK-NFR-007`, `DDALGAK-SEC-013`).
//!
//! Machine safety: nothing here sends a key to another program, moves the pointer, opens a
//! program or a website, or starts the tray app. The engine is `FakeEngine` (it reloads with the
//! engine's own loader and answers in the app's reply shape) or `NoEngine`; "Test" is the
//! simulate mode.

use ddalgak::paths::Paths;
use ddalgak_editor::form::Effect;
use ddalgak_editor::i18n::Lang;
use ddalgak_editor::listing::{Grouping, StateFilter};
use ddalgak_editor::repo::Repo;
use ddalgak_editor::sync::{Engine, EngineView, FakeEngine, NoEngine, SaveState};
use ddalgak_editor::{EditorApp, Options};
use eframe::egui::{self, Key, Modifiers, accesskit::Role};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

struct Sandbox {
    root: PathBuf,
    paths: Paths,
}

impl Sandbox {
    fn new(tag: &str) -> Sandbox {
        let root =
            std::env::temp_dir().join(format!("ddalgak-editor-ui-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Sandbox {
            paths: Paths::under(&root),
            root,
        }
    }

    fn write_rules(&self, doc: &Value) {
        std::fs::create_dir_all(&self.paths.config).unwrap();
        std::fs::write(self.paths.rules(), serde_json::to_vec_pretty(doc).unwrap()).unwrap();
    }

    fn rules(&self) -> Value {
        serde_json::from_slice(&std::fs::read(self.paths.rules()).unwrap()).unwrap()
    }

    fn fake(&self) -> Arc<FakeEngine> {
        Arc::new(FakeEngine::new(self.paths.rules()))
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

const GRANT: &str = "2026-11-04T09:00:00Z";

#[allow(clippy::needless_pass_by_value)]
fn doc(rules: Value) -> Value {
    json!({
        "schema": "ddalgak.rules", "schema_version": 1, "hotkey_contract": 1, "revision": 1,
        "active_profile": "work",
        "profiles": [ { "id": "work", "title": "Work", "default_mode": "base",
            "modes": [ { "id": "base", "title": "Base" }, { "id": "nav", "title": "Navigation" } ],
            "rules": rules } ]
    })
}

fn three_rules() -> Value {
    doc(json!([
        { "id": "caps", "title": "CapsLock acts as Ctrl", "enabled": true,
          "trigger": { "keys": "CapsLock" }, "steps": [ { "type": "remap", "to": "LCtrl", "tap": "Escape" } ],
          "consent": { "input-injection": { "granted_at": GRANT, "via": "gui" } } },
        { "id": "pause", "title": "F13 pauses", "enabled": true,
          "trigger": { "keys": "Ctrl+Alt+Shift+F13" }, "steps": [ { "type": "engine", "op": "toggle_pause" } ] },
        { "id": "thanks", "title": "Thanks", "enabled": false, "modes": ["nav"],
          "trigger": { "keys": "Ctrl+Shift+F2" }, "steps": [ { "type": "type_text", "text": "감사합니다." } ] }
    ]))
}

fn app(sb: &Sandbox, engine: Arc<dyn Engine>) -> EditorApp {
    EditorApp::new(Options::for_test(sb.paths.clone(), engine))
}

fn harness(app: EditorApp) -> Harness<'static, EditorApp> {
    Harness::builder()
        // ADR 0100: tall enough for the rule editor's four sections without scrolling (a
        // control outside the scrolled view is not clickable, as for a real user).
        .with_size(egui::vec2(1280.0, 1500.0))
        // The header asks the engine for its state once a second; under load that answer can
        // arrive inside a run and cost two more frames (it made `run` exceed 4 steps).
        .with_max_steps(12)
        .build_ui_state(|ui, app: &mut EditorApp| app.ui(ui), app)
}

/// Runs frames, waits for the app's answers, runs again.
fn settle(h: &mut Harness<'static, EditorApp>) {
    h.run();
    assert!(h.state_mut().wait_idle(Duration::from_secs(10)));
    h.run();
}

fn click(h: &mut Harness<'static, EditorApp>, label: &str) {
    // Like a user: scrolled into view first (the editor's sections scroll under a fixed footer).
    h.get_by_label(label).scroll_to_me();
    h.run();
    h.get_by_label(label).click();
    h.run();
}

/// The last button with this label (a dialog's button is drawn after the panel behind it).
fn click_last_button(h: &mut Harness<'static, EditorApp>, label: &str) {
    h.get_all_by_role_and_label(Role::Button, label)
        .last()
        .unwrap_or_else(|| panic!("no button {label}"))
        .click();
    h.run();
}

/// ADR 0100: opens the Settings window with the gear (an embedded window in the harness) and
/// goes to `section`.
fn open_settings(h: &mut Harness<'static, EditorApp>, section: &str) {
    h.get_all_by_role_and_label(Role::Button, "Settings")
        .next()
        .expect("the settings gear")
        .click();
    h.run();
    h.get_all_by_label(section)
        .next()
        .unwrap_or_else(|| panic!("no settings section {section}"))
        .click();
    h.run();
}

/// ADR 0100: a rule row's `⋯` menu, then one of its items.
fn row_menu(h: &mut Harness<'static, EditorApp>, title: &str, item: &str) {
    h.get_by_label(&format!("More actions: {title}")).click();
    h.run();
    click_last_button_or_item(h, item);
}

/// The last button or menu item with this label.
fn click_last_button_or_item(h: &mut Harness<'static, EditorApp>, label: &str) {
    let node = h
        .get_all_by_label(label)
        .rfind(|n| {
            matches!(
                n.accesskit_node().role(),
                Role::Button | Role::MenuItem | Role::RadioButton
            )
        })
        .unwrap_or_else(|| panic!("no button or menu item {label}"));
    node.click();
    h.run();
}

/// A text field by its name (its visible label above it has the same name).
fn input<'h>(h: &'h Harness<'static, EditorApp>, label: &'h str) -> egui_kittest::Node<'h> {
    h.get_all_by_label(label)
        .find(|n| {
            matches!(
                n.accesskit_node().role(),
                Role::TextInput | Role::MultilineTextInput | Role::SearchInput
            )
        })
        .unwrap_or_else(|| panic!("no text field {label}"))
}

/// ADR 0100: types key names into a key field (its Type button opens the text field).
fn type_keys(h: &mut Harness<'static, EditorApp>, field: &str, keys: &str) {
    let name = format!("Type instead: {field}");
    if h.query_all_by_label(&name).next().is_none() {
        h.get_by_label(&format!("Type: {field}")).click();
        h.run();
    }
    input(h, &name).set_text_value_accesskit(keys);
    h.run();
}

fn rule<'a>(d: &'a Value, id: &str) -> &'a Value {
    d["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|p| p["rules"].as_array().unwrap())
        .find(|r| r["id"] == id)
        .unwrap_or_else(|| panic!("no rule {id}"))
}

fn ids(d: &Value) -> Vec<String> {
    d["profiles"][0]["rules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap().to_string())
        .collect()
}

/// J1: the first start shows one sentence and three starters, creates nothing, and says that
/// Ddalgak is not running.
#[test]
fn j1_first_start_is_safe_and_a_starter_is_only_a_draft() {
    let sb = Sandbox::new("j1");
    let mut h = harness(app(&sb, Arc::new(NoEngine)));
    settle(&mut h);
    assert!(h.query_by_label_contains("No rules yet").is_some());
    assert!(
        h.query_by_label_contains("Emergency off: Ctrl+Alt+Shift+Esc")
            .is_some()
    );
    // ADR 0100: the rail shows the engine's real state as a status, not a switch: no app.
    assert_eq!(h.state().engine_view(), &EngineView::NotRunning);
    assert!(
        h.query_by_label("Not running").is_some(),
        "the rail's status"
    );
    assert!(
        h.query_all_by_role_and_label(Role::Switch, "Turn on Ddalgak")
            .next()
            .is_none(),
        "the engine switch lives in Settings"
    );
    assert!(
        !sb.paths.rules().exists(),
        "opening the editor writes nothing"
    );
    click(&mut h, "CapsLock acts as Ctrl");
    let f = h.state().form().expect("the starter opens the form");
    assert_eq!(f.fields.effect, Some(Effect::Remap));
    assert!(h.state().has_unsaved_changes());
    assert!(
        !sb.paths.rules().exists(),
        "a starter is a draft until saved"
    );
    assert!(h.query_by_label_contains("draft: not saved").is_some());
}

/// J2/J3/J4: capture the keys, pick a tile, fill the form, simulate, save and enable; consent is
/// asked first (Cancel focused), and "saved and active" appears only after the app's reload.
#[test]
fn j2_create_a_rule_with_capture_tile_form_simulation_and_consent() {
    let sb = Sandbox::new("j2");
    let fake = sb.fake();
    let mut h = harness(app(&sb, fake.clone()));
    settle(&mut h);
    click(&mut h, "New Rule");
    // The capture box records on key-down.
    h.key_press_modifiers(Modifiers::CTRL | Modifiers::SHIFT, Key::F2);
    h.run();
    assert_eq!(
        h.state().form().unwrap().fields.trigger.keys,
        "Ctrl+Shift+F2"
    );
    assert!(
        h.query_by_label("Control plus Shift plus F2").is_some(),
        "announced in words"
    );
    h.key_press(Key::Escape); // ends the capture; the value stays
    h.run();
    assert!(
        h.state().form().is_some(),
        "Esc in the capture box does not close the form"
    );
    click(&mut h, "Type text");
    assert_eq!(
        h.state().form().unwrap().fields.effect,
        Some(Effect::TypeText)
    );
    input(&h, "Text").set_text_value_accesskit("감사합니다.\n김민수 드림");
    h.run();
    assert!(
        h.query_all_by_label_contains("Types 13 characters on 2 lines")
            .next()
            .is_some()
    );
    click(&mut h, "Simulate");
    let sim = h.state().simulation().expect("simulated").clone();
    assert!(sim.fires);
    assert_eq!(sim.typed.as_deref(), Some("감사합니다.\n김민수 드림"));
    assert!(
        !sb.paths.rules().exists(),
        "simulating writes and types nothing"
    );
    click(&mut h, "Save and enable");
    // Consent first: the safe choice has the focus.
    assert!(h.query_by_label("Allow this rule?").is_some());
    assert!(
        h.get_all_by_role_and_label(Role::Button, "Cancel")
            .last()
            .unwrap()
            .is_focused()
    );
    assert!(
        !sb.paths.rules().exists(),
        "nothing is written before Allow"
    );
    click_last_button(&mut h, "Allow");
    let saved = sb.rules();
    let r = &saved["profiles"][0]["rules"][0];
    assert_eq!(r["enabled"], true);
    assert_eq!(r["trigger"]["keys"], "Ctrl+Shift+F2");
    assert_eq!(r["consent"]["input-injection"]["via"], "gui");
    settle(&mut h);
    let rev = saved["revision"].as_u64().unwrap();
    assert_eq!(h.state().save_state(), &SaveState::Active { revision: rev });
    assert!(h.query_by_label_contains("saved and active").is_some());
    assert!(fake.with(|s| s.calls.iter().any(|c| c == "rules.reload")));
    assert!(h.state().form().is_none());
}

/// DDALGAK-FR-052: a save shows "saved; waiting" until the app's reload answer arrives, then
/// "saved and active"; with the engine off or Ddalgak not running it says so instead.
#[test]
fn a_save_is_acknowledged_only_after_the_engine_reload() {
    let sb = Sandbox::new("ack");
    sb.write_rules(&three_rules());
    let fake = sb.fake();
    let mut h = harness(app(&sb, fake.clone()));
    settle(&mut h);
    assert_eq!(h.state().save_state(), &SaveState::Active { revision: 1 });
    fake.with(|s| s.delay_ms = 400);
    click(&mut h, "Enabled: F13 pauses");
    let rev = sb.rules()["revision"].as_u64().unwrap();
    assert_eq!(rev, 2);
    assert_eq!(h.state().save_state(), &SaveState::Waiting { revision: 2 });
    assert!(
        h.query_by_label_contains("saved; waiting for Ddalgak to load it")
            .is_some()
    );
    assert!(
        h.query_by_label_contains("saved and active").is_none(),
        "never shown before the engine confirmed"
    );
    settle(&mut h);
    assert_eq!(h.state().save_state(), &SaveState::Active { revision: 2 });
    // The engine is off: saved and loaded, not active.
    fake.with(|s| {
        s.delay_ms = 0;
        s.engine = "off".into();
    });
    click(&mut h, "Enabled: F13 pauses");
    settle(&mut h);
    assert_eq!(
        h.state().save_state(),
        &SaveState::Loaded {
            revision: 3,
            engine: "off".into()
        }
    );
    // Ddalgak is not running.
    fake.with(|s| s.running = false);
    click(&mut h, "Enabled: F13 pauses");
    settle(&mut h);
    assert_eq!(
        h.state().save_state(),
        &SaveState::NotRunning { revision: 4 }
    );
    assert!(
        h.query_by_label_contains("Ddalgak is not running, so nothing is active")
            .is_some()
    );
}

/// J7: edit, duplicate, enable with consent, disable, delete with confirmation and undo.
#[test]
fn edit_duplicate_enable_disable_delete_and_undo() {
    let sb = Sandbox::new("crud");
    sb.write_rules(&three_rules());
    let mut h = harness(app(&sb, sb.fake()));
    settle(&mut h);
    // Edit opens the panel at step 3 with the rule's values.
    click(
        &mut h,
        "CapsLock  →  CapsLock acts as Left Ctrl; a quick tap is Escape",
    );
    assert!(h.state().form().unwrap().id.is_some());
    input(&h, "Name").set_text_value_accesskit("Caps is Ctrl");
    h.run();
    assert!(h.state().has_unsaved_changes());
    click(&mut h, "Save");
    let d = sb.rules();
    assert_eq!(rule(&d, "caps")["title"], "Caps is Ctrl");
    assert_eq!(
        rule(&d, "caps")["consent"]["input-injection"]["via"],
        "gui",
        "consent kept"
    );
    // Duplicate: a disabled copy right after the original.
    row_menu(&mut h, "Caps is Ctrl", "Duplicate");
    let d = sb.rules();
    let all = ids(&d);
    assert_eq!(all.len(), 4);
    let copy = all[1].clone();
    assert_eq!(rule(&d, &copy)["title"], "Caps is Ctrl (copy)");
    assert_eq!(rule(&d, &copy)["enabled"], false);
    // Enabling the disabled snippet needs consent: Cancel keeps it off, Allow enables it.
    click(&mut h, "Enabled: Thanks");
    assert!(h.query_by_label("Allow this rule?").is_some());
    click_last_button(&mut h, "Cancel");
    assert_eq!(rule(&sb.rules(), "thanks")["enabled"], false);
    click(&mut h, "Enabled: Thanks");
    click_last_button(&mut h, "Allow");
    let d = sb.rules();
    assert_eq!(rule(&d, "thanks")["enabled"], true);
    assert_eq!(
        rule(&d, "thanks")["consent"]["input-injection"]["via"],
        "gui"
    );
    // Delete asks first, Cancel focused; then an Undo toast that stays until dismissed.
    row_menu(&mut h, "Thanks", "Delete");
    assert!(h.query_by_label("Delete rules").is_some());
    assert!(
        h.get_all_by_role_and_label(Role::Button, "Cancel")
            .last()
            .unwrap()
            .is_focused()
    );
    click_last_button(&mut h, "Delete (1)");
    assert!(!ids(&sb.rules()).contains(&"thanks".to_string()));
    assert!(
        h.query_by_label_contains("Deleted 1 rule(s): Thanks")
            .is_some()
    );
    for _ in 0..5 {
        h.run();
    }
    assert!(
        h.query_by_label_contains("Deleted 1 rule(s)").is_some(),
        "the toast stays"
    );
    click_last_button(&mut h, "Undo");
    let d = sb.rules();
    assert!(ids(&d).contains(&"thanks".to_string()), "undo restores it");
    assert_eq!(rule(&d, "thanks")["enabled"], true);
}

/// #79 (seen in the real editor): the Undo toast of a delete stayed after a later change, and its
/// Undo then undid that later change instead of the delete. A newer change removes the toast.
#[test]
fn a_later_change_retires_the_undo_toast_of_a_delete() {
    let sb = Sandbox::new("stale-undo");
    sb.write_rules(&three_rules());
    let mut h = harness(app(&sb, sb.fake()));
    settle(&mut h);
    row_menu(&mut h, "Thanks", "Delete");
    click_last_button(&mut h, "Delete (1)");
    assert!(h.query_by_label_contains("Deleted 1 rule(s)").is_some());
    let before = ids(&sb.rules()).len();
    // Another change: duplicate the remaining first rule.
    row_menu(&mut h, "CapsLock acts as Ctrl", "Duplicate");
    settle(&mut h);
    assert_eq!(ids(&sb.rules()).len(), before + 1);
    assert!(
        h.query_by_label_contains("Deleted 1 rule(s)").is_none(),
        "the delete's Undo toast is gone once a newer change exists"
    );
    assert!(!ids(&sb.rules()).contains(&"thanks".to_string()));
}

/// Every effect tile opens its own form, and its problems are shown in words with Save blocked
/// until the form is valid.
#[test]
fn every_action_form_validates_in_the_window() {
    let sb = Sandbox::new("forms");
    sb.write_rules(&three_rules());
    let mut h = harness(app(&sb, sb.fake()));
    settle(&mut h);
    let tiles = [
        ("Act as another key", Effect::Remap),
        ("Press a shortcut", Effect::SendKeys),
        ("Type text", Effect::TypeText),
        ("Open a program", Effect::OpenProgram),
        ("Open a website", Effect::OpenUrl),
        ("Mouse", Effect::Mouse),
        ("Switch mode", Effect::Mode),
        ("Automation on/off", Effect::Engine),
        ("Block this key", Effect::Block),
        // ADR 0101.
        ("Media control", Effect::Media),
        ("Volume", Effect::Volume),
        ("Open Settings", Effect::Settings),
    ];
    for (i, (tile, effect)) in tiles.into_iter().enumerate() {
        click(&mut h, "New Rule");
        h.key_press(Key::Escape);
        h.run();
        let keys = format!("Ctrl+Alt+Shift+F{}", 13 + i);
        type_keys(&mut h, "Keys", &keys);
        click(&mut h, tile);
        assert_eq!(
            h.state().form().unwrap().fields.effect,
            Some(effect),
            "{tile}"
        );
        let needs_input = !matches!(
            effect,
            Effect::Engine
                | Effect::Block
                | Effect::Mouse
                | Effect::Media
                | Effect::Volume
                | Effect::Settings
        );
        let blocked = !h
            .get_by_role_and_label(Role::Button, "Save and enable")
            .accesskit_node()
            .is_disabled();
        assert_eq!(
            !blocked, needs_input,
            "{tile}: Save blocked while incomplete"
        );
        if needs_input {
            assert!(
                h.query_all_by_label_contains("✕ ").next().is_some(),
                "{tile}: a problem is shown in words"
            );
        }
        // Fill the required field and the form is ready.
        match effect {
            Effect::Remap => type_keys(&mut h, "Acts as", "F20"),
            Effect::SendKeys => type_keys(&mut h, "Shortcut (up to eight chords)", "Ctrl+C"),
            Effect::TypeText => input(&h, "Text").set_text_value_accesskit("hello"),
            Effect::OpenProgram => input(&h, "Program (.exe)")
                .set_text_value_accesskit(r"C:\Windows\System32\notepad.exe"),
            Effect::OpenUrl => input(&h, "Website").set_text_value_accesskit("example.com"),
            Effect::Mode => {
                h.state_mut().form_mut().unwrap().fields.mode.mode = "nav".into();
            }
            _ => {}
        }
        h.run();
        let f = h.state().form().unwrap();
        assert!(
            f.problems(None)
                .iter()
                .all(|p| p.path.starts_with("steps[0].mode")),
            "{tile}: {:?}",
            f.problems(None)
        );
        click(&mut h, "Cancel");
        if h.state().dialog_open() {
            click(&mut h, "Discard changes");
        }
        assert!(h.state().form().is_none(), "{tile}");
    }
    assert_eq!(sb.rules()["revision"], 1, "trying the forms wrote nothing");
}

/// Search, filters, grouping and bulk enable; the list shows only summaries of personal content.
#[test]
fn search_filter_group_and_bulk_enable() {
    let sb = Sandbox::new("search");
    sb.write_rules(&doc(json!([
        { "id": "caps", "title": "CapsLock acts as Ctrl", "enabled": true,
          "trigger": { "keys": "CapsLock" }, "steps": [ { "type": "remap", "to": "LCtrl" } ],
          "consent": { "input-injection": { "granted_at": GRANT, "via": "gui" } } },
        { "id": "sig", "title": "Signature", "enabled": false,
          "trigger": { "keys": "Ctrl+Shift+F3" }, "steps": [ { "type": "type_text", "text": "private words" } ],
          "consent": { "input-injection": { "granted_at": GRANT, "via": "gui" } } },
        { "id": "p1", "title": "Pause one", "enabled": false, "modes": ["nav"],
          "trigger": { "keys": "Ctrl+Alt+Shift+F14" }, "steps": [ { "type": "engine", "op": "pause" } ] },
        { "id": "p2", "title": "Pause two", "enabled": false, "when": { "apps": [ { "exe": "Code.exe" } ] },
          "trigger": { "keys": "Ctrl+Alt+Shift+F15" }, "steps": [ { "type": "engine", "op": "pause" } ] }
    ])));
    let mut h = harness(app(&sb, sb.fake()));
    settle(&mut h);
    assert!(
        h.query_by_label_contains("private words").is_none(),
        "snippet text is not shown"
    );
    // Ctrl+F focuses the search box (rules.search), typing filters.
    h.key_press_modifiers(Modifiers::CTRL, Key::F);
    h.run();
    assert!(h.get_by_label("Search rules").is_focused());
    h.get_by_label("Search rules").type_text("caps");
    h.run();
    assert_eq!(h.state().visible_ids(), ["caps"]);
    h.state_mut().filter_mut().query = "private".into();
    h.run();
    assert!(h.state().visible_ids().is_empty());
    // ADR 0096: the filters are behind "Filters".
    click(&mut h, "Filters");
    click(&mut h, "Also search snippet text");
    assert_eq!(h.state().visible_ids(), ["sig"]);
    h.state_mut().filter_mut().query.clear();
    h.state_mut().filter_mut().snippet_text = false;
    h.state_mut().filter_mut().state = StateFilter::Disabled;
    h.run();
    assert_eq!(h.state().visible_ids(), ["sig", "p1", "p2"]);
    h.state_mut().filter_mut().state = StateFilter::All;
    h.state_mut().filter_mut().app = Some("code.exe".into());
    h.run();
    assert_eq!(h.state().visible_ids(), ["p2"]);
    h.state_mut().filter_mut().app = None;
    h.state_mut().set_grouping(Grouping::Mode);
    h.run();
    assert!(h.query_by_label("Every mode").is_some());
    assert!(h.query_by_label("Navigation").is_some());
    h.state_mut().set_grouping(Grouping::App);
    h.run();
    assert!(h.query_by_label("Code.exe").is_some());
    h.state_mut().set_grouping(Grouping::None);
    h.run();
    // Bulk: select two with Ctrl+click (a plain click opens the rule), enable both at once.
    h.get_by_label_contains("Ctrl+Alt+Shift+F14")
        .click_modifiers(Modifiers::CTRL);
    h.run();
    h.get_by_label_contains("Ctrl+Alt+Shift+F15")
        .click_modifiers(Modifiers::CTRL);
    h.run();
    assert_eq!(h.state().selected(), ["p1", "p2"]);
    assert!(h.query_by_label("2 selected").is_some());
    click(&mut h, "Enable");
    let d = sb.rules();
    assert_eq!(rule(&d, "p1")["enabled"], true);
    assert_eq!(rule(&d, "p2")["enabled"], true);
    assert_eq!(d["revision"], 2, "one change for the bulk action");
    // Step 77: the app-limited rule is evaluated, so no inactive badge is shown for it.
    settle(&mut h);
    assert!(
        h.query_by_label_contains("inactive: condition_unsupported")
            .is_none()
    );
}

/// Drag ordering and Alt+Up/Alt+Down reorder within the profile (the list order is the last
/// precedence step).
#[test]
fn reorder_with_keys_and_drag() {
    let sb = Sandbox::new("order");
    sb.write_rules(&three_rules());
    let mut h = harness(app(&sb, sb.fake()));
    settle(&mut h);
    // Ctrl+click selects without opening the rule.
    h.get_by_label("CapsLock  →  CapsLock acts as Left Ctrl; a quick tap is Escape")
        .click_modifiers(Modifiers::CTRL);
    h.run();
    h.key_press_modifiers(Modifiers::ALT, Key::ArrowDown);
    h.run();
    assert_eq!(ids(&sb.rules()), ["pause", "caps", "thanks"]);
    h.key_press_modifiers(Modifiers::ALT, Key::ArrowUp);
    h.run();
    assert_eq!(ids(&sb.rules()), ["caps", "pause", "thanks"]);
    // Drag the third rule's handle onto the top half of the first row.
    let handle = h
        .get_by_label("Drag to reorder (or Alt+Up / Alt+Down): Thanks")
        .rect()
        .center();
    let target = h
        .get_by_label("CapsLock  →  CapsLock acts as Left Ctrl; a quick tap is Escape")
        .rect();
    h.drag_at(handle);
    h.run();
    for i in 0..6 {
        h.hover_at(handle + egui::vec2(0.0, (target.top() + 2.0 - handle.y) * (i as f32) / 5.0));
        h.run();
    }
    h.drop_at(egui::pos2(target.center().x, target.top() + 2.0));
    h.run();
    assert_eq!(ids(&sb.rules()), ["thanks", "caps", "pause"]);
}

/// Keyboard only (DDALGAK-NFR-007): every step of a journey works without the mouse.
#[test]
fn keyboard_only_navigation() {
    let sb = Sandbox::new("keys");
    sb.write_rules(&three_rules());
    let mut h = harness(app(&sb, sb.fake()));
    settle(&mut h);
    // Down selects the first rule, Down again the second.
    h.key_press(Key::ArrowDown);
    h.run();
    assert_eq!(h.state().selected(), ["caps"]);
    h.key_press(Key::ArrowDown);
    h.run();
    assert_eq!(h.state().selected(), ["pause"]);
    // Shift+Down extends, Ctrl+A selects all shown, Space toggles the focused one.
    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowDown);
    h.run();
    assert_eq!(h.state().selected(), ["pause", "thanks"]);
    h.key_press_modifiers(Modifiers::CTRL, Key::A);
    h.run();
    assert_eq!(h.state().selected().len(), 3);
    h.key_press(Key::Space);
    h.run();
    assert_eq!(h.state().selected().len(), 2);
    h.key_press(Key::ArrowUp);
    h.run();
    // Enter edits, Esc closes an unchanged form.
    h.key_press(Key::Enter);
    h.run();
    assert!(h.state().form().is_some());
    h.key_press(Key::Escape);
    h.run();
    assert!(h.state().form().is_none());
    // Delete asks; Esc cancels the dialog.
    h.key_press(Key::Delete);
    h.run();
    assert!(h.state().dialog_open());
    h.key_press(Key::Escape);
    h.run();
    assert!(!h.state().dialog_open());
    assert_eq!(ids(&sb.rules()).len(), 3);
    // Ctrl+D duplicates, Ctrl+Z undoes it.
    h.key_press_modifiers(Modifiers::CTRL, Key::D);
    h.run();
    assert_eq!(ids(&sb.rules()).len(), 4);
    h.key_press_modifiers(Modifiers::CTRL, Key::Z);
    h.run();
    assert_eq!(ids(&sb.rules()).len(), 3);
    // F5 tests the selected rule in simulate mode.
    h.key_press(Key::ArrowDown);
    h.run();
    h.key_press(Key::F5);
    h.run();
    assert!(h.state().simulation().is_some());
    // F6 cycles the regions; F1 lists the shortcuts.
    h.key_press(Key::F6);
    h.run();
    h.key_press(Key::F1);
    h.run();
    assert!(
        h.query_by_role_and_label(Role::Label, "Keyboard shortcuts")
            .is_some()
    );
    assert!(h.query_by_label("Ctrl+N").is_some());
    h.key_press(Key::Escape);
    h.run();
    // Ctrl+N: a new rule with the capture box recording; Tab leaves it, nothing is trapped.
    h.key_press_modifiers(Modifiers::CTRL, Key::N);
    h.run();
    h.key_press_modifiers(
        Modifiers::CTRL | Modifiers::ALT | Modifiers::SHIFT,
        Key::F16,
    );
    h.run();
    assert_eq!(
        h.state().form().unwrap().fields.trigger.keys,
        "Ctrl+Alt+Shift+F16"
    );
    h.key_press(Key::Tab);
    h.run();
    h.key_press(Key::F18);
    h.run();
    assert_eq!(
        h.state().form().unwrap().fields.trigger.keys,
        "Ctrl+Alt+Shift+F16",
        "after Tab the capture box records nothing"
    );
}

/// Unsaved drafts are never lost silently: leaving asks first (Keep editing focused); closing the
/// window with changes asks too.
#[test]
fn draft_safety_and_undo() {
    let sb = Sandbox::new("draft");
    sb.write_rules(&three_rules());
    let mut h = harness(app(&sb, sb.fake()));
    settle(&mut h);
    click(
        &mut h,
        "CapsLock  →  CapsLock acts as Left Ctrl; a quick tap is Escape",
    );
    input(&h, "Name").set_text_value_accesskit("Changed");
    h.run();
    // ADR 0096: the rule has the page; going back to the list asks first.
    click(&mut h, "← Rules");
    assert!(h.query_by_label("Discard your changes?").is_some());
    assert!(
        h.get_by_role_and_label(Role::Button, "Keep editing")
            .is_focused()
    );
    click(&mut h, "Keep editing");
    assert_eq!(h.state().form().unwrap().fields.title, "Changed");
    assert!(
        !h.state_mut().request_close(),
        "closing with changes asks first"
    );
    h.run();
    click(&mut h, "Keep editing");
    click(&mut h, "Cancel");
    click(&mut h, "Discard changes");
    assert!(h.state().form().is_none());
    assert_eq!(
        rule(&sb.rules(), "caps")["title"],
        "CapsLock acts as Ctrl",
        "nothing written"
    );
    assert!(h.state_mut().request_close());
}

/// A change made headlessly (the CLI's code path: the same store transaction) appears in the
/// open editor; the app is asked to reload; a form open on that rule refuses to overwrite it.
#[test]
fn a_cli_change_appears_and_is_never_overwritten() {
    let sb = Sandbox::new("cli");
    sb.write_rules(&three_rules());
    let fake = sb.fake();
    let mut opts = Options::for_test(sb.paths.clone(), fake.clone());
    opts.watch = true;
    let mut h = harness(EditorApp::new(opts));
    settle(&mut h);
    click(
        &mut h,
        "CapsLock  →  CapsLock acts as Left Ctrl; a quick tap is Escape",
    );
    input(&h, "Name").set_text_value_accesskit("Mine");
    h.run();
    let reloads = fake.with(|s| s.calls.iter().filter(|c| *c == "rules.reload").count());
    // "ddalgak rule edit caps --title …" and "rule disable pause" from another process.
    let cli = Repo::new(sb.paths.clone());
    cli.commit("rule.edit", |d| {
        ddalgak::edit::edit(d, "caps", &json!({ "title": "Renamed by the CLI" }), &[])
    })
    .unwrap();
    cli.commit("rule.disable", |d| {
        ddalgak::edit::set_enabled(d, &["pause".into()], false, &[])
    })
    .unwrap();
    // The folder watcher notices (real ReadDirectoryChanges notifications, no polling).
    let until = Instant::now() + Duration::from_secs(10);
    while h.state().revision() != 3 && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(20));
        h.run();
    }
    assert_eq!(h.state().revision(), 3, "the CLI's change appears");
    settle(&mut h);
    assert!(fake.with(|s| s.calls.iter().filter(|c| *c == "rules.reload").count()) > reloads);
    assert_eq!(h.state().save_state(), &SaveState::Active { revision: 3 });
    assert!(
        h.state()
            .rows()
            .iter()
            .any(|r| r.title == "Renamed by the CLI")
    );
    assert!(h.state().external_change());
    assert!(
        h.query_by_label("The rules changed outside the editor.")
            .is_some()
    );
    click(&mut h, "Save");
    assert!(
        matches!(h.state().banner(), Some(ddalgak_editor::app::Banner::Error { code, .. }) if code == "rule_conflict"),
        "{:?}",
        h.state().banner()
    );
    assert_eq!(rule(&sb.rules(), "caps")["title"], "Renamed by the CLI");
}

/// Korean labels (DDALGAK-FR-053, ADR 0092): names (sections, buttons, steps, action tiles) stay
/// English, explanations are Korean; the language is chosen in Settings and saved.
#[test]
fn korean_labels() {
    let sb = Sandbox::new("ko");
    sb.write_rules(&three_rules());
    let mut opts = Options::for_test(sb.paths.clone(), sb.fake());
    opts.lang = Lang::Ko;
    let mut h = harness(EditorApp::new(opts));
    settle(&mut h);
    for label in ["Rules", "Playground", "New Rule"] {
        assert!(
            h.query_all_by_label(label).count() >= 1,
            "{label} stays English"
        );
    }
    click(&mut h, "New Rule");
    assert!(
        h.query_all_by_label("이 키를 누르면").count() >= 1,
        "the explanation is Korean"
    );
    h.key_press(Key::Escape);
    h.run();
    for tile in [
        "Act as another key",
        "Type text",
        "Open a program",
        "Open a website",
        "Mouse",
        "Switch mode",
        "Automation on/off",
    ] {
        assert!(h.query_by_label(tile).is_some(), "{tile}");
    }
    assert!(h.query_by_label_contains("일어나는 일").is_some());
    click(&mut h, "Cancel");
    open_settings(&mut h, "Appearance");
    click(&mut h, "English");
    assert_eq!(h.state().prefs().lang, kim_ui::LangPref::En);
    let saved: Value =
        serde_json::from_slice(&std::fs::read(sb.paths.config.join("editor.json")).unwrap())
            .unwrap();
    assert_eq!(saved["language"], "en");
    click(&mut h, "Light");
    assert_eq!(h.state().prefs().theme, kim_ui::ThemePref::Light);
}

/// Many rules and modes: 1000 rules (a profile's limit) over 32 modes open, filter and scroll
/// with only the visible rows drawn.
#[test]
fn many_rules_and_modes_stay_usable() {
    let sb = Sandbox::new("many");
    let modes: Vec<Value> = (0..32)
        .map(|i| json!({ "id": format!("m{i}"), "title": format!("Mode {i}") }))
        .collect();
    let rules: Vec<Value> = (0..1000)
        .map(|i| {
            json!({ "id": format!("r{i}"), "title": format!("Rule {i}"), "enabled": false,
            "modes": [format!("m{}", i % 32)],
            "trigger": { "keys": format!("Ctrl+Alt+Shift+F{}", 13 + i % 8) },
            "steps": [ { "type": "engine", "op": "pause" } ] })
        })
        .collect();
    sb.write_rules(
        &json!({ "schema": "ddalgak.rules", "schema_version": 1, "hotkey_contract": 1,
        "revision": 1, "active_profile": "p", "profiles": [ { "id": "p", "title": "P",
        "default_mode": "m0", "modes": modes, "rules": rules } ] }),
    );
    let t = Instant::now();
    let mut h = harness(app(&sb, sb.fake()));
    settle(&mut h);
    let opened = t.elapsed();
    assert_eq!(h.state().rows().len(), 1000);
    let t = Instant::now();
    h.state_mut().filter_mut().query = "rule 99".into();
    h.run();
    let frame = t.elapsed();
    // Every word must match: "rule" and "99" (Rule 99, 199, …, 899 and 990…999).
    assert_eq!(h.state().visible_ids().len(), 19);
    // Debug-build budgets; the release build is several times faster.
    assert!(opened < Duration::from_secs(30), "opening took {opened:?}");
    assert!(
        frame < Duration::from_secs(5),
        "a search frame took {frame:?}"
    );
    // Bulk over the shown rules is one change.
    h.key_press_modifiers(Modifiers::CTRL, Key::A);
    h.run();
    assert_eq!(h.state().selected().len(), 19);
}

/// Export writes a portable file without consent; import adds the rules turned off, names every
/// launch target and runs nothing (DDALGAK-SEC-013).
#[test]
fn export_and_import_never_open_or_run_anything() {
    let sb = Sandbox::new("io");
    sb.write_rules(&doc(json!([
        { "id": "site", "title": "Open example", "enabled": true,
          "trigger": { "keys": "Ctrl+Alt+Shift+F17" }, "steps": [ { "type": "open_url", "url": "https://example.com/" } ],
          "consent": { "launch-programs": { "granted_at": GRANT, "via": "gui",
              "fingerprint": ddalgak::validate::launch_fingerprint(&json!({ "steps": [ { "type": "open_url", "url": "https://example.com/" } ] })) } } }
    ])));
    let fake = sb.fake();
    let mut h = harness(app(&sb, fake.clone()));
    settle(&mut h);
    // ADR 0100: Import and Export are in Settings → Advanced (the gear).
    open_settings(&mut h, "Advanced");
    click(&mut h, "Export…");
    let file = sb.root.join("shared.json");
    input(&h, "File").set_text_value_accesskit(&file.display().to_string());
    h.run();
    click_last_button(&mut h, "Export…");
    let exported: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert!(!exported.to_string().contains("consent"));
    assert_eq!(exported["profiles"][0]["rules"][0]["enabled"], false);
    click(&mut h, "Close");
    // Import into another, empty store.
    let other = Sandbox::new("io-target");
    let mut h2 = harness(app(&other, other.fake()));
    settle(&mut h2);
    open_settings(&mut h2, "Advanced");
    click(&mut h2, "Import…");
    input(&h2, "File").set_text_value_accesskit(&file.display().to_string());
    h2.run();
    click(&mut h2, "Check the file");
    assert!(
        h2.query_by_label_contains("https://example.com/").is_some(),
        "the target is named"
    );
    click(&mut h2, "Add them turned off");
    let d = other.rules();
    let r = &d["profiles"][0]["rules"][0];
    assert_eq!(r["enabled"], false);
    assert!(r.get("consent").is_none());
    settle(&mut h2);
    assert!(
        h2.query_all_by_label_contains("? needs review")
            .next()
            .is_some()
    );
    // Enabling it asks for consent naming the exact website.
    click(&mut h2, "Enabled: Open example");
    assert!(
        h2.query_by_label("Let this rule open this program or website?")
            .is_some()
    );
    assert!(h2.query_by_label("https://example.com/").is_some());
    click_last_button(&mut h2, "Cancel");
    assert_eq!(other.rules()["profiles"][0]["rules"][0]["enabled"], false);
    // No call opened anything: the fake app only saw status and reloads.
    for f in [&fake, &other.fake()] {
        assert!(f.with(|s| s.calls.iter().all(|c| c == "status" || c == "rules.reload")));
    }
}

/// "Test" (F5, the detail's button) runs the simulate mode: a launch rule names what it would
/// open and opens nothing; a mouse rule moves only the simulated pointer.
#[test]
fn test_start_uses_the_simulation() {
    let sb = Sandbox::new("test");
    sb.write_rules(&doc(json!([
        { "id": "np", "title": "Open Notepad", "enabled": false,
          "trigger": { "keys": "Ctrl+Alt+Shift+F18" }, "steps": [ { "type": "open_program", "path": "C:\\Windows\\System32\\notepad.exe" } ] },
        { "id": "mv", "title": "Pointer right", "enabled": false,
          "trigger": { "keys": "Ctrl+Alt+Shift+F19" }, "steps": [ { "type": "mouse_move", "mode": "relative", "dx": 100, "dy": 0 } ] }
    ])));
    let mut h = harness(app(&sb, sb.fake()));
    settle(&mut h);
    // ADR 0096: a click opens the rule; its page offers the Playground test.
    h.get_all_by_label_contains("Opens notepad.exe")
        .next()
        .unwrap()
        .click();
    h.run();
    click(&mut h, "Test in Playground");
    let sim = h.state().simulation().unwrap().clone();
    assert!(
        sim.lines.iter().any(|l| l.contains("nothing was opened")),
        "{sim:?}"
    );
    click(&mut h, "← Rules");
    // Ctrl+click: Notepad's rule out of the selection, the pointer rule in.
    h.get_all_by_label_contains("Ctrl+Alt+Shift+F18")
        .next()
        .unwrap()
        .click_modifiers(Modifiers::CTRL);
    h.run();
    h.get_all_by_label_contains("Ctrl+Alt+Shift+F19")
        .next()
        .unwrap()
        .click_modifiers(Modifiers::CTRL);
    h.run();
    h.key_press(Key::F5);
    h.run();
    let sim = h.state().simulation().unwrap().clone();
    let (from, to) = sim.pointer.unwrap();
    assert_eq!(to.x - from.x, 100, "{sim:?}");
    assert!(
        sim.lines
            .iter()
            .any(|l| l.contains("the real pointer did not move"))
    );
}

/// The Modes page: enable and disable a mode, add one; the default mode cannot be disabled.
#[test]
fn modes_page() {
    let sb = Sandbox::new("modes");
    sb.write_rules(&three_rules());
    let mut h = harness(app(&sb, sb.fake()));
    settle(&mut h);
    // ADR 0100: Settings → Modes.
    open_settings(&mut h, "Modes");
    click(&mut h, "Enabled: Navigation");
    let d = sb.rules();
    assert_eq!(d["profiles"][0]["modes"][1]["enabled"], false);
    click(&mut h, "Enabled: Navigation");
    input(&h, "New mode name").set_text_value_accesskit("Symbols");
    h.run();
    click(&mut h, "+ New mode");
    let d = sb.rules();
    assert!(
        d["profiles"][0]["modes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["title"] == "Symbols")
    );
}

/// Step 75 (J6, `DDALGAK-FR-030…032`, `DDALGAK-SEC-013`): two complete profiles for the browser.
fn two_profiles() -> Value {
    let site = json!({ "steps": [ { "type": "open_url", "url": "https://example.com/" } ] });
    json!({
        "schema": "ddalgak.rules", "schema_version": 1, "hotkey_contract": 1, "revision": 1,
        "active_profile": "work",
        "profiles": [
            { "id": "work", "title": "Work", "description": "Office keyboard", "default_mode": "base",
              "modes": [ { "id": "base", "title": "Base" }, { "id": "nav", "title": "Navigation" } ],
              "rules": [
                { "id": "caps", "title": "CapsLock acts as Ctrl", "enabled": true,
                  "trigger": { "keys": "CapsLock" }, "steps": [ { "type": "remap", "to": "LCtrl" } ],
                  "consent": { "input-injection": { "granted_at": GRANT, "via": "gui" } } },
                { "id": "site", "title": "Open example", "enabled": true,
                  "trigger": { "keys": "Ctrl+Alt+Shift+F17" }, "steps": [ { "type": "open_url", "url": "https://example.com/" } ],
                  "consent": { "launch-programs": { "granted_at": GRANT, "via": "gui",
                      "fingerprint": ddalgak::validate::launch_fingerprint(&site) } } }
              ] },
            { "id": "gaming", "title": "Gaming", "default_mode": "base",
              "modes": [ { "id": "base", "title": "Base" } ],
              "rules": [
                { "id": "g-caps", "title": "CapsLock is blocked", "enabled": true,
                  "trigger": { "keys": "CapsLock" }, "steps": [ { "type": "block" } ] }
              ] }
        ]
    })
}

/// ADR 0100: a profile row's `⋯` menu in Settings → Profiles, then one of its items.
fn profile_menu(h: &mut Harness<'static, EditorApp>, title: &str, item: &str) {
    h.get_by_label(&format!("More actions: {title}")).click();
    h.run();
    click_last_button_or_item(h, item);
}

/// The `⋯` menu of the last profile row, then one of its items.
fn last_profile_menu(h: &mut Harness<'static, EditorApp>, item: &str) {
    h.get_all_by_label_contains("More actions: ")
        .last()
        .expect("a profile row")
        .click();
    h.run();
    click_last_button_or_item(h, item);
}

fn profile_ids(d: &Value) -> Vec<String> {
    d["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap().to_string())
        .collect()
}

/// #77 (seen in the real editor): the header showed the engine's state and mode from the start
/// only; a mode key, a shortcut or the CLI changed them without the header following.
#[test]
fn the_header_follows_engine_changes_made_elsewhere() {
    let sb = Sandbox::new("live-header");
    sb.write_rules(&three_rules());
    let fake = sb.fake();
    let mut h = harness(app(&sb, fake.clone()));
    settle(&mut h);
    assert!(h.query_by_label("Mode: base").is_some());
    fake.with(|s| {
        s.engine = "paused".into();
        s.mode = Some("nav".into());
    });
    std::thread::sleep(Duration::from_millis(1100));
    settle(&mut h);
    assert!(
        h.query_by_label("Mode: nav").is_some(),
        "the new mode is shown"
    );
    // ADR 0100: the rail's status says Paused (the controls are in Settings).
    assert!(
        matches!(h.state().engine_view(), EngineView::Running { state, .. } if state == "paused"),
        "the new state is shown"
    );
    assert!(h.query_by_label("Paused").is_some());
}

/// #77 (seen in the real editor): a mode without notes drew its rule count over its name.
#[test]
fn a_mode_row_does_not_draw_its_count_over_its_name() {
    let sb = Sandbox::new("mode-row");
    sb.write_rules(&three_rules());
    let mut h = harness(app(&sb, sb.fake()));
    settle(&mut h);
    open_settings(&mut h, "Modes");
    // ADR 0100: the rows are the design system's settings rows: the name, then the count on
    // its own line below it (measured from what was really laid out).
    kim_ui::inspect::force(true);
    h.run();
    let comps = kim_ui::inspect::take();
    kim_ui::inspect::force(false);
    let row = comps
        .iter()
        .find(|c| c.id == "row:Navigation")
        .expect("the Navigation row");
    let name = row.text.as_ref().expect("its name");
    assert!(!name.truncated, "{row:?}");
    assert!(
        row.bounds.h >= 2.0 * name.bounds.h,
        "the count is a line of its own under the name: {row:?}"
    );
}

/// #79 (seen in the real editor): the "profile per program" rows were built once, so a new
/// profile had no row and a deleted one kept its row until the editor restarted.
#[test]
fn the_profile_per_program_rows_follow_added_and_deleted_profiles() {
    let sb = Sandbox::new("by-app-rows");
    sb.write_rules(&two_profiles());
    let mut h = harness(app(&sb, sb.fake()));
    settle(&mut h);
    open_settings(&mut h, "Profiles");
    assert!(h.query_by_label("gaming").is_some());
    input(&h, "New profile name").set_text_value_accesskit("Travel");
    h.run();
    click(&mut h, "+ New profile");
    settle(&mut h);
    assert!(profile_ids(&sb.rules()).contains(&"travel".to_string()));
    assert!(
        h.query_by_label("travel").is_some(),
        "the new profile has its row"
    );
    // Delete Gaming: its row goes.
    profile_menu(&mut h, "Gaming", "Delete profile");
    click_last_button(&mut h, "Delete profile");
    settle(&mut h);
    assert!(!profile_ids(&sb.rules()).contains(&"gaming".to_string()));
    assert!(
        h.query_by_label("gaming").is_none(),
        "the deleted profile's row is gone"
    );
}

/// J6: the profile browser (search, sort), a switch only after its side-by-side diff (Cancel
/// first), duplicate, rename, delete with undo, history with a restore, and an export imported
/// into another store through the validation preview: never active, rules off, no consent.
#[test]
fn j6_profiles_browse_switch_with_a_diff_history_export_and_import() {
    let sb = Sandbox::new("profiles");
    sb.write_rules(&two_profiles());
    let fake = sb.fake();
    let mut h = harness(app(&sb, fake.clone()));
    settle(&mut h);
    open_settings(&mut h, "Profiles");
    assert_eq!(
        h.state().settings_section(),
        Some(ddalgak_editor::app::SettingsSection::Profiles)
    );
    assert_eq!(h.state().profile_rows().len(), 2);
    assert_eq!(
        h.query_all_by_label("Switch…").count(),
        1,
        "only the other profile can be switched to"
    );
    // Search and sort.
    input(&h, "Search profiles").set_text_value_accesskit("gam");
    h.run();
    let shown: Vec<String> = h.state().profile_rows().into_iter().map(|r| r.id).collect();
    assert_eq!(shown, ["gaming"]);
    input(&h, "Search profiles").set_text_value_accesskit("");
    h.run();
    // Switch: the diff first, Cancel changes nothing.
    click(&mut h, "Switch…");
    assert!(h.query_by_label("Now: Work").is_some());
    assert!(h.query_by_label("After: Gaming").is_some());
    assert!(
        h.query_all_by_label_contains("CapsLock is blocked")
            .next()
            .is_some(),
        "the right side names what the key does after the switch"
    );
    // ADR 0100: the rule list stays behind the Settings window (it names hosts only); the diff
    // never shows a launch target.
    assert!(
        h.query_all_by_label_contains("https://example.com")
            .next()
            .is_none(),
        "the diff shows no launch target"
    );
    click_last_button(&mut h, "Cancel");
    assert_eq!(sb.rules()["active_profile"], "work");
    click(&mut h, "Switch…");
    click(&mut h, "Switch to this profile");
    assert_eq!(sb.rules()["active_profile"], "gaming");
    settle(&mut h);
    assert!(
        h.query_all_by_label_contains("Switched; the running app confirmed it")
            .next()
            .is_some(),
        "the switch is confirmed only after the app's reload"
    );
    assert_eq!(h.state().switching_to(), None);
    // Duplicate (rules get new IDs), rename, delete with undo.
    profile_menu(&mut h, "Work", "Duplicate");
    let d = sb.rules();
    assert_eq!(profile_ids(&d), ["work", "gaming", "work-2"]);
    assert_eq!(d["profiles"][2]["rules"][0]["id"], "caps-2");
    last_profile_menu(&mut h, "Rename…");
    input(&h, "Profile name").set_text_value_accesskit("Work at home");
    h.run();
    click_last_button(&mut h, "Save");
    assert_eq!(sb.rules()["profiles"][2]["title"], "Work at home");
    profile_menu(&mut h, "Work at home", "Delete profile");
    click_last_button(&mut h, "Delete profile");
    assert_eq!(profile_ids(&sb.rules()), ["work", "gaming"]);
    click(&mut h, "Undo");
    assert_eq!(profile_ids(&sb.rules()), ["work", "gaming", "work-2"]);
    settle(&mut h);
    // History of `work`: rename it, then restore the earlier version side by side.
    profile_menu(&mut h, "Work", "Rename…");
    input(&h, "Profile name").set_text_value_accesskit("Office");
    h.run();
    click_last_button(&mut h, "Save");
    assert_eq!(sb.rules()["profiles"][0]["title"], "Office");
    profile_menu(&mut h, "Office", "History…");
    let older = h
        .query_all_by_label_contains("profile.apply")
        .next()
        .expect("a version from before the rename");
    older.click();
    h.run();
    assert!(
        h.query_by_label("Now").is_some(),
        "the side-by-side comparison"
    );
    click(&mut h, "Restore this version");
    assert_eq!(
        sb.rules()["profiles"][0]["title"],
        "Work",
        "restored as a new change"
    );
    assert_eq!(
        sb.rules()["active_profile"],
        "gaming",
        "a restore never switches"
    );
    settle(&mut h);
    // A save point from the history.
    profile_menu(&mut h, "Work", "History…");
    input(&h, "Label for a saved version").set_text_value_accesskit("before games");
    h.run();
    click(&mut h, "Save this version");
    assert!(
        h.query_all_by_label_contains("“before games”")
            .next()
            .is_some()
    );
    click_last_button(&mut h, "Close");
    // Export `work` and import it into another, empty store.
    profile_menu(&mut h, "Work", "Export…");
    let file = sb.root.join("work.json");
    input(&h, "File").set_text_value_accesskit(&file.display().to_string());
    h.run();
    click_last_button(&mut h, "Export…");
    let exported: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert!(!exported.to_string().contains("consent"));
    assert_eq!(exported["profiles"][0]["rules"][0]["enabled"], false);
    click_last_button(&mut h, "Close");
    let other = Sandbox::new("profiles-target");
    let other_fake = other.fake();
    let mut h2 = harness(app(&other, other_fake.clone()));
    settle(&mut h2);
    open_settings(&mut h2, "Profiles");
    click(&mut h2, "Import profile…");
    input(&h2, "File").set_text_value_accesskit(&file.display().to_string());
    h2.run();
    click(&mut h2, "Check the file");
    assert!(
        h2.query_all_by_label_contains("https://example.com/")
            .next()
            .is_some(),
        "the preview names the exact target"
    );
    assert!(
        h2.query_all_by_label_contains("needs your consent")
            .next()
            .is_some()
    );
    click(&mut h2, "Add them, not active");
    let d = other.rules();
    assert_eq!(profile_ids(&d), ["default", "work"]);
    assert_eq!(
        d["active_profile"], "default",
        "an import is never switched to"
    );
    for r in d["profiles"][1]["rules"].as_array().unwrap() {
        assert_eq!(r["enabled"], false);
        assert!(r.get("consent").is_none());
    }
    // A corrupt file is refused in the preview with the reason.
    let bad = other.root.join("bad.json");
    std::fs::write(&bad, b"{ \"schema\": \"ddalgak.rules\", ").unwrap();
    click(&mut h2, "Import profile…");
    input(&h2, "File").set_text_value_accesskit(&bad.display().to_string());
    h2.run();
    click(&mut h2, "Check the file");
    assert!(
        h2.query_all_by_label_contains("cannot be imported")
            .next()
            .is_some()
    );
    assert!(h2.query_by_label("Add them, not active").is_none());
    // Nothing was opened or run: the apps only saw status requests and reloads.
    for f in [&fake, &other_fake] {
        assert!(f.with(|s| s.calls.iter().all(|c| c == "status" || c == "rules.reload")));
    }
}

/// The active profile cannot be deleted from the window either, and the Korean labels exist.
#[test]
fn the_active_profile_is_never_deleted_and_the_page_speaks_korean() {
    let sb = Sandbox::new("profiles-ko");
    sb.write_rules(&two_profiles());
    let mut h = harness(app(&sb, sb.fake()));
    settle(&mut h);
    open_settings(&mut h, "Profiles");
    h.get_by_label("More actions: Work").click();
    h.run();
    let item = h
        .get_all_by_label("Delete profile")
        .find(|n| n.accesskit_node().role() == Role::MenuItem)
        .expect("the menu's Delete profile");
    assert!(
        item.accesskit_node().is_disabled(),
        "the active profile's Delete is disabled"
    );
    h.key_press(Key::Escape);
    h.run();
    // ADR 0096/0100: the language is chosen in Settings → Appearance; names stay English.
    open_settings(&mut h, "Appearance");
    click(&mut h, "한국어");
    open_settings(&mut h, "Profiles");
    assert!(
        h.query_all_by_label_contains("완전한 묶음")
            .next()
            .is_some()
            || h.query_all_by_label_contains("프로필").next().is_some(),
        "the explanations are Korean"
    );
    assert!(h.query_by_label("Import profile…").is_some());
}

/// Step 78 (DDALGAK-FR-018, DDALGAK-FR-074, J4 *Several steps*): a macro built in the form — one step from the
/// effect form, a wait, and keys recorded in this window only — reordered and trimmed by
/// keyboard-reachable buttons, tested, and saved with its repeat count. Recording takes the keys
/// away from the editor's fields and shortcuts and stops with Esc.
#[test]
fn a_macro_is_built_recorded_reordered_and_saved() {
    let sb = Sandbox::new("macro");
    let fake = sb.fake();
    let mut h = harness(app(&sb, fake));
    settle(&mut h);
    click(&mut h, "New Rule");
    h.key_press_modifiers(Modifiers::CTRL | Modifiers::SHIFT, Key::F2);
    h.run();
    h.key_press(Key::Escape);
    h.run();
    click(&mut h, "Type text");
    input(&h, "Text").set_text_value_accesskit("안녕");
    h.run();
    // ADR 0096/0100: several steps are folded under Advanced.
    click(&mut h, "Advanced · Several steps (a macro)");
    click(&mut h, "Add to macro");
    click(&mut h, "Add a wait");
    click(&mut h, "● Record keys");
    assert!(h.state().recording());
    assert!(h.query_by_label_contains("● Recording").is_some());
    h.event(egui::Event::Text("hi".into()));
    h.run();
    h.key_press(Key::Enter);
    h.run();
    h.key_press(Key::Escape); // stops; not recorded
    h.run();
    assert!(!h.state().recording());
    // Pauses between recorded keys are kept as waits (the harness clock moves between frames):
    // remove every wait, from the last, through the rows' buttons.
    let steps = h.state().form().unwrap().fields.macro_steps.clone();
    let kept: Vec<&Value> = steps.iter().filter(|s| s["type"] != "delay").collect();
    assert_eq!(
        kept,
        [
            &json!({ "type": "type_text", "text": "안녕" }),
            &json!({ "type": "type_text", "text": "hi" }),
            &json!({ "type": "send_keys", "keys": "Enter" })
        ],
        "{steps:?}"
    );
    for i in (0..steps.len()).rev() {
        if steps[i]["type"] == "delay" {
            click(&mut h, &format!("Remove step {}", i + 1));
        }
    }
    // Reorder: Enter before "hi".
    click(&mut h, "Move up step 3");
    let steps = h.state().form().unwrap().fields.macro_steps.clone();
    let types: Vec<&str> = steps.iter().map(|s| s["type"].as_str().unwrap()).collect();
    assert_eq!(types, ["type_text", "send_keys", "type_text"]);
    // The repeat count's spin box (a DragValue does not take accessibility text input).
    assert!(
        h.query_by_role_and_label(Role::SpinButton, "Repeat the steps (times)")
            .is_some()
    );
    h.state_mut().form_mut().unwrap().fields.times = 2;
    h.run();
    assert!(
        h.query_by_label_contains("Runs 3 steps in order, 2 times")
            .is_some()
    );
    click(&mut h, "Simulate");
    assert!(h.state().simulation().expect("simulated").fires);
    assert!(!sb.paths.rules().exists(), "nothing written before saving");
    click(&mut h, "Save and enable");
    click_last_button(&mut h, "Allow");
    let saved = sb.rules();
    let r = &saved["profiles"][0]["rules"][0];
    assert_eq!(
        r["steps"],
        json!([
            { "type": "type_text", "text": "안녕" },
            { "type": "send_keys", "keys": "Enter" },
            { "type": "type_text", "text": "hi" }
        ])
    );
    assert_eq!(r["times"], 2);
    // The engine accepts what the editor saved.
    let l = ddalgak::rules::load(&sb.paths.rules());
    assert!(l.inactive.is_empty(), "{:?}", l.inactive);
}

/// Step 80 (J7, ADR 0081): an AutoHotkey v2 script imported through the same preview — the
/// report counts its rules and lists the lines that need manual migration; the rules arrive in a
/// new profile, never active, every rule off and without consent; a script with nothing
/// convertible is refused with its lines. Nothing is run.
#[test]
fn an_autohotkey_script_is_imported_through_the_preview_without_running() {
    let sb = Sandbox::new("ahk");
    sb.write_rules(&three_rules());
    let fake = sb.fake();
    let mut h = harness(app(&sb, fake));
    settle(&mut h);
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/contracts/examples/ddalgak-ahk-v1");
    open_settings(&mut h, "Profiles");
    click(&mut h, "Import profile…");
    input(&h, "File")
        .set_text_value_accesskit(&fixtures.join("02-manual.ahk").display().to_string());
    h.run();
    click(&mut h, "Check the file");
    assert!(
        h.query_all_by_label_contains("Nothing in the script converts")
            .next()
            .is_some()
    );
    assert!(
        h.query_all_by_label_contains("Manual, line 9:1")
            .next()
            .is_some(),
        "line and column"
    );
    assert!(h.query_by_label("Add them, not active").is_none());
    input(&h, "File")
        .set_text_value_accesskit(&fixtures.join("01-convertible.ahk").display().to_string());
    h.run();
    click(&mut h, "Check the file");
    assert!(h.query_all_by_label_contains("Rules: 18").next().is_some());
    assert!(
        h.query_all_by_label_contains("read, never run")
            .next()
            .is_some()
    );
    click(&mut h, "Add them, not active");
    settle(&mut h);
    let d = sb.rules();
    assert_eq!(
        d["active_profile"], "work",
        "an import is never switched to"
    );
    let profiles = d["profiles"].as_array().unwrap();
    let p = profiles
        .iter()
        .find(|p| p["title"] == "Imported from 01-convertible.ahk")
        .unwrap_or_else(|| panic!("imported: {:#}", d["profiles"]));
    assert_eq!(p["title"], "Imported from 01-convertible.ahk");
    let rules = p["rules"].as_array().unwrap();
    assert_eq!(rules.len(), 18);
    for r in rules {
        assert_eq!(r["enabled"], false);
        assert!(r.get("consent").is_none());
    }
}

/// ADR 0096: the rail's switch is the engine's real state, and turning it off or on sends the
/// request to the app; the rail has no top bar above it.
#[test]
fn the_engine_switch_follows_and_drives_the_real_state() {
    let sb = Sandbox::new("switch");
    sb.write_rules(&three_rules());
    let fake = sb.fake();
    let mut h = harness(app(&sb, fake.clone()));
    settle(&mut h);
    let on = |h: &Harness<'static, EditorApp>| matches!(h.state().engine_view(), EngineView::Running { state, .. } if state != "off");
    assert!(on(&h), "the fake app runs");
    // ADR 0100: the engine switch is in Settings → General.
    open_settings(&mut h, "General");
    h.get_all_by_role_and_label(Role::Switch, "Turn on Ddalgak")
        .next()
        .unwrap()
        .click();
    settle(&mut h);
    std::thread::sleep(Duration::from_millis(1100));
    settle(&mut h);
    assert_eq!(fake.with(|s| s.engine.clone()), "off");
    assert!(!on(&h), "the switch shows the engine turned off");
    h.get_all_by_role_and_label(Role::Switch, "Turn on Ddalgak")
        .next()
        .unwrap()
        .click();
    settle(&mut h);
    std::thread::sleep(Duration::from_millis(1100));
    settle(&mut h);
    assert_eq!(fake.with(|s| s.engine.clone()), "running");
    assert!(on(&h));
}

/// ADR 0096: "Hold + press" saves a hold layer (a mode the held key opens), the rule that holds
/// it and the rule limited to it, in one change; the list shows `CapsLock + J`; opening the rule
/// again shows it as Hold + press; keys can be picked (side-specific modifiers kept apart).
#[test]
fn hold_and_press_is_saved_as_a_layer_and_keys_can_be_picked() {
    let sb = Sandbox::new("hold");
    sb.write_rules(&three_rules());
    let mut h = harness(app(&sb, sb.fake()));
    settle(&mut h);
    click(&mut h, "New Rule");
    click(&mut h, "Hold + press");
    // The held key, picked: Tab (CapsLock already has a rule here; a held key must be free).
    click(&mut h, "Pick keys by clicking: Hold");
    click_last_button(&mut h, "Tab");
    click(&mut h, "Confirm");
    // The pressed key, picked too: RCtrl+J (a side-specific modifier stays RCtrl).
    click(&mut h, "Pick keys by clicking: Press");
    // ADR 0100: RCtrl is in the Common keys, a modifier (it combines with the key).
    click_last_button(&mut h, "RCtrl");
    h.get_by_label("Search keys").click();
    h.run();
    h.get_by_label("Search keys").type_text("J");
    h.run();
    click_last_button(&mut h, "J");
    click(&mut h, "Confirm");
    {
        let t = &h.state().form().unwrap().fields.trigger;
        assert_eq!(t.hold_key, "Tab");
        assert_eq!(t.keys, "RCtrl+J");
    }
    click(&mut h, "Act as another key");
    h.state_mut().form_mut().unwrap().fields.remap.to = "Down".into();
    h.run();
    click(&mut h, "Save and enable");
    click_last_button(&mut h, "Allow");
    let d = sb.rules();
    let p = &d["profiles"][0];
    assert!(
        p["modes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["id"] == "hold-tab" && m["title"] == "Hold Tab"),
        "{p:#}"
    );
    let rules = p["rules"].as_array().unwrap();
    assert!(rules.iter().any(|r| r["trigger"]["keys"] == "Tab"
        && r["steps"][0] == json!({ "type": "mode", "op": "hold", "mode": "hold-tab" })));
    let mine = rules
        .iter()
        .find(|r| r["trigger"]["keys"] == "RCtrl+J")
        .unwrap();
    assert_eq!(mine["modes"], json!(["hold-tab"]));
    assert_eq!(mine["steps"][0]["to"], "Down");
    settle(&mut h);
    // The list names the combination; opening it again shows Hold + press.
    let id = mine["id"].as_str().unwrap().to_string();
    let label = h
        .state()
        .rows()
        .iter()
        .find(|r| r.id == id)
        .map(|r| format!("{}  →  {}", r.trigger, r.summary))
        .unwrap();
    h.get_by_label(&label).click();
    h.run();
    let t = &h.state().form().unwrap().fields.trigger;
    assert_eq!(t.combo, ddalgak_editor::form::Combo::Hold);
    assert_eq!(t.hold_key, "Tab");
    assert!(
        !h.state().has_unsaved_changes(),
        "reading it back is no change"
    );
}

/// ADR 0100: the gear opens Settings as a window of its own (embedded in the harness); the gear
/// again only brings it forward; Esc closes it; the main window keeps its open rule meanwhile.
#[test]
fn settings_is_a_separate_window_that_never_replaces_the_main_one() {
    let sb = Sandbox::new("settings-window");
    sb.write_rules(&three_rules());
    let mut h = harness(app(&sb, sb.fake()));
    settle(&mut h);
    click(&mut h, "New Rule");
    h.key_press(Key::Escape); // ends the key capture
    h.run();
    assert!(h.state().form().is_some());
    open_settings(&mut h, "Profiles");
    assert_eq!(
        h.state().settings_section(),
        Some(ddalgak_editor::app::SettingsSection::Profiles)
    );
    assert!(h.state().form().is_some(), "the rule being made stays open");
    // The gear again: the same window, the same section (focused, not a second window).
    h.get_all_by_role_and_label(Role::Button, "Settings")
        .next()
        .unwrap()
        .click();
    h.run();
    assert_eq!(
        h.state().settings_section(),
        Some(ddalgak_editor::app::SettingsSection::Profiles)
    );
    assert_eq!(h.query_all_by_label("Import profile…").count(), 1);
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().settings_section(), None, "Esc closes Settings");
    assert!(
        h.state().form().is_some(),
        "closing Settings closes nothing else"
    );
    assert!(
        h.query_all_by_role_and_label(Role::Switch, "Turn on Ddalgak")
            .next()
            .is_none()
    );
}

/// ADR 0100: on/off and Pause are separate switches in Settings → General; each shows the
/// engine's answer, not the click: while the app has not answered it stays as it was, disabled,
/// and says it is waiting. Without the app both are disabled and say why.
#[test]
fn engine_and_pause_switches_wait_for_the_engine() {
    let sb = Sandbox::new("engine-switches");
    sb.write_rules(&three_rules());
    let fake = sb.fake();
    let mut h = harness(app(&sb, fake.clone()));
    settle(&mut h);
    open_settings(&mut h, "General");
    fake.with(|s| s.delay_ms = 600);
    click(&mut h, "Pause automation");
    let pause = h.get_by_role_and_label(Role::Switch, "Pause automation");
    assert!(
        pause.accesskit_node().is_disabled(),
        "disabled while waiting"
    );
    assert!(h.state().engine_waiting(), "waiting for the app's answer");
    assert!(
        h.query_by_label("Changing…").is_some(),
        "the rail says so too"
    );
    assert!(
        matches!(h.state().engine_view(), EngineView::Running { state, .. } if state == "running"),
        "not shown paused before the answer"
    );
    settle(&mut h);
    std::thread::sleep(Duration::from_millis(700));
    settle(&mut h);
    assert_eq!(fake.with(|s| s.engine.clone()), "paused");
    assert!(
        matches!(h.state().engine_view(), EngineView::Running { state, .. } if state == "paused")
    );
    assert!(h.query_by_label("Paused").is_some(), "the rail follows");
    // Resume is the same switch; turning Ddalgak off is the other one.
    fake.with(|s| s.delay_ms = 0);
    click(&mut h, "Pause automation");
    settle(&mut h);
    assert_eq!(fake.with(|s| s.engine.clone()), "running");
    let calls = fake.with(|s| s.calls.clone());
    assert!(calls.iter().any(|c| c == "engine.pause"));
    assert!(calls.iter().any(|c| c == "engine.resume"));
    assert!(
        !calls.iter().any(|c| c == "engine.stop"),
        "pause never turns it off"
    );
    // Without the app: both switches are disabled and say why.
    let none = Sandbox::new("engine-switches-none");
    none.write_rules(&three_rules());
    let mut h2 = harness(app(&none, Arc::new(NoEngine)));
    settle(&mut h2);
    open_settings(&mut h2, "General");
    for name in ["Turn on Ddalgak", "Pause automation"] {
        assert!(
            h2.get_by_role_and_label(Role::Switch, name)
                .accesskit_node()
                .is_disabled(),
            "{name}"
        );
    }
    assert!(h2.query_by_label("Not running").is_some());
    assert!(
        h2.query_by_label_contains("Ctrl+Alt+Shift+Esc").is_some(),
        "the emergency chord is always shown"
    );
}

/// ADR 0100: Filters open a popover with the real filters and an active-filter count; the row
/// menu edits, duplicates, disables and deletes.
#[test]
fn the_filter_popover_and_the_row_menu() {
    let sb = Sandbox::new("toolbar");
    sb.write_rules(&three_rules());
    let mut h = harness(app(&sb, sb.fake()));
    settle(&mut h);
    assert!(
        h.query_by_label("Reset filters").is_none(),
        "closed at first"
    );
    click(&mut h, "Filters");
    assert!(h.query_by_label("Reset filters").is_some(), "the popover");
    click(&mut h, "Also search snippet text");
    assert_eq!(h.state().active_filters(), 1);
    click(&mut h, "Reset filters");
    assert_eq!(h.state().active_filters(), 0);
    h.key_press(Key::Escape);
    h.run();
    // The row menu: Disable, then Edit opens the editor.
    row_menu(&mut h, "CapsLock acts as Ctrl", "Disable");
    assert_eq!(rule(&sb.rules(), "caps")["enabled"], false);
    row_menu(&mut h, "CapsLock acts as Ctrl", "Edit");
    assert_eq!(
        h.state().form().and_then(|f| f.id.clone()).as_deref(),
        Some("caps")
    );
}

/// ADR 0100: special keys by clicking: CapsLock alone, LCtrl+Space, RCtrl+RShift+A, LAlt+Left,
/// RAlt+Tab — saved exactly so and accepted by the engine; LShift+RShift, which the engine
/// cannot use, is refused in the picker (Confirm stays off) instead of saved.
#[test]
fn the_key_picker_builds_special_and_side_specific_keys() {
    let sb = Sandbox::new("picker");
    let mut h = harness(app(&sb, sb.fake()));
    settle(&mut h);
    click(&mut h, "New Rule");
    h.key_press(Key::Escape);
    h.run();
    let pick = |h: &mut Harness<'static, EditorApp>, keys: &[&str]| {
        click(h, "Pick keys by clicking: Keys");
        click_last_button(h, "Clear");
        for k in keys {
            click_last_button(h, k);
        }
    };
    for (keys, want) in [
        (&["CapsLock"][..], "CapsLock"),
        (&["LCtrl", "Space"][..], "LCtrl+Space"),
        (&["RCtrl", "RShift", "A"][..], "RCtrl+RShift+A"),
        (&["LAlt", "Left"][..], "LAlt+Left"),
    ] {
        if keys.contains(&"A") {
            click(&mut h, "Pick keys by clicking: Keys");
            click_last_button(&mut h, "Clear");
            click_last_button(&mut h, "RCtrl");
            click_last_button(&mut h, "RShift");
            click(&mut h, "A–Z, 0–9");
            click_last_button(&mut h, "A");
        } else {
            pick(&mut h, keys);
        }
        assert!(
            h.query_by_label_contains("This combination can be used")
                .is_some(),
            "{want}"
        );
        click_last_button(&mut h, "Confirm");
        assert_eq!(h.state().form().unwrap().fields.trigger.keys, want);
        assert!(ddalgak::trigger::Trigger::parse(want).is_ok(), "{want}");
    }
    // RAlt+Tab: Windows keeps Alt+Tab (task switching) for itself, so the picker refuses it
    // with the reason instead of saving a rule that could never fire.
    pick(&mut h, &["RAlt", "Tab"]);
    assert!(
        h.get_by_role_and_label(Role::Button, "Confirm")
            .accesskit_node()
            .is_disabled()
    );
    assert!(
        h.query_by_label_contains("the chord is reserved (Task switching)")
            .is_some(),
        "the reason is shown"
    );
    click_last_button(&mut h, "Cancel");
    assert_eq!(
        h.state().form().unwrap().fields.trigger.keys,
        "LAlt+Left",
        "Cancel keeps the keys"
    );
    // Left and right Shift together: the engine cannot use it, so the picker says so.
    pick(&mut h, &["LShift", "RShift"]);
    assert!(
        h.get_by_role_and_label(Role::Button, "Confirm")
            .accesskit_node()
            .is_disabled(),
        "Confirm is off for a chord the engine refuses"
    );
    assert!(
        h.query_by_label_contains("This combination can be used")
            .is_none(),
        "no false promise"
    );
    // A key is removed on its own.
    click_last_button(&mut h, "RShift  ×");
    assert!(
        h.query_by_label_contains("This combination can be used")
            .is_some()
    );
    click_last_button(&mut h, "Confirm");
    assert_eq!(h.state().form().unwrap().fields.trigger.keys, "LShift");
}

/// ADR 0100: Start at sign-in writes only the Run value (a memory store here) and says so.
#[test]
fn start_at_sign_in_writes_only_its_run_value() {
    let sb = Sandbox::new("startup");
    sb.write_rules(&three_rules());
    let mut opts = Options::for_test(sb.paths.clone(), sb.fake());
    std::fs::write(&opts.gui_exe, b"").unwrap();
    opts.startup = Box::new(ddalgak::startup::Memory::default());
    let gui = opts.gui_exe.clone();
    let mut h = harness(EditorApp::new(opts));
    settle(&mut h);
    open_settings(&mut h, "General");
    click(&mut h, "Start at sign-in");
    assert_eq!(
        h.state().startup_command().as_deref(),
        Some(ddalgak::startup::command_for(&gui).as_str())
    );
    assert!(
        h.query_by_label_contains("will start when you sign in")
            .is_some()
    );
    click(&mut h, "Start at sign-in");
    assert_eq!(h.state().startup_command(), None);
}

/// ADR 0100: a global shortcut is set with the picker into hotkeys.json (checked by the hotkey
/// contract) and turned off again; the page says when the tray app uses it.
#[test]
fn a_global_shortcut_is_picked_saved_and_turned_off() {
    let sb = Sandbox::new("global-shortcut");
    sb.write_rules(&three_rules());
    let mut h = harness(app(&sb, sb.fake()));
    settle(&mut h);
    open_settings(&mut h, "Shortcuts");
    h.get_all_by_role_and_label(Role::Button, "Pick")
        .next()
        .unwrap()
        .click();
    h.run();
    for k in ["Ctrl", "Alt", "F1–F24", "F9"] {
        click_last_button_or_item(&mut h, k);
    }
    click_last_button(&mut h, "Confirm");
    let saved = std::fs::read_to_string(sb.paths.hotkeys()).expect("hotkeys.json written");
    assert!(
        saved.contains("engine.toggle") && saved.contains("Ctrl+Alt+F9"),
        "{saved}"
    );
    assert!(h.query_by_label_contains("Restart Ddalgak").is_some());
    assert_eq!(
        ddalgak::shortcuts::Shortcuts::load(&sb.paths, false)
            .list
            .iter()
            .find(|s| s.action == "engine.toggle")
            .map(|s| s.keys.clone())
            .as_deref(),
        Some("Ctrl+Alt+F9")
    );
    click_last_button(&mut h, "Off");
    assert!(
        ddalgak::shortcuts::Shortcuts::load(&sb.paths, false)
            .list
            .iter()
            .all(|s| s.action != "engine.toggle")
    );
}

/// ADR 0100: Settings → Modes switches the running engine's mode (and only through the app).
#[test]
fn a_mode_is_switched_from_settings() {
    let sb = Sandbox::new("mode-switch");
    sb.write_rules(&three_rules());
    let fake = sb.fake();
    let mut h = harness(app(&sb, fake.clone()));
    settle(&mut h);
    open_settings(&mut h, "Modes");
    click(&mut h, "Switch");
    settle(&mut h);
    assert!(fake.with(|s| s.calls.iter().any(|c| c == "mode.set")));
    assert_eq!(fake.with(|s| s.mode.clone()).as_deref(), Some("nav"));
    assert!(h.query_by_label_contains("Mode switched").is_some());
}

/// ADR 0100 (found on the real desktop): Settings → Modes adds the mode to the profile it shows,
/// even while a rule of another profile is open in the main window.
#[test]
fn a_mode_added_in_settings_goes_to_the_active_profile() {
    let sb = Sandbox::new("mode-in-profile");
    sb.write_rules(&two_profiles());
    let mut h = harness(app(&sb, sb.fake()));
    settle(&mut h);
    click(&mut h, "New Rule");
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().form().unwrap().profile, "work");
    open_settings(&mut h, "Profiles");
    click(&mut h, "Switch…");
    click(&mut h, "Switch to this profile");
    assert_eq!(sb.rules()["active_profile"], "gaming");
    open_settings(&mut h, "Modes");
    input(&h, "New mode name").set_text_value_accesskit("Symbols");
    h.run();
    click(&mut h, "+ New mode");
    let d = sb.rules();
    let titles = |i: usize| -> Vec<String> {
        d["profiles"][i]["modes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["title"].as_str().unwrap().to_string())
            .collect()
    };
    assert!(
        titles(1).contains(&"Symbols".to_string()),
        "{:?}",
        titles(1)
    );
    assert!(
        !titles(0).contains(&"Symbols".to_string()),
        "{:?}",
        titles(0)
    );
}

/// ADR 0100 (found on the real desktop): choosing a theme keeps the window's language; only
/// the language choice changes it.
#[test]
fn choosing_a_theme_keeps_the_language() {
    let sb = Sandbox::new("theme-lang");
    sb.write_rules(&three_rules());
    let mut opts = Options::for_test(sb.paths.clone(), sb.fake());
    opts.lang = Lang::Ko;
    let mut h = harness(EditorApp::new(opts));
    settle(&mut h);
    open_settings(&mut h, "Appearance");
    click(&mut h, "Light");
    assert_eq!(
        h.state().lang,
        Lang::Ko,
        "the theme changed, the language stayed"
    );
    assert_eq!(h.state().prefs().theme, kim_ui::ThemePref::Light);
}

/// ADR 0101: the maintainer's keyboard mappings from an empty profile, by clicking only:
/// CapsLock+W → Up (CapsLock picked as a modifier, no mode made), Right Shift+Enter → Media
/// Play/Pause, CapsLock+< → types "←" (the `<` symbol is Shift+Comma), CapsLock+Backspace →
/// automation on/off, CapsLock+/ → Settings; the sentences say it in words; Settings keeps
/// CapsLock off and lists the shortcut that works while automation is off.
#[test]
fn adr0101_capslock_right_shift_media_and_arrows_by_clicking() {
    let sb = Sandbox::new("adr0101");
    let mut h = harness(app(&sb, sb.fake()));
    settle(&mut h);
    let save = |h: &mut Harness<'static, EditorApp>| {
        click(h, "Save and enable");
        if h.query_all_by_label("Allow").next().is_some() {
            click_last_button(h, "Allow");
        }
        settle(h);
    };
    let pick = |h: &mut Harness<'static, EditorApp>, field: &str, keys: &[&str]| {
        click(h, &format!("Pick keys by clicking: {field}"));
        click_last_button(h, "Clear");
        for k in keys {
            if *k == "<" || *k == "/" {
                click_last_button_or_item(h, "Symbols");
            } else if k.len() == 1 {
                click_last_button_or_item(h, "A–Z, 0–9");
            } else {
                click_last_button_or_item(h, "Common");
            }
            click_last_button(h, k);
        }
        assert!(
            h.query_by_label_contains("This combination can be used")
                .is_some(),
            "{keys:?}"
        );
        click_last_button(h, "Confirm");
    };
    // 1. CapsLock + W → Up.
    click(&mut h, "New Rule");
    h.key_press(Key::Escape);
    h.run();
    pick(&mut h, "Keys", &["CapsLock", "W"]);
    assert_eq!(h.state().form().unwrap().fields.trigger.keys, "CapsLock+W");
    click(&mut h, "Act as another key");
    pick(&mut h, "Acts as", &["Up"]);
    assert!(
        h.query_by_label_contains("When CapsLock is held and W is pressed, send Up arrow.")
            .is_some(),
        "the rule in words"
    );
    save(&mut h);
    // 2. Right Shift + Enter → Play/Pause.
    click(&mut h, "New Rule");
    h.key_press(Key::Escape);
    h.run();
    pick(&mut h, "Keys", &["RShift", "Enter"]);
    click(&mut h, "Media control");
    assert!(
        h.query_by_label_contains("When Right Shift + Enter is pressed, toggle media playback.")
            .is_some()
    );
    save(&mut h);
    // 3. CapsLock + < → types ←.
    click(&mut h, "New Rule");
    h.key_press(Key::Escape);
    h.run();
    pick(&mut h, "Keys", &["CapsLock", "<"]);
    assert_eq!(
        h.state().form().unwrap().fields.trigger.keys,
        "CapsLock+Shift+Comma"
    );
    click(&mut h, "Type text");
    input(&h, "Text").set_text_value_accesskit("\u{2190}");
    h.run();
    assert!(
        h.query_by_label_contains("When CapsLock is held and < is pressed, type")
            .is_some()
    );
    save(&mut h);
    // 4. CapsLock + Backspace → automation on/off (no consent: nothing is injected).
    click(&mut h, "New Rule");
    h.key_press(Key::Escape);
    h.run();
    pick(&mut h, "Keys", &["CapsLock", "Backspace"]);
    click(&mut h, "Automation on/off");
    save(&mut h);
    // 5. CapsLock + / → Settings.
    click(&mut h, "New Rule");
    h.key_press(Key::Escape);
    h.run();
    pick(&mut h, "Keys", &["CapsLock", "/"]);
    click(&mut h, "Open Settings");
    save(&mut h);
    let d = sb.rules();
    let rules = d["profiles"][0]["rules"].as_array().unwrap();
    let by = |keys: &str| {
        rules
            .iter()
            .find(|r| r["trigger"]["keys"] == keys)
            .unwrap_or_else(|| panic!("no rule on {keys}: {rules:#?}"))
    };
    assert_eq!(
        by("CapsLock+W")["steps"][0],
        json!({ "type": "remap", "to": "Up" })
    );
    assert!(by("CapsLock+W").get("modes").is_none(), "no layer mode");
    assert_eq!(
        by("RShift+Enter")["steps"][0],
        json!({ "type": "send_keys", "keys": "MediaPlayPause" })
    );
    assert_eq!(by("CapsLock+Shift+Comma")["steps"][0]["text"], "\u{2190}");
    assert_eq!(
        by("CapsLock+Backspace")["steps"][0],
        json!({ "type": "engine", "op": "toggle_pause" })
    );
    assert_eq!(
        by("CapsLock+code:Slash")["steps"][0],
        json!({ "type": "engine", "op": "open_settings" })
    );
    assert_eq!(d["profiles"][0]["modes"].as_array().unwrap().len(), 1);
    assert!(rules.iter().all(|r| r["enabled"] == true));
    // Every one is active in the engine's own loader.
    let l = ddalgak::rules::parse(&serde_json::to_vec(&d).unwrap());
    assert!(l.inactive.is_empty(), "{:?}", l.inactive);
    assert_eq!(l.bindings.len(), 5);
    // Settings: CapsLock stays off; the on/off shortcut is listed.
    open_settings(&mut h, "General");
    assert!(
        h.query_by_label_contains("CapsLock + Backspace").is_some(),
        "the shortcut that works while automation is off"
    );
    h.get_all_by_role_and_label(Role::Switch, "Keep CapsLock off")
        .next()
        .expect("the CapsLock switch")
        .click();
    settle(&mut h);
    assert_eq!(sb.rules()["settings"]["capslock"], "always_off");
}
