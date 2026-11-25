// SPDX-License-Identifier: AGPL-3.0-or-later
//! The template environment (decision record 0019 items 2 and 3).
//!
//! Strict undefined variables, no auto-escaping, no loader (a template cannot
//! read files), debug on so errors show the line, a recursion limit, and fuel
//! set per render in proportion to the program. The numeric surface is
//! IEEE-exact only (rounding, `sqrt`, `floor`, `trunc`) so output is the same
//! on every platform (NFR-06).
//!
//! The template engine frees its values by reference counting, so a
//! `namespace` that holds itself (`{% set ns.me = ns %}`, or through a list or
//! a second namespace) would never be freed: memory leaked on every post of
//! such a template (issue #794, the nightly `post_definition` fuzz failure).
//! Each render therefore records the namespaces it makes, and
//! [`Namespaces`] empties them when the render is over, which breaks every
//! such cycle (namespaces are the only values a template can change).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use minijinja::syntax::SyntaxConfig;
use minijinja::value::ValueOrKwargs;
use minijinja::{AutoEscape, Environment, Error, ErrorKind, UndefinedBehavior, Value};

use crate::error::{PostError, Result};

/// Template recursion limit.
pub const RECURSION_LIMIT: usize = 64;
/// Fuel for a render that has no steps.
pub const FUEL_BASE: u64 = 100_000;
/// Fuel per step of the program (decision 0019 item 3 measured about 323).
pub const FUEL_PER_STEP: u64 = 400;

/// Fixed-decimal text: round half to even on the exact binary value, never `-0`.
pub fn fixed_str(v: f64, decimals: usize) -> String {
    let s = format!("{v:.decimals$}");
    match s.strip_prefix('-') {
        Some(rest) if rest.chars().all(|c| c == '0' || c == '.') => rest.to_string(),
        _ => s,
    }
}

/// The fewest significant figures a feed word is written with (decision
/// record 0076): half a unit of the last of three is at most 0.5 % of the
/// value, so rounding alone cannot take a move's time past gate 2's 1 %.
pub const FEED_SIGNIFICANT_FIGURES: usize = 3;

/// A feed as [`fixed_str`] writes it with at least `decimals` places, and
/// more (up to 9) where the value would otherwise keep fewer than
/// [`FEED_SIGNIFICANT_FIGURES`] significant figures: `4.237` at one decimal
/// is `4.24`, `1200.04` stays `1200.0`. Only multiplication by a power of
/// ten is used, so the text is the same on every platform (NFR-06).
pub fn feed_str(v: f64, decimals: usize) -> String {
    // Powers of ten by repeated multiplication, exact at these sizes: `powi`
    // is not guaranteed the same everywhere.
    let power = |n: usize| (0..n).fold(1.0_f64, |p, _| p * 10.0);
    let least = power(FEED_SIGNIFICANT_FIGURES - 1);
    let mut places = decimals.min(9);
    while places < 9 && v.abs() * power(places) < least {
        places += 1;
    }
    fixed_str(v, places)
}

fn number(v: &Value, what: &str) -> std::result::Result<f64, Error> {
    let x = f64::try_from(v.clone()).map_err(|_| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("{what}: expected a number, got {}", v.kind()),
        )
    })?;
    if x.is_finite() {
        Ok(x)
    } else {
        Err(Error::new(
            ErrorKind::InvalidOperation,
            format!("{what}: {x} is not a finite number"),
        ))
    }
}

fn decimals(d: &Value) -> std::result::Result<usize, Error> {
    let n = i64::try_from(d.clone())
        .map_err(|_| Error::new(ErrorKind::InvalidOperation, "decimals must be an integer"))?;
    usize::try_from(n).ok().filter(|n| *n <= 9).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("decimals {n} is outside 0..=9"),
        )
    })
}

