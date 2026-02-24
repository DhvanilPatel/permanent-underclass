use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[test]
fn generated_event_catalogs_have_no_choice_kind_field() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("data/events");
    let mut checked_events = 0_usize;

    for entry in fs::read_dir(&dir).expect("data/events must be readable") {
        let path = entry.expect("read_dir entry").path();
        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }

        let raw = fs::read_to_string(&path).expect("jsonl file must be readable");
        for (idx, line) in raw.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            checked_events += 1;
            let ev: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|err| {
                panic!(
                    "failed to parse {}:{} as json: {}",
                    path.display(),
                    idx + 1,
                    err
                )
            });
            let choices = ev
                .get("choices")
                .and_then(|v| v.as_array())
                .expect("event must include a choices array");
            for (choice_idx, choice) in choices.iter().enumerate() {
                assert!(
                    choice.get("kind").is_none(),
                    "{}:{} choice {} still contains legacy `kind` field",
                    path.display(),
                    idx + 1,
                    choice_idx + 1
                );
            }
        }
    }

    assert!(
        checked_events > 0,
        "expected at least one event in data/events"
    );
}

#[test]
fn generated_event_catalogs_include_events_without_flag_usage() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("data/events");
    let mut checked_events = 0_usize;
    let mut non_flag_events = 0_usize;

    for entry in fs::read_dir(&dir).expect("data/events must be readable") {
        let path = entry.expect("read_dir entry").path();
        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }

        let raw = fs::read_to_string(&path).expect("jsonl file must be readable");
        for (idx, line) in raw.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            checked_events += 1;
            let ev: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|err| {
                panic!(
                    "failed to parse {}:{} as json: {}",
                    path.display(),
                    idx + 1,
                    err
                )
            });
            let choices = ev
                .get("choices")
                .and_then(|v| v.as_array())
                .expect("event must include a choices array");

            let event_uses_flags =
                array_has_entries(ev.get("requires")) || array_has_entries(ev.get("forbids"));
            let choice_uses_flags = choices.iter().any(|choice| {
                array_has_entries(choice.get("requires"))
                    || array_has_entries(choice.get("forbids"))
                    || array_has_entries(choice.get("effects"))
            });

            if !event_uses_flags && !choice_uses_flags {
                non_flag_events += 1;
            }
        }
    }

    assert!(
        checked_events > 0,
        "expected at least one event in data/events"
    );
    assert!(
        non_flag_events > 0,
        "expected at least one event without flag usage across data/events"
    );
}

fn array_has_entries(value: Option<&serde_json::Value>) -> bool {
    value
        .and_then(|v| v.as_array())
        .is_some_and(|entries| !entries.is_empty())
}

#[test]
fn generated_event_catalogs_only_reference_known_flags() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let events_dir = root.join("data/events");
    let flags_path = root.join("data/flags.jsonl");
    let known_flags = load_known_flags(&flags_path);
    let mut checked_events = 0_usize;

    for entry in fs::read_dir(&events_dir).expect("data/events must be readable") {
        let path = entry.expect("read_dir entry").path();
        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }

        let raw = fs::read_to_string(&path).expect("jsonl file must be readable");
        for (idx, line) in raw.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            checked_events += 1;
            let ev: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|err| {
                panic!(
                    "failed to parse {}:{} as json: {}",
                    path.display(),
                    idx + 1,
                    err
                )
            });

            assert_known_flag_array(
                &ev,
                "requires",
                &known_flags,
                &path,
                idx + 1,
                "event requires",
            );
            assert_known_flag_array(
                &ev,
                "forbids",
                &known_flags,
                &path,
                idx + 1,
                "event forbids",
            );

            let choices = ev
                .get("choices")
                .and_then(|v| v.as_array())
                .expect("event must include a choices array");
            for (choice_idx, choice) in choices.iter().enumerate() {
                assert_known_flag_array(
                    choice,
                    "requires",
                    &known_flags,
                    &path,
                    idx + 1,
                    &format!("choice {} requires", choice_idx + 1),
                );
                assert_known_flag_array(
                    choice,
                    "forbids",
                    &known_flags,
                    &path,
                    idx + 1,
                    &format!("choice {} forbids", choice_idx + 1),
                );

                if let Some(effects) = choice.get("effects").and_then(|v| v.as_array()) {
                    for (effect_idx, effect) in effects.iter().enumerate() {
                        let flag =
                            effect
                                .get("flag")
                                .and_then(|v| v.as_str())
                                .unwrap_or_else(|| {
                                    panic!(
                                        "{}:{} choice {} effect {} missing string flag",
                                        path.display(),
                                        idx + 1,
                                        choice_idx + 1,
                                        effect_idx + 1
                                    )
                                });
                        assert!(
                            known_flags.contains(flag),
                            "{}:{} choice {} effect {} references unknown flag '{}'",
                            path.display(),
                            idx + 1,
                            choice_idx + 1,
                            effect_idx + 1,
                            flag
                        );
                    }
                }
            }
        }
    }

    assert!(
        checked_events > 0,
        "expected at least one event in data/events"
    );
}

fn load_known_flags(path: &Path) -> HashSet<String> {
    let raw = fs::read_to_string(path).expect("data/flags.jsonl must be readable");
    let mut out = HashSet::new();
    for (idx, raw_line) in raw.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let row: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|err| {
            panic!(
                "{}:{} failed to parse JSONL row: {}",
                path.display(),
                idx + 1,
                err
            )
        });
        let flag = row.get("flag").and_then(|v| v.as_str()).unwrap_or_else(|| {
            panic!(
                "{} row {} is missing a string `flag` field",
                path.display(),
                idx + 1
            )
        });
        out.insert(flag.to_string());
    }
    out
}

fn assert_known_flag_array(
    owner: &serde_json::Value,
    field: &str,
    known_flags: &HashSet<String>,
    path: &Path,
    line_no: usize,
    context: &str,
) {
    if let Some(flags) = owner.get(field).and_then(|v| v.as_array()) {
        for (flag_idx, flag_value) in flags.iter().enumerate() {
            let flag = flag_value.as_str().unwrap_or_else(|| {
                panic!(
                    "{}:{} {} flag {} is not a string",
                    path.display(),
                    line_no,
                    context,
                    flag_idx + 1
                )
            });
            assert!(
                known_flags.contains(flag),
                "{}:{} {} references unknown flag '{}'",
                path.display(),
                line_no,
                context,
                flag
            );
        }
    }
}
