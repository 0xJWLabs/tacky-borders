#![allow(clippy::wrong_self_convention)]

use std::borrow::Cow;

use super::XString;

pub trait AsRefStrExt {
    /// Convert an unmodified input string back into the public output type
    ///
    /// Called when inspection of the string tells us we didn't need to make any substitutions.
    fn as_ocow(&self) -> Cow<'_, str>;
}

pub trait PathBufExt {
    /// Converts a `PathBuf` into an `String`, if possible
    ///
    /// We might not be able to represent a non-Unicode path.
    /// In that case, this function returns `None`.
    fn try_into_string(self) -> Option<XString>;
}

/// Method on the reference [`&Wstr`](Wstr)
///
/// This can't be in the main [`WstrExt`] trait because
/// the type `Chars` would have a lifetime -
/// ie it would be a GAT, which is very new in Rust and we don't want to rely on.
pub trait WstrRefExt {
    /// Iterator over characters
    type Chars: Iterator<Item = char> + CharsExt;
    fn chars_approx(self) -> Self::Chars;
}

/// Methods on the characters iterator from [`Wstr.chars_approx()`](WstrRefExt::chars_approx)
pub trait CharsExt {
    fn len(&self) -> usize;
}

pub trait OStringExt {
    /// Convert an output string we have been accumulating into the public output type
    fn as_ocow(self) -> Cow<'static, str>;
}
