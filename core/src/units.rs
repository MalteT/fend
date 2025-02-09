use std::borrow::Cow;

use crate::error::{FendError, Interrupt};
use crate::eval::evaluate_to_value;
use crate::num::Number;
use crate::value::Value;

mod builtin;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) enum PrefixRule {
    NoPrefixesAllowed,
    LongPrefixAllowed,
    LongPrefix,
    ShortPrefixAllowed,
    ShortPrefix,
}

#[derive(Debug)]
pub(crate) struct UnitDef {
    singular: &'static str,
    plural: &'static str,
    prefix_rule: PrefixRule,
    value: Value,
}

fn expr_unit<I: Interrupt>(
    singular: &'static str,
    plural: &'static str,
    definition: &'static str,
    context: &mut crate::Context,
    int: &I,
) -> Result<UnitDef, FendError> {
    let mut definition = definition.trim();
    let mut rule = PrefixRule::NoPrefixesAllowed;
    if let Some(remaining) = definition.strip_prefix("l@") {
        definition = remaining;
        rule = PrefixRule::LongPrefixAllowed;
    }
    if let Some(remaining) = definition.strip_prefix("lp@") {
        definition = remaining;
        rule = PrefixRule::LongPrefix;
    }
    if let Some(remaining) = definition.strip_prefix("s@") {
        definition = remaining;
        rule = PrefixRule::ShortPrefixAllowed;
    }
    if let Some(remaining) = definition.strip_prefix("sp@") {
        definition = remaining;
        rule = PrefixRule::ShortPrefix;
    }
    if definition == "!" {
        return Ok(UnitDef {
            value: Value::Num(Box::new(Number::new_base_unit(
                Cow::Borrowed(singular),
                Cow::Borrowed(plural),
            ))),
            prefix_rule: rule,
            singular,
            plural,
        });
    }
    let (alias, definition) = definition
        .strip_prefix('=')
        .map_or((false, definition), |remaining| (true, remaining));
    let mut num = evaluate_to_value(definition, None, context, int)?.expect_num()?;
    if !alias && rule != PrefixRule::LongPrefix {
        num = Number::create_unit_value_from_value(
            &num,
            Cow::Borrowed(""),
            Cow::Borrowed(singular),
            Cow::Borrowed(plural),
            int,
        )?;
    }
    Ok(UnitDef {
        value: Value::Num(Box::new(num)),
        prefix_rule: rule,
        singular,
        plural,
    })
}

fn construct_prefixed_unit<I: Interrupt>(
    a: UnitDef,
    b: UnitDef,
    int: &I,
) -> Result<Value, FendError> {
    let product = a.value.expect_num()?.mul(b.value.expect_num()?, int)?;
    assert_eq!(a.singular, a.plural);
    let unit = Number::create_unit_value_from_value(
        &product,
        Cow::Borrowed(a.singular),
        Cow::Borrowed(b.singular),
        Cow::Borrowed(b.plural),
        int,
    )?;
    Ok(Value::Num(Box::new(unit)))
}

pub(crate) fn query_unit<'a, I: Interrupt>(
    ident: &'a str,
    context: &mut crate::Context,
    int: &I,
) -> Result<Value, FendError> {
    if ident.starts_with('\'') && ident.ends_with('\'') && ident.len() >= 3 {
        let ident = ident.split_at(1).1;
        let ident = ident.split_at(ident.len() - 1).0;
        return Ok(Value::Num(Box::new(Number::new_base_unit(
            ident.to_string().into(),
            ident.to_string().into(),
        ))));
    }
    query_unit_static(ident, context, int)
}

pub(crate) fn query_unit_static<'a, I: Interrupt>(
    ident: &'a str,
    context: &mut crate::Context,
    int: &I,
) -> Result<Value, FendError> {
    match query_unit_case_sensitive(ident, true, context, int) {
        Err(FendError::IdentifierNotFound(_)) => (),
        Err(e) => return Err(e),
        Ok(value) => {
            return Ok(value);
        }
    }
    query_unit_case_sensitive(ident, false, context, int)
}

fn query_unit_case_sensitive<I: Interrupt>(
    ident: &str,
    case_sensitive: bool,
    context: &mut crate::Context,
    int: &I,
) -> Result<Value, FendError> {
    match query_unit_internal(ident, false, case_sensitive, context, int) {
        Err(FendError::IdentifierNotFound(_)) => (),
        Err(e) => return Err(e),
        Ok(unit) => {
            // Return value without prefix. Note that lone short prefixes
            // won't be returned here.
            return Ok(unit.value);
        }
    }
    let mut split_idx = ident.chars().next().unwrap().len_utf8();
    while split_idx < ident.len() {
        let (prefix, remaining_ident) = ident.split_at(split_idx);
        split_idx += remaining_ident.chars().next().unwrap().len_utf8();
        let a = match query_unit_internal(prefix, true, case_sensitive, context, int) {
            Err(FendError::IdentifierNotFound(_)) => continue,
            Err(e) => {
                return Err(e);
            }
            Ok(a) => a,
        };
        match query_unit_internal(remaining_ident, false, case_sensitive, context, int) {
            Err(FendError::IdentifierNotFound(_)) => continue,
            Err(e) => return Err(e),
            Ok(b) => {
                if (a.prefix_rule == PrefixRule::LongPrefix
                    && b.prefix_rule == PrefixRule::LongPrefixAllowed)
                    || (a.prefix_rule == PrefixRule::ShortPrefix
                        && b.prefix_rule == PrefixRule::ShortPrefixAllowed)
                {
                    // now construct a new unit!
                    return construct_prefixed_unit(a, b, int);
                }
                return Err(FendError::IdentifierNotFound(ident.to_string().into()));
            }
        };
    }
    Err(FendError::IdentifierNotFound(ident.to_string().into()))
}

fn query_unit_internal<'a, I: Interrupt>(
    ident: &'a str,
    short_prefixes: bool,
    case_sensitive: bool,
    context: &mut crate::Context,
    int: &I,
) -> Result<UnitDef, FendError> {
    if ident == "C" {
        return if context.fc_mode == crate::FCMode::CelsiusFahrenheit {
            expr_unit("C", "C", "=\u{b0}C", context, int)
        } else {
            expr_unit("C", "C", "s@coulomb", context, int)
        };
    } else if ident == "F" {
        return if context.fc_mode == crate::FCMode::CelsiusFahrenheit {
            expr_unit("F", "F", "=\u{b0}F", context, int)
        } else {
            expr_unit("F", "F", "s@farad", context, int)
        };
    }
    if let Some((s, p, expr)) = builtin::query_unit(ident, short_prefixes, case_sensitive) {
        expr_unit(s, p, expr, context, int)
    } else {
        Err(FendError::IdentifierNotFound(ident.to_string().into()))
    }
}

pub(crate) fn get_completions_for_prefix(prefix: &str) -> Vec<crate::Completion> {
    use crate::Completion;

    let mut result = vec![];

    let mut add = |name: &str| {
        if name.starts_with(prefix) && name != prefix {
            result.push(Completion {
                display: name.to_string(),
                insert: name.split_at(prefix.len()).1.to_string(),
            });
        }
    };

    for group in builtin::ALL_UNIT_DEFS {
        for (s, _, _, _) in *group {
            // only add singular name, since plurals
            // unnecessarily clutter autocompletions
            add(s);
        }
    }

    result.sort_by(|a, b| a.display().cmp(b.display()));

    result
}
