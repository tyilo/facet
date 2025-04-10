use crate::opaque::{Opaque, OpaqueConst, OpaqueUninit};
use bitflags::bitflags;
use core::cmp::Ordering;

use crate::Shape;

//======== Type Information ========

/// A function that formats the name of a type.
///
/// This helps avoid allocations, and it takes options.
pub type TypeNameFn = fn(f: &mut core::fmt::Formatter, opts: TypeNameOpts) -> core::fmt::Result;

/// Options for formatting the name of a type
#[non_exhaustive]
#[derive(Clone, Copy)]
pub struct TypeNameOpts {
    /// as long as this is > 0, keep formatting the type parameters
    /// when it reaches 0, format type parameters as `...`
    /// if negative, all type parameters are formatted
    pub recurse_ttl: isize,
}

impl Default for TypeNameOpts {
    fn default() -> Self {
        Self { recurse_ttl: -1 }
    }
}

impl TypeNameOpts {
    /// Create a new `NameOpts` for which none of the type parameters are formatted
    pub fn none() -> Self {
        Self { recurse_ttl: 0 }
    }

    /// Create a new `NameOpts` for which only the direct children are formatted
    pub fn one() -> Self {
        Self { recurse_ttl: 1 }
    }

    /// Create a new `NameOpts` for which all type parameters are formatted
    pub fn infinite() -> Self {
        Self { recurse_ttl: -1 }
    }

    /// Decrease the `recurse_ttl` — if it's != 0, returns options to pass when
    /// formatting children type parameters.
    ///
    /// If this returns `None` and you have type parameters, you should render a
    /// `…` (unicode ellipsis) character instead of your list of types.
    ///
    /// See the implementation for `Vec` for examples.
    pub fn for_children(&self) -> Option<Self> {
        match self.recurse_ttl.cmp(&0) {
            Ordering::Greater => Some(Self {
                recurse_ttl: self.recurse_ttl - 1,
            }),
            Ordering::Less => Some(Self {
                recurse_ttl: self.recurse_ttl,
            }),
            Ordering::Equal => None,
        }
    }
}

//======== Memory Management ========

/// Function to drop a value
///
/// # Safety
///
/// The `value` parameter must point to aligned, initialized memory of the correct type.
pub type DropInPlaceFn = for<'mem> unsafe fn(value: Opaque<'mem>);

/// Generates a [`DropInPlaceFn`] for a concrete type
pub const fn drop_in_place_fn_for<T>() -> Option<DropInPlaceFn> {
    Some(|value: Opaque<'_>| unsafe {
        value.drop_in_place::<T>();
    })
}

/// Function to clone a value into another already-allocated value
///
/// # Safety
///
/// The `source` parameter must point to aligned, initialized memory of the correct type.
/// The `target` parameter has the correct layout and alignment, but points to
/// uninitialized memory. The function returns the same pointer wrapped in an [`Opaque`].
pub type CloneIntoFn = for<'src, 'dst> unsafe fn(
    source: OpaqueConst<'src>,
    target: OpaqueUninit<'dst>,
) -> Opaque<'dst>;

/// Generates a [`CloneInPlaceFn`] for a concrete type
pub const fn clone_into_fn_for<T: Clone>() -> Option<CloneIntoFn> {
    Some(|source: OpaqueConst<'_>, target: OpaqueUninit<'_>| unsafe {
        let source_val = source.as_ref::<T>();
        target.put(source_val.clone())
    })
}

/// Function to set a value to its default in-place
///
/// # Safety
///
/// The `target` parameter has the correct layout and alignment, but points to
/// uninitialized memory. The function returns the same pointer wrapped in an [`Opaque`].
pub type DefaultInPlaceFn = for<'mem> unsafe fn(target: OpaqueUninit<'mem>) -> Opaque<'mem>;

/// Generates a [`DefaultInPlaceFn`] for a concrete type
pub const fn default_in_place_fn_for<T: Default>() -> Option<DefaultInPlaceFn> {
    Some(|target: OpaqueUninit<'_>| unsafe { target.put(T::default()) })
}

//======== Conversion ========

/// Function to parse a value from a string.
///
/// If both [`DisplayFn`] and [`ParseFn`] are set, we should be able to round-trip the value.
///
/// # Safety
///
/// The `target` parameter has the correct layout and alignment, but points to
/// uninitialized memory. If this function succeeds, it should return `Ok` with the
/// same pointer wrapped in an [`Opaque`]. If parsing fails, it returns `Err` with an error.
pub type ParseFn =
    for<'mem> unsafe fn(s: &str, target: OpaqueUninit<'mem>) -> Result<Opaque<'mem>, ParseError>;

