use std::ops::Deref;
use std::ops::DerefMut;
use windows::Win32::Foundation::RECT;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Rect(pub RECT);

impl From<RECT> for Rect {
    fn from(rect: RECT) -> Self {
        Self(rect)
    }
}

impl From<Rect> for RECT {
    fn from(rect: Rect) -> Self {
        rect.0
    }
}

impl Rect {
    pub fn is_same_size_as(&self, rhs: &Self) -> bool {
        self.0.right - self.0.left == rhs.0.right - rhs.0.left
            && self.0.bottom - self.0.top == rhs.0.bottom - rhs.0.top
    }

    pub fn is_visible(&self) -> bool {
        self.0.top >= 0 || self.0.left >= 0 || self.0.bottom >= 0 || self.0.right >= 0
    }
}

impl Deref for Rect {
    type Target = RECT;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Rect {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Rect {
    /// decrease the size of self by the padding amount.
    pub fn add_padding<T>(&mut self, padding: T)
    where
        T: Into<Option<i32>>,
    {
        if let Some(padding) = padding.into() {
            self.0.left += padding;
            self.0.top += padding;
            self.0.right -= padding;
            self.0.bottom -= padding;
        }
    }

    /// increase the size of self by the margin amount.
    pub fn add_margin(&mut self, margin: i32) {
        self.0.left -= margin;
        self.0.top -= margin;
        self.0.right += margin;
        self.0.bottom += margin;
    }

    #[must_use]
    pub fn x(&self) -> i32 {
        self.0.left
    }

    #[must_use]
    pub fn y(&self) -> i32 {
        self.0.top
    }

    #[must_use]
    pub fn width(&self) -> i32 {
        self.0.right - self.0.left
    }

    #[must_use]
    pub fn height(&self) -> i32 {
        self.0.bottom - self.0.top
    }

    #[must_use]
    pub const fn scale(&self, system_dpi: i32, rect_dpi: i32) -> Rect {
        Rect(RECT {
            left: (self.0.left * system_dpi) / rect_dpi,
            top: (self.0.top * system_dpi) / rect_dpi,
            right: (self.0.right * system_dpi) / rect_dpi,
            bottom: (self.0.bottom * system_dpi) / rect_dpi,
        })
    }
}
