use std::borrow::Cow;
use std::path::{Path, PathBuf};

pub trait ToCow<'a, T>
where
    T: ToOwned + ?Sized,
{
    fn to_cow(self) -> Cow<'a, T>;
}

impl<'a, T> ToCow<'a, T> for Cow<'a, T>
where
    T: ToOwned + ?Sized,
{
    fn to_cow(self) -> Cow<'a, T> {
        self
    }
}

impl<'a, T> ToCow<'a, T> for &'a T
where
    T: ToOwned + ?Sized,
{
    fn to_cow(self) -> Cow<'a, T> {
        Cow::Borrowed(self)
    }
}

impl<'a, T> ToCow<'a, T> for T
where
    T: Clone,
{
    fn to_cow(self) -> Cow<'a, T> {
        Cow::Owned(self)
    }
}

impl<'a> ToCow<'a, Path> for &'a str {
    fn to_cow(self) -> Cow<'a, Path> {
        Cow::Borrowed(Path::new(self))
    }
}

impl<'a> ToCow<'a, Path> for PathBuf {
    fn to_cow(self) -> Cow<'a, Path> {
        Cow::Owned(self)
    }
}
