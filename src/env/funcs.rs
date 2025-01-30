use regex::Regex;

use crate::{user_config::UserConfig, windows_api::WindowsApi};
use std::borrow::Cow;

use super::AsRefStrExt;
use super::OStringExt;
use super::PathBufExt;
use super::WstrRefExt;
use super::XString;

#[inline]
pub fn full_path<SI>(input: &SI) -> anyhow::Result<Cow<str>>
where
    SI: AsRef<str> + ?Sized,
{
    env(input).map(|r| match r {
        Cow::Borrowed(s) => tilde(s),
        Cow::Owned(s) => {
            if !input.as_ref().starts_with('~') && s.starts_with("~") {
                s.into()
            } else if let Cow::Owned(s) = tilde(&s) {
                s.into()
            } else {
                s.into()
            }
        }
    })
}

fn home_dir() -> Option<XString> {
    WindowsApi::home_dir()
        .ok()
        .and_then(|s| s.try_into_string()) // Convert PathBuf to String
        .map(|s| s.replace('/', "\\"))
        .or_else(|| {
            std::env::var("USERPROFILE")
                .ok()
                .map(|s| s.replace('/', "\\"))
        })
        .or_else(|| {
            let drive = std::env::var("HOMEDRIVE")
                .ok()
                .unwrap_or_else(|| "C:".to_string());
            let path = std::env::var("HOMEPATH").ok().or_else(|| {
                std::env::var("USERNAME").ok().and_then(|username| {
                    WindowsApi::username()
                        .ok()
                        .or(Some(username))
                        .map(|u| format!("\\Users\\{}", u))
                })
            });

            path.map(|path| format!("{}{}", drive, path).replace('/', "\\"))
        })
}

fn user_config_dir() -> Option<XString> {
    let config = UserConfig::get_config_dir().unwrap_or_default();
    config.try_into_string().map(|s| s.replace('/', "\\"))
}

pub fn env<SI>(input: &SI) -> anyhow::Result<Cow<str>>
where
    SI: AsRef<str> + ?Sized,
{
    let base_input_str = input.as_ref();
    let input_str = base_input_str.replace('/', "\\");

    if let Some(idx) = input_str.find(['$', '%']) {
        let mut result = String::with_capacity(input_str.len());

        let mut input_str = input_str.as_str();
        let mut next_special_idx = idx;

        loop {
            result.push_str(&input_str[..next_special_idx]);

            input_str = &input_str[next_special_idx..];
            if input_str.is_empty() {
                break;
            }

            fn find_special(s: &str) -> usize {
                s.find(['$', '%']).unwrap_or(s.len())
            }

            let lookup =
                |var_name: &str, default: Option<&str>| -> anyhow::Result<Option<String>> {
                    if var_name.eq_ignore_ascii_case("USERCONFIG") {
                        return Ok(user_config_dir());
                    }
                    match std::env::var(var_name) {
                        Ok(value) => Ok(Some(value)),
                        Err(_) => Ok(default.map(String::from)),
                    }
                };

            if input_str.starts_with('%') {
                if let Some(closing_percent_idx) = input_str[1..].find('%') {
                    let var_section = &input_str[1..closing_percent_idx + 1];

                    let (var_name, default) = match var_section.split_once('=') {
                        Some((name, default)) => (name, Some(default)),
                        None => (var_section, None),
                    };

                    match lookup(var_name, default) {
                        Ok(Some(var_value)) => {
                            result.push_str(&var_value);
                            input_str = &input_str[closing_percent_idx + 2..];
                        }
                        _ => {
                            result.push_str(&input_str[..closing_percent_idx + 2]);
                            input_str = &input_str[closing_percent_idx + 2..];
                        }
                    }
                    next_special_idx = find_special(input_str);
                    continue;
                } else {
                    result.push('%');
                    input_str = &input_str[1..];
                    next_special_idx = find_special(input_str);
                    continue;
                }
            } else if input_str.starts_with("${") {
                if let Some(closing_brace_idx) = input_str.find('}') {
                    let var_section = &input_str[2..closing_brace_idx];

                    let (var_name, default) = match var_section.split_once(':') {
                        Some((name, default)) => (name, Some(default)),
                        None => (var_section, None),
                    };

                    match lookup(var_name, default) {
                        Ok(Some(var_value)) => {
                            result.push_str(&var_value);
                            input_str = &input_str[closing_brace_idx + 1..];
                        }
                        _ => {
                            result.push_str(&input_str[..closing_brace_idx + 1]);
                            input_str = &input_str[closing_brace_idx + 1..];
                        }
                    }
                    next_special_idx = find_special(input_str);
                    continue;
                } else {
                    result.push_str("${");
                    input_str = &input_str[2..];
                    next_special_idx = find_special(input_str);
                    continue;
                }
            } else if input_str.starts_with('$') {
                let end_idx = input_str[1..]
                    .find(|c: char| !is_valid_var_name_char(c))
                    .map(|pos| pos + 1)
                    .unwrap_or(input_str.len());

                let var_name = &input_str[1..end_idx];

                match lookup(var_name, None) {
                    Ok(Some(var_value)) => {
                        result.push_str(&var_value);
                        input_str = &input_str[end_idx..];
                    }
                    _ => {
                        result.push_str(&input_str[..end_idx]);
                        input_str = &input_str[end_idx..];
                    }
                }
                next_special_idx = find_special(input_str);
                continue;
            }

            result.push(input_str.chars_approx().next().unwrap_or_default());
            input_str = &input_str[1..];
            next_special_idx = find_special(input_str);
        }

        Ok(result.as_ocow())
    } else {
        Ok(input.as_ocow())
    }
}