/// Generates a [`ParseFn`] for a concrete type
pub const fn parse_fn_for<T: core::str::FromStr>() -> Option<ParseFn> {
    Some(|s: &str, target: OpaqueUninit<'_>| unsafe {
        match s.parse::<T>() {
            Ok(value) => Ok(target.put(value)),
            Err(_) => Err(ParseError::Generic("failed to parse string")),
        }
    })
}

/// Error returned by [`ParseFn`]
#[non_exhaustive]
#[derive(Debug)]
pub enum ParseError {
    /// Generic error message
    Generic(&'static str),
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParseError::Generic(msg) => write!(f, "Parse failed: {}", msg),
        }
    }
}

impl core::error::Error for ParseError {}

/// Function to try converting from another type
///
/// # Safety
///
/// The `target` parameter has the correct layout and alignment, but points to
/// uninitialized memory. If this function succeeds, it should return `Ok` with the
/// same pointer wrapped in an [`Opaque`]. If conversion fails, it returns `Err` with an error.
pub type TryFromFn = for<'src, 'mem> unsafe fn(
    source: OpaqueConst<'src>,
    target: OpaqueUninit<'mem>,
) -> Result<Opaque<'mem>, TryFromError>;

/// Error type for TryFrom conversion failures
#[non_exhaustive]
#[derive(Debug)]
pub enum TryFromError {
    /// Generic conversion error
    Generic(&'static str),
    /// The target shape doesn't implement conversion from any source shape (no try_from in vtable)
    Unimplemented(&'static Shape),
    /// The target shape has a conversion implementation, but it doesn't support converting from this specific source shape
    Incompatible {
        /// The source shape that we tried to convert from
        source: &'static Shape,
        /// The target shape that we tried to convert to
        target: &'static Shape,
    },
}

impl core::fmt::Display for TryFromError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TryFromError::Generic(msg) => write!(f, "Conversion failed: {}", msg),
            TryFromError::Unimplemented(shape) => write!(
                f,
                "Conversion failed: Shape {} doesn't implement any conversions (no try_from function)",
                shape
            ),
            TryFromError::Incompatible { source, target } => write!(
                f,
                "Conversion failed: Cannot convert from shape {} to shape {}",
                source, target
            ),
        }
    }
}

impl core::error::Error for TryFromError {}

//======== Comparison ========

/// Function to check if two values are partially equal
///
/// # Safety
///
/// Both `left` and `right` parameters must point to aligned, initialized memory of the correct type.
pub type PartialEqFn = for<'l, 'r> unsafe fn(left: OpaqueConst<'l>, right: OpaqueConst<'r>) -> bool;

/// Generates a [`PartialEqFn`] for a concrete type
pub const fn partial_eq_fn_for<T: PartialEq>() -> Option<PartialEqFn> {
    Some(|left: OpaqueConst<'_>, right: OpaqueConst<'_>| -> bool {
        let left_val = unsafe { left.as_ref::<T>() };
        let right_val = unsafe { right.as_ref::<T>() };
        left_val == right_val
    })
}

/// Function to compare two values and return their ordering if comparable
///
/// # Safety
///
/// Both `left` and `right` parameters must point to aligned, initialized memory of the correct type.
pub type PartialOrdFn =
    for<'l, 'r> unsafe fn(left: OpaqueConst<'l>, right: OpaqueConst<'r>) -> Option<Ordering>;

/// Generates a [`PartialOrdFn`] for a concrete type
pub const fn partial_ord_fn_for<T: PartialOrd>() -> Option<PartialOrdFn> {
    Some(
        |left: OpaqueConst<'_>, right: OpaqueConst<'_>| -> Option<Ordering> {
            let left_val = unsafe { left.as_ref::<T>() };
            let right_val = unsafe { right.as_ref::<T>() };
            left_val.partial_cmp(right_val)
        },
    )
}

/// Function to compare two values and return their ordering
///
/// # Safety
///
/// Both `left` and `right` parameters must point to aligned, initialized memory of the correct type.
pub type CmpFn = for<'l, 'r> unsafe fn(left: OpaqueConst<'l>, right: OpaqueConst<'r>) -> Ordering;

