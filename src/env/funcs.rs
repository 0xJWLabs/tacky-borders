use super::{XString, wtraits::*};
use crate::{user_config::UserConfig, windows_api::WindowsApi};
use std::borrow::Cow;

#[inline]
pub fn full_path<SI>(input: &SI) -> anyhow::Result<Cow<str>>
where
    SI: AsRef<str> + ?Sized,
{
    env(input).map(|r| match r {
        Cow::Borrowed(s) => tilde_with_context(s, home_dir),
        Cow::Owned(s) => {
            if !input.as_ref().starts_with('~') && s.starts_with("~") {
                // return as is
                s.into()
            } else if let Cow::Owned(s) = tilde_with_context(&s, home_dir) {
                s.into()
            } else {
                s.into()
            }
        }
    })
}

fn tilde_with_context<SI, P, HD>(input: &SI, home_dir: HD) -> Cow<str>
where
    SI: AsRef<str> + ?Sized,
    P: AsRef<str>,
    HD: FnOnce() -> Option<P>,
{
    let input_str = input.as_ref().replace('/', "\\");

    if let Some(input_after_tilde) = input_str.strip_prefix('~') {
        if input_after_tilde.is_empty() || (cfg!(windows) && input_after_tilde.starts_with('\\')) {
            let home = home_dir()
                .map(|e| e.as_ref().to_string().as_ocow()) // Convert to owned String and then to Cow<str>
                .or_else(|| std::env::var("USERPROFILE").ok().map(|s| s.as_ocow()))
                .or_else(|| {
                    let drive = std::env::var("HOMEDRIVE").ok().unwrap_or("C:".to_string());
                    let path = std::env::var("HOMEPATH").ok().or_else(|| {
                        std::env::var("USERNAME").ok().and_then(|username| {
                            WindowsApi::username()
                                .ok()
                                .or(Some(username))
                                .map(|u| format!("\\Users\\{}", u))
                        })
                    });

                    path.map(|path| format!("{}{}", drive, path).as_ocow())
                });

            home.map_or_else(
                || input.as_ocow(),
                |hd| format!("{}{}", hd.as_ref(), input_after_tilde).as_ocow(),
            )
        } else {
            input.as_ocow()
        }
    } else {
        input.as_ocow()
    }
}

fn home_dir() -> Option<XString> {
    let hd = WindowsApi::home_dir().unwrap_or_default();
    hd.try_into_string().map(|s| s.replace('/', "\\"))
}

fn user_config_dir() -> Option<XString> {
    let config = UserConfig::get_config_dir().unwrap_or_default();
    config.try_into_string().map(|s| s.replace('/', "\\"))
}

fn env<SI>(input: &SI) -> anyhow::Result<Cow<str>>
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

            let lookup = |var_name: &str| -> anyhow::Result<Option<String>> {
                match var_name.to_ascii_uppercase().as_str() {
                    "USERCONFIG" => Ok(user_config_dir()),
                    _ => std::env::var(var_name)
                        .map(Some)
                        .map_err(anyhow::Error::from),
                }
            };

            // Check if we're processing a '%' wrapped variable
            if input_str.starts_with('%') {
                match input_str[1..].find('%') {
                    Some(closing_percent_idx) => {
                        let var_name = &input_str[1..closing_percent_idx + 1];
                        match lookup(var_name) {
                            Ok(Some(var_value)) => {
                                result.push_str(var_value.as_ref());
                                input_str = &input_str[closing_percent_idx + 2..];
                                next_special_idx = find_special(input_str);
                            }
                            _ => {
                                let value = &input_str[..closing_percent_idx + 2];
                                result.push_str(value);
                                input_str = &input_str[closing_percent_idx + 2..];
                                next_special_idx = find_special(input_str);
                            }
                        }
                    }
                    None => {
                        result.push('%');
                        input_str = &input_str[1..];
                        next_special_idx = find_special(input_str);
                    }
                }
            } else {
                let mut next_chars = input_str[1..].chars_approx();
                if let Some(next_char) = next_chars.next() {
                    if is_valid_var_name_char(next_char) {
                        let mut end_idx;
                        loop {
                            end_idx = input_str.len() - next_chars.len();
                            match next_chars.next() {
                                Some(c) if is_valid_var_name_char(c) => {}
                                _ => break,
                            }
                        }

                        let var_name = &input_str[1..end_idx];
                        match lookup(var_name) {
                            Ok(Some(var_value)) => {
                                result.push_str(var_value.as_ref());
                                input_str = &input_str[end_idx..];
                                next_special_idx = find_special(input_str);
                            }
                            _ => {
                                result.push_str(&input_str[..end_idx]);
                                input_str = &input_str[end_idx..];
                                next_special_idx = find_special(input_str);
                            }
                        }
                    } else {
                        result.push('$');
                        input_str = &input_str[1..];
                        next_special_idx = find_special(input_str);
                    }
                } else {
                    result.push('$');
                    input_str = &input_str[1..];
                    next_special_idx = find_special(input_str);
                }
            }
        }

        Ok(result.as_ocow())
    } else {
        Ok(input.as_ocow())
    }
}

fn is_valid_var_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
