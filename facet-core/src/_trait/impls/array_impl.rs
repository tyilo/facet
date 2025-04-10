use crate::*;
use core::alloc::Layout;
use std::{cmp::Ordering, iter::zip};

unsafe impl<T, const L: usize> Facet for [T; L]
where
    T: Facet,
{
    const SHAPE: &'static Shape = &const {
        Shape::builder()
            .id(ConstTypeId::of::<[T; L]>())
            .layout(Layout::new::<[T; L]>())
            .vtable(
                &const {
                    let mut builder = ValueVTable::builder()
                        .marker_traits(T::SHAPE.vtable.marker_traits)
                        .type_name(|f, opts| {
                            if let Some(opts) = opts.for_children() {
                                write!(f, "[")?;
                                (T::SHAPE.vtable.type_name)(f, opts)?;
                                write!(f, "; {L}]")
                            } else {
                                write!(f, "[⋯; {L}]")
                            }
                        })
                        .drop_in_place(|value| unsafe {
                            core::ptr::drop_in_place(value.as_mut::<[T; L]>());
                        });
                    if T::SHAPE.vtable.display.is_some() {
                        builder = builder.display(|value, f| {
                            let value = unsafe { value.as_ref::<[T; L]>() };
                            write!(f, "[")?;

                            for (idx, value) in value.iter().enumerate() {
                                unsafe {
                                    (T::SHAPE.vtable.display.unwrap_unchecked())(
                                        OpaqueConst::new(value),
                                        f,
                                    )?
                                };
                                if idx != L - 1 {
                                    write!(f, ", ")?;
                                }
                            }
                            write!(f, "]")
                        });
                    }
                    if T::SHAPE.vtable.debug.is_some() {
                        builder = builder.debug(|value, f| {
                            let value = unsafe { value.as_ref::<[T; L]>() };
                            write!(f, "[")?;

                            for (idx, value) in value.iter().enumerate() {
                                unsafe {
                                    (T::SHAPE.vtable.debug.unwrap_unchecked())(
                                        OpaqueConst::new(value),
                                        f,
                                    )?
                                };
                                if idx != L - 1 {
                                    write!(f, ", ")?;
                                }
                            }
                            write!(f, "]")
                        });
                    }
                    if T::SHAPE.vtable.eq.is_some() {
                        builder = builder.eq(|a, b| {
                            let a = unsafe { a.as_ref::<[T; L]>() };
                            let b = unsafe { b.as_ref::<[T; L]>() };
                            zip(a, b).all(|(a, b)| unsafe {
                                (T::SHAPE.vtable.eq.unwrap_unchecked())(
                                    OpaqueConst::new(a),
                                    OpaqueConst::new(b),
                                )
                            })
                        });
                    }
                    if L == 0 {
                        // Zero-length arrays implement `Default` irrespective of the element type
                        builder =
                            builder.default_in_place(|target| unsafe { target.assume_init() });
                    } else if L <= 32 && T::SHAPE.vtable.default_in_place.is_some() {
                        builder = builder.default_in_place(|target| unsafe {
                            let t_dip = T::SHAPE.vtable.default_in_place.unwrap_unchecked();
                            let stride = T::SHAPE.layout.pad_to_align().size();
                            for offset in (0..L).step_by(stride) {
                                t_dip(target.field_uninit(offset));
                            }
                            target.assume_init()
                        });
                    } else {
                        // arrays do not yet implement `Default` for > 32 elements due to
                        // specializing the `0` len case
                    }
                    if T::SHAPE.vtable.clone_into.is_some() {
                        builder = builder.clone_into(|src, dst| unsafe {
                            let t_cip = T::SHAPE.vtable.clone_into.unwrap_unchecked();
                            let src = src.as_ref::<[T; L]>();
                            let stride = T::SHAPE.layout.pad_to_align().size();
                            for (idx, src) in src.iter().enumerate() {
                                (t_cip)(OpaqueConst::new(src), dst.field_uninit(idx * stride));
                            }
                            dst.assume_init()
                        });
                    }
                    if T::SHAPE.vtable.partial_ord.is_some() {
                        builder = builder.partial_ord(|a, b| {
                            let a = unsafe { a.as_ref::<[T; L]>() };
                            let b = unsafe { b.as_ref::<[T; L]>() };
                            zip(a, b)
                                .find_map(|(a, b)| unsafe {
                                    match (T::SHAPE.vtable.partial_ord.unwrap_unchecked())(
                                        OpaqueConst::new(a),
                                        OpaqueConst::new(b),
                                    ) {
                                        Some(Ordering::Equal) => None,
                                        c => Some(c),
                                    }
                                })
                                .unwrap_or(Some(Ordering::Equal))
                        });
                    }
                    if T::SHAPE.vtable.ord.is_some() {
                        builder = builder.ord(|a, b| {
                            let a = unsafe { a.as_ref::<[T; L]>() };
                            let b = unsafe { b.as_ref::<[T; L]>() };
                            zip(a, b)
                                .find_map(|(a, b)| unsafe {
                                    match (T::SHAPE.vtable.ord.unwrap_unchecked())(
                                        OpaqueConst::new(a),
                                        OpaqueConst::new(b),
                                    ) {
                                        Ordering::Equal => None,
                                        c => Some(c),
                                    }
                                })
                                .unwrap_or(Ordering::Equal)
                        });
                    }
                    if T::SHAPE.vtable.hash.is_some() {
                        builder = builder.hash(|value, state, hasher| {
                            let value = unsafe { value.as_ref::<[T; L]>() };
                            for value in value {
                                unsafe {
                                    (T::SHAPE.vtable.hash.unwrap_unchecked())(
                                        OpaqueConst::new(value),
                                        state,
                                        hasher,
                                    )
                                }
                            }
                        });
                    }
                    builder.build()
                },
            )
            .def(Def::List(
                ListDef::builder()
                    .vtable(
                        &const {
                            ListVTable::builder()
                        .init_in_place_with_capacity(|_, _| Err(()))
                        .push(|_, _| {
                            panic!("Cannot push to [T; {L}]");
                        })
                        .len(|_| L)
                        .get_item_ptr(|ptr, index| unsafe {
                            if index >= L {
                                panic!(
                                    "Index out of bounds: the len is {L} but the index is {index}"
                                );
                            }
                            OpaqueConst::new(ptr.as_ptr::<[T; L]>())
                        })
                        .build()
                        },
                    )
                    .t(T::SHAPE)
                    .build(),
            ))
            .build()
    };
}