/// Generates a [`CmpFn`] for a concrete type
pub const fn cmp_fn_for<T: Ord>() -> Option<CmpFn> {
    Some(
        |left: OpaqueConst<'_>, right: OpaqueConst<'_>| -> Ordering {
            let left_val = unsafe { left.as_ref::<T>() };
            let right_val = unsafe { right.as_ref::<T>() };
            left_val.cmp(right_val)
        },
    )
}

//======== Hashing ========

/// Function to hash a value
///
/// # Safety
///
/// The `value` parameter must point to aligned, initialized memory of the correct type.
/// The hasher pointer must be a valid pointer to a Hasher trait object.
pub type HashFn = for<'mem> unsafe fn(
    value: OpaqueConst<'mem>,
    hasher_this: Opaque<'mem>,
    hasher_write_fn: HasherWriteFn,
);

/// Generates a [`HashFn`] for a concrete type
pub const fn hash_fn_for<T: core::hash::Hash>() -> Option<HashFn> {
    Some(
        |value: OpaqueConst<'_>, hasher_this: Opaque<'_>, hasher_write_fn: HasherWriteFn| unsafe {
            let val = value.as_ref::<T>();
            val.hash(&mut HasherProxy::new(hasher_this, hasher_write_fn));
        },
    )
}

/// Function to write bytes to a hasher
///
/// # Safety
///
/// The `hasher_self` parameter must be a valid pointer to a hasher
pub type HasherWriteFn = for<'mem> unsafe fn(hasher_self: Opaque<'mem>, bytes: &[u8]);

/// Provides an implementation of [`core::hash::Hasher`] for a given hasher pointer and write function
///
/// See [`HashFn`] for more details on the parameters.
///
/// Example usage (for a type that already implements `Hasher`)
///
/// ```rust,ignore
/// hash: Some(|value, hasher_self, hasher_write_fn| unsafe {
///     value
///         .as_ref::<Self>()
///         .hash(&mut HasherProxy::new(hasher_self, hasher_write_fn));
/// }),
/// ```
pub struct HasherProxy<'a> {
    hasher_this: Opaque<'a>,
    hasher_write_fn: HasherWriteFn,
}

impl<'a> HasherProxy<'a> {
    /// Create a new `HasherProxy` from a hasher pointer and a write function
    ///
    /// # Safety
    ///
    /// The `hasher_this` parameter must be a valid pointer to a Hasher trait object.
    /// The `hasher_write_fn` parameter must be a valid function pointer.
    pub unsafe fn new(hasher_this: Opaque<'a>, hasher_write_fn: HasherWriteFn) -> Self {
        Self {
            hasher_this,
            hasher_write_fn,
        }
    }
}

impl core::hash::Hasher for HasherProxy<'_> {
    fn finish(&self) -> u64 {
        unimplemented!("finish is not needed for this implementation")
    }
    fn write(&mut self, bytes: &[u8]) {
        unsafe { (self.hasher_write_fn)(self.hasher_this, bytes) }
    }
}

//======== Marker Traits ========

bitflags! {
    /// Bitflags for common marker traits that a type may implement
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct MarkerTraits: u8 {
        /// Indicates that the type implements the [`Eq`] marker trait
        const EQ = 1 << 0;
        /// Indicates that the type implements the [`Send`] marker trait
        const SEND = 1 << 1;
        /// Indicates that the type implements the [`Sync`] marker trait
        const SYNC = 1 << 2;
        /// Indicates that the type implements the [`Copy`] marker trait
        const COPY = 1 << 3;
    }
}

//======== Display and Debug ========

/// Function to format a value for display
///
/// If both [`DisplayFn`] and [`ParseFn`] are set, we should be able to round-trip the value.
///
/// # Safety
///
/// The `value` parameter must point to aligned, initialized memory of the correct type.
pub type DisplayFn = for<'mem> unsafe fn(
    value: OpaqueConst<'mem>,
    f: &mut core::fmt::Formatter,
) -> core::fmt::Result;

/// Generates a [`DisplayFn`] for a concrete type
pub const fn display_fn_for<T: core::fmt::Display>() -> Option<DisplayFn> {
    Some(
        |value: OpaqueConst<'_>, f: &mut core::fmt::Formatter| -> core::fmt::Result {
            let val = unsafe { value.as_ref::<T>() };
            write!(f, "{val}")
        },
    )
}

/// Function to format a value for debug.
/// If this returns None, the shape did not implement Debug.
///
/// # Safety
///
/// The `value` parameter must point to aligned, initialized memory of the correct type.
pub type DebugFn = for<'mem> unsafe fn(
    value: OpaqueConst<'mem>,
    f: &mut core::fmt::Formatter,
) -> core::fmt::Result;

