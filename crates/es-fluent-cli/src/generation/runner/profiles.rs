//! Keeps workspace members on their workspace profile settings inside the runner.
//!
//! The runner crate is its own workspace, so the application's members become
//! ordinary path dependencies there. Mirrored `[profile.<name>.package."*"]`
//! overrides would then apply to them, for example compiling application crates
//! with dependency-level optimization. Explicit per-member overrides restore the
//! settings Cargo uses for those crates in the application's own builds, which
//! also lets compiler caches reuse those units.

use std::collections::BTreeMap;
use toml::{Table, Value};

/// How Cargo compiles a workspace member inside the runner build.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MemberUnit {
    /// Compiled for the target, possibly also for the host.
    Target,
    /// Compiled only for the host, as a proc macro, a build dependency, or a
    /// dependency of one. Explicit `build-override` settings apply to it.
    HostOnly,
}

/// Profile keys whose built-in defaults are known for the root profiles.
const DEFAULTED_KEYS: &[&str] = &[
    "opt-level",
    "debug",
    "debug-assertions",
    "overflow-checks",
    "codegen-units",
];

/// Adds `[profile.<name>.package.<member>]` overrides that undo `"*"` overrides
/// for every workspace member.
///
/// Keys a member already overrides explicitly are kept. Host-only members take
/// a key from the profile chain's `build-override` before its base settings,
/// as Cargo does for workspace members. A `"*"` key whose value cannot be
/// resolved, such as a key outside [`DEFAULTED_KEYS`] that the profile chain
/// never sets, is left to the `"*"` override.
pub(super) fn restore_member_profiles(
    profiles: &mut Table,
    members: &BTreeMap<String, MemberUnit>,
) {
    let names = profiles.keys().cloned().collect::<Vec<_>>();
    for name in names {
        let star_keys = star_override_keys(profiles, &name, 0);
        if star_keys.is_empty() {
            continue;
        }

        let resolve = |unit: MemberUnit| {
            star_keys
                .iter()
                .filter_map(|key| {
                    let host_value = match unit {
                        MemberUnit::Target => None,
                        MemberUnit::HostOnly => build_override_value(profiles, &name, key, 0),
                    };
                    host_value
                        .or_else(|| base_value(profiles, &name, key, 0))
                        .map(|value| (key.clone(), value))
                })
                .collect::<Vec<_>>()
        };
        let target_values = resolve(MemberUnit::Target);
        let host_values = resolve(MemberUnit::HostOnly);
        if target_values.is_empty() && host_values.is_empty() {
            continue;
        }

        let Some(Value::Table(profile)) = profiles.get_mut(&name) else {
            continue;
        };
        let Value::Table(packages) = profile
            .entry("package")
            .or_insert_with(|| Value::Table(Table::new()))
        else {
            continue;
        };
        for (member, unit) in members {
            let values = match unit {
                MemberUnit::Target => &target_values,
                MemberUnit::HostOnly => &host_values,
            };
            if values.is_empty() {
                continue;
            }
            let Value::Table(member_table) = packages
                .entry(member.clone())
                .or_insert_with(|| Value::Table(Table::new()))
            else {
                continue;
            };
            for (key, value) in values {
                member_table
                    .entry(key.clone())
                    .or_insert_with(|| value.clone());
            }
        }
    }
}

/// Maximum `inherits` chain length followed before giving up on a cycle.
const MAX_INHERITS_DEPTH: usize = 16;

fn parent_profile(profiles: &Table, name: &str) -> Option<String> {
    let explicit = profiles
        .get(name)
        .and_then(Value::as_table)
        .and_then(|profile| profile.get("inherits"))
        .and_then(Value::as_str);
    match (name, explicit) {
        (_, Some(parent)) => Some(parent.to_string()),
        ("test", None) => Some("dev".to_string()),
        ("bench", None) => Some("release".to_string()),
        _ => None,
    }
}

/// Collects the keys set by `package."*"` in `name` or any profile it inherits.
fn star_override_keys(profiles: &Table, name: &str, depth: usize) -> Vec<String> {
    if depth > MAX_INHERITS_DEPTH {
        return Vec::new();
    }
    let mut keys = parent_profile(profiles, name)
        .map(|parent| star_override_keys(profiles, &parent, depth + 1))
        .unwrap_or_default();
    let own = profiles
        .get(name)
        .and_then(Value::as_table)
        .and_then(|profile| profile.get("package"))
        .and_then(Value::as_table)
        .and_then(|packages| packages.get("*"))
        .and_then(Value::as_table);
    for key in own.into_iter().flat_map(Table::keys) {
        if !keys.contains(key) {
            keys.push(key.clone());
        }
    }
    keys
}

/// Resolves the value a profile uses for `key` before package overrides apply.
fn base_value(profiles: &Table, name: &str, key: &str, depth: usize) -> Option<Value> {
    if depth > MAX_INHERITS_DEPTH {
        return None;
    }
    if let Some(value) = profiles
        .get(name)
        .and_then(Value::as_table)
        .and_then(|profile| profile.get(key))
    {
        return Some(value.clone());
    }
    match parent_profile(profiles, name) {
        Some(parent) => base_value(profiles, &parent, key, depth + 1),
        None => builtin_default(name, key),
    }
}