pub fn check_env<SI: AsRef<str> + ?Sized>(path: &SI) -> anyhow::Result<Cow<'static, str>> {
    let base_input_str = path.as_ref();
    let input_str = base_input_str.replace('/', "\\");
    let re = Regex::new(r"\$(\w+)|%(\w+)%|\$\{(\w+)(?::([^}]*))?\}|%(\w+)=([^%]*)%").unwrap();

    // Your lookup closure remains unchanged
    let lookup = |var_name: &str, default: Option<&str>| -> Option<String> {
        if var_name.eq_ignore_ascii_case("USERCONFIG") {
            return user_config_dir();
        }
        match std::env::var(var_name) {
            Ok(value) => Some(value),
            Err(_) => default.map(String::from),
        }
    };

    let result = re.replace_all(&input_str, |caps: &regex::Captures<'_>| {
        if let Some(var) = caps.get(1).or(caps.get(2)) {
            // `$VAR` or `%VAR%`
            lookup(var.as_str(), None)
                .map_or_else(|| caps[0].to_string().as_ocow(), |v| v.as_ocow())
        } else if let Some(var) = caps.get(3) {
            // `${VAR:default}`
            let default = caps.get(4).map(|d| d.as_str());
            lookup(var.as_str(), default)
                .map_or_else(|| caps[0].to_string().as_ocow(), |v| v.as_ocow())
        } else if let Some(var) = caps.get(5) {
            // `%VAR=default%`
            let default = caps.get(6).map(|d| d.as_str());
            lookup(var.as_str(), default)
                .map_or_else(|| caps[0].to_string().as_ocow(), |v| v.as_ocow())
        } else {
            // Default case: no match, return original match as owned
            caps[0].to_string().as_ocow()
        }
    });

    // Convert the result into a Cow<'static, str> using as_ocow
    Ok(result.into_owned().as_ocow())
}

pub fn resolve_env_vars<SI: AsRef<str> + ?Sized>(input: &SI) -> anyhow::Result<Cow<'static, str>> {
    let base_input_str = input.as_ref();
    let input_str = base_input_str.replace('/', "\\");

    let mut result = String::with_capacity(input_str.len() * 2);
    let mut chars = input_str.char_indices().peekable();

    let lookup_var = |var_name: &str, default: Option<&str>| -> anyhow::Result<Option<String>> {
        if var_name.eq_ignore_ascii_case("USERCONFIG") {
            return Ok(user_config_dir());
        }
        match std::env::var(var_name) {
            Ok(value) => Ok(Some(value)),
            Err(_) => Ok(default.map(String::from)),
        }
    };

    while let Some((i, c)) = chars.next() {
        match c {
            '$' => {
                match chars.peek() {
                    Some((_, '{')) => {
                        // Braced syntax ${VAR:default}
                        chars.next(); // Consume '{'
                        let mut var_name = String::new();
                        let mut default = None;
                        let mut in_default = false;

                        for (_, c) in chars.by_ref() {
                            match c {
                                '}' => break,
                                ':' if !in_default => {
                                    in_default = true;
                                    default = Some(String::new());
                                }
                                _ => {
                                    if in_default {
                                        default.as_mut().unwrap().push(c);
                                    } else {
                                        var_name.push(c);
                                    }
                                }
                            }
                        }

                        match lookup_var(&var_name, default.as_deref())? {
                            Some(value) => result.push_str(&value),
                            None => {
                                result.push_str("${");
                                result.push_str(&var_name);
                                if let Some(d) = default {
                                    result.push(':');
                                    result.push_str(&d);
                                }
                                result.push('}');
                            }
                        }
                    }
                    _ => {
                        // Simple syntax $VAR
                        let mut var_name = String::new();
                        let start = i + 1;
                        let mut end = start;

                        while let Some((j, c)) = chars.peek() {
                            if c.is_alphanumeric() || *c == '_' {
                                var_name.push(*c);
                                end = *j + 1;
                                chars.next();
                            } else {
                                break;
                            }
                        }

                        match lookup_var(&var_name, None)? {
                            Some(value) => result.push_str(&value),
                            None => result.push_str(&input_str[i..end]),
                        }
                    }
                }
            }
            '%' => {
                // %VAR% or %VAR=default%
                let mut var_name = String::new();
                let mut default = None;
                let mut in_default = false;
                let mut closed = false;

                for (_, c) in chars.by_ref() {
                    match c {
                        '%' => {
                            closed = true;
                            break;
                        }
                        '=' if !in_default => {
                            in_default = true;
                            default = Some(String::new());
                        }
                        _ => {
                            if in_default {
                                default.as_mut().unwrap().push(c);
                            } else {
                                var_name.push(c);
                            }
                        }
                    }
                }

                if closed {
                    match lookup_var(&var_name, default.as_deref())? {
                        Some(value) => result.push_str(&value),
                        None => {
                            result.push('%');
                            result.push_str(&var_name);
                            if let Some(d) = default {
                                result.push('=');
                                result.push_str(&d);
                            }
                            result.push('%');
                        }
                    }
                } else {
                    result.push('%');
                    result.push_str(&var_name);
                    if let Some(d) = default {
                        result.push('=');
                        result.push_str(&d);
                    }
                }
            }
            _ => result.push(c),
        }
    }

    Ok(result.as_ocow())
}