/// Generates a [`DebugFn`] for a concrete type
pub const fn debug_fn_for<T: core::fmt::Debug>() -> Option<DebugFn> {
    Some(
        |value: OpaqueConst<'_>, f: &mut core::fmt::Formatter| -> core::fmt::Result {
            let val = unsafe { value.as_ref::<T>() };
            write!(f, "{val:?}")
        },
    )
}

/// VTable for common operations that can be performed on any shape
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ValueVTable {
    /// cf. [`TypeNameFn`]
    pub type_name: TypeNameFn,

    /// cf. [`DisplayFn`]
    pub display: Option<DisplayFn>,

    /// cf. [`DebugFn`]
    pub debug: Option<DebugFn>,

    /// cf. [`DefaultInPlaceFn`]
    pub default_in_place: Option<DefaultInPlaceFn>,

    /// cf. [`CloneInPlaceFn`]
    pub clone_into: Option<CloneIntoFn>,

    /// Marker traits implemented by the type
    // FIXME: move out of vtable, it's not really... functions.
    // Belongs in Shape directly.
    pub marker_traits: MarkerTraits,

    /// cf. [`PartialEqFn`] for equality comparison
    pub eq: Option<PartialEqFn>,

    /// cf. [`PartialOrdFn`] for partial ordering comparison
    pub partial_ord: Option<PartialOrdFn>,

    /// cf. [`CmpFn`] for total ordering
    pub ord: Option<CmpFn>,

    /// cf. [`HashFn`]
    pub hash: Option<HashFn>,

    /// cf. [`DropInPlaceFn`] — if None, drops without side-effects
    pub drop_in_place: Option<DropInPlaceFn>,

    /// cf. [`ParseFn`]
    pub parse: Option<ParseFn>,

    /// cf. [`TryFromFn`]
    pub try_from: Option<TryFromFn>,
}

impl ValueVTable {
    /// Check if the type implements the [`Eq`] marker trait
    pub fn is_eq(&self) -> bool {
        self.marker_traits.contains(MarkerTraits::EQ)
    }

    /// Check if the type implements the [`Send`] marker trait
    pub fn is_send(&self) -> bool {
        self.marker_traits.contains(MarkerTraits::SEND)
    }

    /// Check if the type implements the [`Sync`] marker trait
    pub fn is_sync(&self) -> bool {
        self.marker_traits.contains(MarkerTraits::SYNC)
    }

    /// Check if the type implements the [`Copy`] marker trait
    pub fn is_copy(&self) -> bool {
        self.marker_traits.contains(MarkerTraits::COPY)
    }

    /// Creates a new [`ValueVTableBuilder`]
    pub const fn builder() -> ValueVTableBuilder {
        ValueVTableBuilder::new()
    }
}

/// Builds a [`ValueVTable`]
pub struct ValueVTableBuilder {
    type_name: Option<TypeNameFn>,
    display: Option<DisplayFn>,
    debug: Option<DebugFn>,
    default_in_place: Option<DefaultInPlaceFn>,
    clone_into: Option<CloneIntoFn>,
    marker_traits: MarkerTraits,
    eq: Option<PartialEqFn>,
    partial_ord: Option<PartialOrdFn>,
    ord: Option<CmpFn>,
    hash: Option<HashFn>,
    drop_in_place: Option<DropInPlaceFn>,
    parse: Option<ParseFn>,
    try_from: Option<TryFromFn>,
}

