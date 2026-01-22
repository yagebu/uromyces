//! Macros

/// Macro to define the From trait for the enum for all the variants.
/// Assumes the inner type is the same name as the variant.
macro_rules! enum_from_inner {
    ($enum:ident, $($variant:ident),+ $(,)?) => {
        $(
            impl From<$variant> for $enum {
                fn from(e: $variant) -> Self {
                    $enum::$variant(e)
                }
            }
        )*
    };
}

/// Macro to define the From trait for the enum for all the variants borrowing.
/// Assumes the inner type is the same name as the variant.
macro_rules! enum_from_inner_borrowed {
    ($enum:ident, $($variant:ident),+ $(,)?) => {
        $(
            impl<'a> From<&'a $variant> for $enum<'a> {
                fn from(e: &'a $variant) -> Self {
                    $enum::$variant(e)
                }
            }
        )*
    };
}

/// Create an `as_variant` to restrict to one variant.
macro_rules! as_inner_method {
    ($method_name:ident,$variant:ident) => {
        /// Turn the enum into the given variant.
        pub(crate) fn $method_name(&self) -> Option<&$variant> {
            if let Self::$variant(e) = self {
                Some(e)
            } else {
                None
            }
        }
    };
}

pub(crate) use as_inner_method;
pub(crate) use enum_from_inner;
pub(crate) use enum_from_inner_borrowed;
