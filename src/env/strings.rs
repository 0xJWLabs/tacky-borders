use super::wtraits::{AsRefStrExt, CharsExt, OStringExt, PathBufExt, WstrRefExt};
use std::str::Chars;
use std::{borrow::Cow, path::PathBuf};

pub type XString = <str as ToOwned>::Owned;

impl<S: AsRef<str> + ?Sized> AsRefStrExt for S {
    fn as_ocow(&self) -> Cow<'_, str> {
        self.as_ref().into()
    }
}

impl<'s> WstrRefExt for &'s str {
    type Chars = Chars<'s>;

    /// Must be used only for the-{}-unbracketed $varname expansion variable name termination detection
    ///
    /// The implementation for `paths.rs` is ... limited.
    fn chars_approx(self) -> Chars<'s> {
        self.chars()
    }
}

impl CharsExt for Chars<'_> {
    fn len(&self) -> usize {
        self.as_str().len()
    }
}

impl OStringExt for String {
    fn as_ocow(self) -> Cow<'static, str> {
        self.into()
    }
}

impl PathBufExt for PathBuf {
    fn try_into_string(self) -> Option<String> {
        self.into_os_string().into_string().ok()
    }
}