impl ValueVTableBuilder {
    /// Creates a new [`ValueVTableBuilder`] with all fields set to `None`.
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self {
            type_name: None,
            display: None,
            debug: None,
            default_in_place: None,
            clone_into: None,
            marker_traits: MarkerTraits::empty(),
            eq: None,
            partial_ord: None,
            ord: None,
            hash: None,
            drop_in_place: None,
            parse: None,
            try_from: None,
        }
    }

    /// Sets the type name function for this builder.
    pub const fn type_name(mut self, type_name: TypeNameFn) -> Self {
        self.type_name = Some(type_name);
        self
    }

    /// Sets the display function for this builder.
    pub const fn display(mut self, display: DisplayFn) -> Self {
        self.display = Some(display);
        self
    }

    /// Sets the display function for this builder if Some.
    pub const fn display_maybe(mut self, display: Option<DisplayFn>) -> Self {
        self.display = display;
        self
    }

    /// Sets the debug function for this builder.
    pub const fn debug(mut self, debug: DebugFn) -> Self {
        self.debug = Some(debug);
        self
    }

    /// Sets the debug function for this builder if Some.
    pub const fn debug_maybe(mut self, debug: Option<DebugFn>) -> Self {
        self.debug = debug;
        self
    }

    /// Sets the default_in_place function for this builder.
    pub const fn default_in_place(mut self, default_in_place: DefaultInPlaceFn) -> Self {
        self.default_in_place = Some(default_in_place);
        self
    }

    /// Sets the default_in_place function for this builder if Some.
    pub const fn default_in_place_maybe(
        mut self,
        default_in_place: Option<DefaultInPlaceFn>,
    ) -> Self {
        self.default_in_place = default_in_place;
        self
    }

    /// Sets the clone_into function for this builder.
    pub const fn clone_into(mut self, clone_into: CloneIntoFn) -> Self {
        self.clone_into = Some(clone_into);
        self
    }

    /// Sets the clone_into function for this builder if Some.
    pub const fn clone_into_maybe(mut self, clone_into: Option<CloneIntoFn>) -> Self {
        self.clone_into = clone_into;
        self
    }

    /// Sets the marker traits for this builder.
    pub const fn marker_traits(mut self, marker_traits: MarkerTraits) -> Self {
        self.marker_traits = marker_traits;
        self
    }

    /// Sets the eq function for this builder.
    pub const fn eq(mut self, eq: PartialEqFn) -> Self {
        self.eq = Some(eq);
        self
    }

    /// Sets the eq function for this builder if Some.
    pub const fn eq_maybe(mut self, eq: Option<PartialEqFn>) -> Self {
        self.eq = eq;
        self
    }

    /// Sets the partial_ord function for this builder.
    pub const fn partial_ord(mut self, partial_ord: PartialOrdFn) -> Self {
        self.partial_ord = Some(partial_ord);
        self
    }

    /// Sets the partial_ord function for this builder if Some.
    pub const fn partial_ord_maybe(mut self, partial_ord: Option<PartialOrdFn>) -> Self {
        self.partial_ord = partial_ord;
        self
    }

    /// Sets the ord function for this builder.
    pub const fn ord(mut self, ord: CmpFn) -> Self {
        self.ord = Some(ord);
        self
    }

    /// Sets the ord function for this builder if Some.
    pub const fn ord_maybe(mut self, ord: Option<CmpFn>) -> Self {
        self.ord = ord;
        self
    }

    /// Sets the hash function for this builder.
    pub const fn hash(mut self, hash: HashFn) -> Self {
        self.hash = Some(hash);
        self
    }

    /// Sets the hash function for this builder if Some.
    pub const fn hash_maybe(mut self, hash: Option<HashFn>) -> Self {
        self.hash = hash;
        self
    }

    /// Sets the drop_in_place function for this builder.
    pub const fn drop_in_place(mut self, drop_in_place: DropInPlaceFn) -> Self {
        self.drop_in_place = Some(drop_in_place);
        self
    }

    /// Sets the drop_in_place function for this builder if Some.
    pub const fn drop_in_place_maybe(mut self, drop_in_place: Option<DropInPlaceFn>) -> Self {
        self.drop_in_place = drop_in_place;
        self
    }

    /// Sets the parse function for this builder.
    pub const fn parse(mut self, parse: ParseFn) -> Self {
        self.parse = Some(parse);
        self
    }

    /// Sets the parse function for this builder if Some.
    pub const fn parse_maybe(mut self, parse: Option<ParseFn>) -> Self {
        self.parse = parse;
        self
    }

    /// Sets the try_from function for this builder.
    pub const fn try_from(mut self, try_from: TryFromFn) -> Self {
        self.try_from = Some(try_from);
        self
    }

    /// Sets the try_from function for this builder if Some.
    pub const fn try_from_maybe(mut self, try_from: Option<TryFromFn>) -> Self {
        self.try_from = try_from;
        self
    }

    /// Builds the [`ValueVTable`] from the current state of the builder.
    pub const fn build(self) -> ValueVTable {
        ValueVTable {
            type_name: self.type_name.unwrap(),
            display: self.display,
            debug: self.debug,
            default_in_place: self.default_in_place,
            clone_into: self.clone_into,
            marker_traits: self.marker_traits,
            eq: self.eq,
            partial_ord: self.partial_ord,
            ord: self.ord,
            hash: self.hash,
            drop_in_place: self.drop_in_place,
            parse: self.parse,
            try_from: self.try_from,
        }
    }
}