// fn env<SI>(input: &SI) -> anyhow::Result<Cow<str>>
// where
//     SI: AsRef<str> + ?Sized,
// {
//     let base_input_str = input.as_ref();
//     let input_str = base_input_str.replace('/', "\\");
//
//     if let Some(idx) = input_str.find(['$', '%']) {
//         let mut result = String::with_capacity(input_str.len());
//
//         let mut input_str = input_str.as_str();
//         let mut next_special_idx = idx;
//
//         loop {
//             result.push_str(&input_str[..next_special_idx]);
//
//             input_str = &input_str[next_special_idx..];
//             if input_str.is_empty() {
//                 break;
//             }
//
//             fn find_special(s: &str) -> usize {
//                 s.find(['$', '%']).unwrap_or(s.len())
//             }
//
//             let lookup = |var_name: &str| -> anyhow::Result<Option<String>> {
//                 match var_name.to_ascii_uppercase().as_str() {
//                     "USERCONFIG" => Ok(user_config_dir()),
//                     _ => std::env::var(var_name)
//                         .map(Some)
//                         .map_err(anyhow::Error::from),
//                 }
//             };
//
//             // Check if we're processing a '%' wrapped variable
//             if input_str.starts_with('%') {
//                 match input_str[1..].find('%') {
//                     Some(closing_percent_idx) => {
//                         let var_name = &input_str[1..closing_percent_idx + 1];
//                         match lookup(var_name) {
//                             Ok(Some(var_value)) => {
//                                 result.push_str(var_value.as_ref());
//                                 input_str = &input_str[closing_percent_idx + 2..];
//                                 next_special_idx = find_special(input_str);
//                             }
//                             _ => {
//                                 let value = &input_str[..closing_percent_idx + 2];
//                                 result.push_str(value);
//                                 input_str = &input_str[closing_percent_idx + 2..];
//                                 next_special_idx = find_special(input_str);
//                             }
//                         }
//                     }
//                     None => {
//                         result.push('%');
//                         input_str = &input_str[1..];
//                         next_special_idx = find_special(input_str);
//                     }
//                 }
//             } else {
//                 let mut next_chars = input_str[1..].chars_approx();
//                 if let Some(next_char) = next_chars.next() {
//                     if is_valid_var_name_char(next_char) {
//                         let mut end_idx;
//                         loop {
//                             end_idx = input_str.len() - next_chars.len();
//                             match next_chars.next() {
//                                 Some(c) if is_valid_var_name_char(c) => {}
//                                 _ => break,
//                             }
//                         }
//
//                         let var_name = &input_str[1..end_idx];
//                         match lookup(var_name) {
//                             Ok(Some(var_value)) => {
//                                 result.push_str(var_value.as_ref());
//                                 input_str = &input_str[end_idx..];
//                                 next_special_idx = find_special(input_str);
//                             }
//                             _ => {
//                                 result.push_str(&input_str[..end_idx]);
//                                 input_str = &input_str[end_idx..];
//                                 next_special_idx = find_special(input_str);
//                             }
//                         }
//                     } else {
//                         result.push('$');
//                         input_str = &input_str[1..];
//                         next_special_idx = find_special(input_str);
//                     }
//                 } else {
//                     result.push('$');
//                     input_str = &input_str[1..];
//                     next_special_idx = find_special(input_str);
//                 }
//             }
//         }
//
//         Ok(result.as_ocow())
//     } else {
//         Ok(input.as_ocow())
//     }
// }

fn is_valid_var_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn tilde<P: AsRef<str> + ?Sized>(path_user_input: &P) -> Cow<str> {
    let p = path_user_input.as_ref().replace('/', "\\").as_ocow();

    let home = home_dir().map(|home| home.as_ocow());

    if p == "~" {
        return home.unwrap_or(p);
    }

    if let Some(input_after_tilde) = p.strip_prefix("~") {
        match home {
            Some(home_dir) => format!("{}{}", home_dir, input_after_tilde).as_ocow(),
            None => p, // If no home directory found, return the original path
        }
    } else {
        p
    }
}