/// Resolves an explicit `build-override` value for `key` along the profile chain.
fn build_override_value(profiles: &Table, name: &str, key: &str, depth: usize) -> Option<Value> {
    if depth > MAX_INHERITS_DEPTH {
        return None;
    }
    if let Some(value) = profiles
        .get(name)
        .and_then(Value::as_table)
        .and_then(|profile| profile.get("build-override"))
        .and_then(Value::as_table)
        .and_then(|build_override| build_override.get(key))
    {
        return Some(value.clone());
    }
    parent_profile(profiles, name)
        .and_then(|parent| build_override_value(profiles, &parent, key, depth + 1))
}

fn builtin_default(profile: &str, key: &str) -> Option<Value> {
    let development = match profile {
        "dev" => true,
        "release" => false,
        _ => return None,
    };
    if !DEFAULTED_KEYS.contains(&key) {
        return None;
    }
    Some(match key {
        "opt-level" => Value::Integer(if development { 0 } else { 3 }),
        "codegen-units" => Value::Integer(if development { 256 } else { 16 }),
        _ => Value::Boolean(development),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profiles(source: &str) -> Table {
        source.parse::<Table>().expect("profiles toml")
    }

    fn members() -> BTreeMap<String, MemberUnit> {
        BTreeMap::from([
            ("app".to_string(), MemberUnit::Target),
            ("shared".to_string(), MemberUnit::Target),
        ])
    }

    #[test]
    fn members_keep_base_settings_overridden_by_star() {
        let mut table = profiles(
            r#"
            [dev]
            opt-level = 1
            debug = "limited"

            [dev.package."*"]
            opt-level = 3
            debug = false
            "#,
        );
        restore_member_profiles(&mut table, &members());

        for member in ["app", "shared"] {
            let member = &table["dev"]["package"][member];
            assert_eq!(member["opt-level"], Value::Integer(1));
            assert_eq!(member["debug"], Value::String("limited".to_string()));
        }
        assert_eq!(table["dev"]["package"]["*"]["opt-level"], Value::Integer(3));
    }

    #[test]
    fn unset_base_keys_use_builtin_profile_defaults() {
        let mut table = profiles(
            r#"
            [dev.package."*"]
            opt-level = 3
            codegen-units = 1

            [release.package."*"]
            debug-assertions = true
            "#,
        );
        restore_member_profiles(&mut table, &members());

        let dev = &table["dev"]["package"]["app"];
        assert_eq!(dev["opt-level"], Value::Integer(0));
        assert_eq!(dev["codegen-units"], Value::Integer(256));
        assert_eq!(
            table["release"]["package"]["app"]["debug-assertions"],
            Value::Boolean(false)
        );
    }

    #[test]
    fn explicit_member_overrides_are_preserved() {
        let mut table = profiles(
            r#"
            [dev]
            opt-level = 1

            [dev.package."*"]
            opt-level = 3
            debug = false

            [dev.package.app]
            opt-level = 2
            "#,
        );
        restore_member_profiles(&mut table, &members());

        let app = &table["dev"]["package"]["app"];
        assert_eq!(app["opt-level"], Value::Integer(2));
        assert_eq!(app["debug"], Value::Boolean(true));
    }

    #[test]
    fn inheriting_profiles_restore_their_own_base_values() {
        let mut table = profiles(
            r#"
            [dev]
            opt-level = 1

            [dev.package."*"]
            opt-level = 3

            [wasm-dev]
            inherits = "dev"
            opt-level = "s"
            "#,
        );
        restore_member_profiles(&mut table, &members());

        assert_eq!(
            table["dev"]["package"]["app"]["opt-level"],
            Value::Integer(1)
        );
        assert_eq!(
            table["wasm-dev"]["package"]["app"]["opt-level"],
            Value::String("s".to_string())
        );
    }

    #[test]
    fn host_only_members_prefer_explicit_build_override_values() {
        let mut table = profiles(
            r#"
            [dev]
            opt-level = 1
            debug = "limited"

            [dev.build-override]
            opt-level = 2

            [dev.package."*"]
            opt-level = 3
            debug = false

            [wasm-dev]
            inherits = "dev"
            "#,
        );
        let members = BTreeMap::from([
            ("app".to_string(), MemberUnit::Target),
            ("derive".to_string(), MemberUnit::HostOnly),
        ]);
        restore_member_profiles(&mut table, &members);

        let app = &table["dev"]["package"]["app"];
        assert_eq!(app["opt-level"], Value::Integer(1));
        let derive = &table["dev"]["package"]["derive"];
        assert_eq!(derive["opt-level"], Value::Integer(2));
        assert_eq!(derive["debug"], Value::String("limited".to_string()));
        assert_eq!(
            table["wasm-dev"]["package"]["derive"]["opt-level"],
            Value::Integer(2),
            "build-override values are inherited"
        );
    }

    #[test]
    fn profiles_without_star_overrides_or_resolvable_keys_are_unchanged() {
        let source = r#"
            [release]
            lto = "thin"

            [custom]
            inherits = "custom"

            [custom.package."*"]
            opt-level = 3

            [dev.package."*"]
            strip = "debuginfo"
            "#;
        let mut table = profiles(source);
        restore_member_profiles(&mut table, &members());

        assert_eq!(table, profiles(source));
    }
}