/// True for a name the template syntax can write after a dot (ASCII letters,
/// digits and `_`, not starting with a digit).
fn is_name(key: &str) -> bool {
    let mut chars = key.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The namespaces one render has made. When this is dropped, after the
/// render, every key of every one is set to `none`, which frees a namespace
/// that holds itself (issue #794).
#[derive(Debug, Default)]
pub struct Namespaces(Arc<Mutex<Vec<Value>>>);

impl Namespaces {
    /// Empties every recorded namespace and forgets it.
    fn release(&self) {
        let spaces = std::mem::take(&mut *self.0.lock().unwrap_or_else(PoisonError::into_inner));
        // A small template of `set` statements is the only way to change a
        // namespace from outside the engine. Its own environment has no fuel
        // limit, and it makes no namespaces.
        let mut source = String::new();
        let mut context = BTreeMap::new();
        for (i, space) in spaces.into_iter().enumerate() {
            let name = format!("n{i}");
            for key in space.try_iter().into_iter().flatten() {
                // Every key is a name: `set` writes only names, and the
                // `namespace` function refuses any other key.
                if let Some(key) = key.as_str().filter(|k| is_name(k)) {
                    source.push_str(&format!("{{% set {name}.{key} = none %}}"));
                }
            }
            context.insert(name, space);
        }
        if !source.is_empty() {
            // It cannot fail: every name is in the context and every key is a name.
            let _ = Environment::new().render_str(&source, context);
        }
    }
}

impl Drop for Namespaces {
    fn drop(&mut self) {
        self.release();
    }
}

/// A fresh environment for one render of `steps` steps, and the record of the
/// namespaces it makes, to keep until the render is over.
pub fn environment(steps: usize) -> Result<(Environment<'static>, Namespaces)> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    env.set_auto_escape_callback(|_| AutoEscape::None);
    let syntax = SyntaxConfig::builder()
        .trim_blocks(true)
        .lstrip_blocks(true)
        .keep_trailing_newline(true)
        .build()
        .map_err(|e| PostError::Template(e.to_string()))?;
    env.set_syntax(syntax);
    env.set_debug(true);
    let fuel = FUEL_BASE.saturating_add(FUEL_PER_STEP.saturating_mul(steps as u64));
    env.set_fuel(Some(fuel));
    env.set_recursion_limit(RECURSION_LIMIT);
    env.add_filter(
        "fixed",
        |v: Value, d: Value| -> std::result::Result<String, Error> {
            Ok(fixed_str(number(&v, "fixed")?, decimals(&d)?))
        },
    );
    env.add_filter(
        "feed",
        |v: Value, d: Value| -> std::result::Result<String, Error> {
            Ok(feed_str(number(&v, "feed")?, decimals(&d)?))
        },
    );
    env.add_filter(
        "rounded",
        |v: Value, d: Value| -> std::result::Result<f64, Error> {
            let text = fixed_str(number(&v, "rounded")?, decimals(&d)?);
            text.parse::<f64>()
                .map_err(|e| Error::new(ErrorKind::InvalidOperation, e.to_string()))
        },
    );
    env.add_filter("sqrt", |v: Value| -> std::result::Result<f64, Error> {
        let x = number(&v, "sqrt")?;
        if x < 0.0 {
            return Err(Error::new(
                ErrorKind::InvalidOperation,
                "sqrt of a negative number",
            ));
        }
        Ok(x.sqrt())
    });
    env.add_filter("floor", |v: Value| -> std::result::Result<f64, Error> {
        Ok(number(&v, "floor")?.floor())
    });
    env.add_filter("trunc", |v: Value| -> std::result::Result<f64, Error> {
        Ok(number(&v, "trunc")?.trunc())
    });
    env.add_global("pi", std::f64::consts::PI);
    let namespaces = Namespaces::default();
    let made = Arc::clone(&namespaces.0);
    env.add_function(
        "namespace",
        move |defaults: Option<ValueOrKwargs>| -> std::result::Result<Value, Error> {
            let space = minijinja::functions::namespace(defaults)?;
            if let Some(key) = space.try_iter()?.find(|k| !k.as_str().is_some_and(is_name)) {
                return Err(Error::new(
                    ErrorKind::InvalidOperation,
                    format!(
                        "namespace: the key {key} is not a name (letters, digits and _, \
                         not starting with a digit)"
                    ),
                ));
            }
            made.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(space.clone());
            Ok(space)
        },
    );
    Ok((env, namespaces))
}

/// Turns an engine error into a message naming the template and line.
pub fn describe(error: &Error) -> String {
    let line = error
        .line()
        .map_or(String::new(), |l| format!(" at line {l}"));
    let name = error.name().unwrap_or("template");
    let detail = error.detail().unwrap_or("");
    format!("{name}{line}: {} {detail}", error.kind())
        .trim_end()
        .to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::sync::Weak;

    use minijinja::value::Object;

    use super::*;

    /// A value the test can see being freed.
    #[derive(Debug)]
    struct Probe;

    impl Object for Probe {}

    /// Renders `source` with `probe` in the context, and reports whether the
    /// probe is still alive once the render and the environment are gone.
    fn probe_survives(source: &str) -> (std::result::Result<String, String>, bool) {
        let probe = Arc::new(Probe);
        let weak: Weak<Probe> = Arc::downgrade(&probe);
        let result = {
            let (env, _namespaces) = environment(0).unwrap();
            let value = Value::from_dyn_object(probe);
            env.render_str(source, minijinja::context! { probe => value })
                .map_err(|e| e.to_string())
        };
        (result, weak.upgrade().is_some())
    }

    #[test]
    fn req_nfr_07_a_namespace_that_holds_itself_is_freed_after_the_render() {
        for source in [
            // The nightly fuzz input of issue #794 stored a namespace in itself.
            "{% set st = namespace(p=probe) %}{% set st.me = st %}",
            "{% set st = namespace(p=probe) %}{% set st.all = [st] %}",
            "{% set a = namespace(p=probe) %}{% set b = namespace(a=a) %}{% set a.b = b %}",
            // A render that fails after making the cycle frees it too.
            "{% set st = namespace(p=probe) %}{% set st.me = st %}{{ missing }}",
        ] {
            let (_, alive) = probe_survives(source);
            assert!(!alive, "{source}");
        }
    }

    #[test]
    fn req_nfr_07_namespaces_render_as_before() {
        let (text, alive) = probe_survives(
            "{% set st = namespace(n=1) %}{% for i in [1, 2] %}{% set st.n = st.n + i %}\
             {% endfor %}{{ st.n }} {{ namespace({'if': 2})['if'] }}",
        );
        assert_eq!(text.unwrap(), "4 2");
        assert!(!alive);
    }

    #[test]
    fn req_nfr_07_a_namespace_key_that_is_not_a_name_is_refused() {
        for source in [
            "{% set st = namespace({'a b': probe}) %}",
            "{% set st = namespace({'': probe}) %}",
            "{% set st = namespace({'9a': probe}) %}",
        ] {
            let (result, alive) = probe_survives(source);
            let err = result.unwrap_err();
            assert!(err.contains("is not a name"), "{source}: {err}");
            assert!(!alive, "{source}");
        }
    }
}
