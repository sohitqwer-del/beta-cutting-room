//! Original Apache-2.0 helper for dirs-sys. No upstream option-ext code.
/// Check whether an optional value equals a borrowed value.
pub trait OptionExt<T> {
    /// Return false for None, otherwise compare the contained value.
    fn contains<U: ?Sized>(&self, value: &U) -> bool
    where
        T: PartialEq<U>;
}
impl<T> OptionExt<T> for Option<T> {
    fn contains<U: ?Sized>(&self, value: &U) -> bool
    where
        T: PartialEq<U>,
    {
        self.as_ref().is_some_and(|item| item == value)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn borrowed_optional_values() {
        assert!(Some("abc").contains(&"abc"));
        assert!(!None::<&str>.contains(&"abc"));
    }
}
