use super::*;
use unsynn::*;

/// Processes an enum to implement Facet
///
/// Example input:
/// ```rust
/// #[repr(u8)]
/// enum Color {
///     Red,
///     Green,
///     Blue(u8, u8),
///     Custom { r: u8, g: u8, b: u8 }
/// }
/// ```
pub(crate) fn process_enum(parsed: Enum) -> proc_macro::TokenStream {
    let enum_name = parsed.name.to_string();

    // Check for explicit repr attribute
    let has_repr = parsed
        .attributes
        .iter()
        .any(|attr| matches!(attr.body.content, AttributeInner::Repr(_)));

    if !has_repr {
        return r#"compile_error!("Enums must have an explicit representation (e.g. #[repr(u8)]) to be used with Facet")"#
            .into_token_stream()
            .into();
    }

    // Get discriminant type - we'll use this for all variants
    let mut discriminant_type = "u8".to_string(); // Default
    for attr in &parsed.attributes {
        if let AttributeInner::Repr(repr_attr) = &attr.body.content {
            discriminant_type = repr_attr.attr.content.to_string();
            break;
        }
    }

    // Collect shadow struct definitions separately from variant expressions
    let mut shadow_struct_defs = Vec::new();
    let mut variant_expressions = Vec::new();

    // Process each variant using enumerate to get discriminant values
    for (discriminant_value, var_like) in parsed.body.content.0.iter().enumerate() {
        match &var_like.value {
            EnumVariantLike::Unit(unit) => {
                let variant_name = unit.name.to_string();
                variant_expressions.push(format!(
                    "facet::enum_unit_variant!({enum_name}, {variant_name}, {discriminant_value})"
                ));
            }
            EnumVariantLike::Tuple(tuple) => {
                let variant_name = tuple.name.to_string();

                // Generate shadow struct for this tuple variant to calculate offsets
                let shadow_struct_name = format!("__Shadow{}_{}", enum_name, variant_name);

                // Build the list of fields and types for the shadow struct
                let fields_with_types = tuple
                    ._paren
                    .content
                    .0
                    .iter()
                    .enumerate()
                    .map(|(idx, field)| {
                        let typ = field.value.typ.to_string();
                        format!("_{}: {}", idx, typ)
                    })
                    .collect::<Vec<String>>()
                    .join(", ");

                // Add shadow struct definition
                shadow_struct_defs.push(format!(
                    "#[repr(C)]\nstruct {} {{\n    _discriminant: {},\n    {}\n}}",
                    shadow_struct_name, discriminant_type, fields_with_types
                ));

                // Build the list of field types with calculated offsets
                let fields_with_offsets = tuple
                    ._paren
                    .content
                    .0
                    .iter()
                    .enumerate()
                    .map(|(idx, field)| {
                        let typ = field.value.typ.to_string();
                        format!(
                            "({}, core::mem::offset_of!({}, _{}))",
                            typ, shadow_struct_name, idx
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(", ");

                // Add variant expression - now with discriminant
                variant_expressions.push(format!(
                    "facet::enum_tuple_variant!({}, {}, [{}], {})",
                    enum_name, variant_name, fields_with_offsets, discriminant_value
                ));
            }
            EnumVariantLike::Struct(struct_var) => {
                let variant_name = struct_var.name.to_string();

                // Generate shadow struct for this struct variant to calculate offsets
                let shadow_struct_name = format!("__Shadow{}_{}", enum_name, variant_name);

                // Build the list of fields and types
                let fields_with_types = struct_var
                    .fields
                    .content
                    .0
                    .iter()
                    .map(|field| {
                        let name = field.value.name.to_string();
                        let typ = field.value.typ.to_string();
                        format!("{}: {}", name, typ)
                    })
                    .collect::<Vec<String>>()
                    .join(", ");

                // Add shadow struct definition
                shadow_struct_defs.push(format!(
                    "#[repr(C)]\nstruct {} {{\n    _discriminant: {},\n    {}\n}}",
                    shadow_struct_name, discriminant_type, fields_with_types
                ));

                // Build the list of field types with calculated offsets
                let fields_with_offsets = struct_var
                    .fields
                    .content
                    .0
                    .iter()
                    .map(|field| {
                        let name = field.value.name.to_string();
                        let typ = field.value.typ.to_string();
                        format!(
                            "({}: {}, core::mem::offset_of!({}, {}))",
                            name, typ, shadow_struct_name, name
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(", ");

                // Add variant expression - now with discriminant
                variant_expressions.push(format!(
                    "facet::enum_struct_variant!({}, {}, {{{}}}, {})",
                    enum_name, variant_name, fields_with_offsets, discriminant_value
                ));
            }
        }
    }

    // Join the shadow struct definitions and variant expressions
    let shadow_structs = shadow_struct_defs.join("\n\n");
    let variants = variant_expressions.join(", ");

    // Extract the repr type
    let mut repr_type = "Default"; // Default fallback
    for attr in &parsed.attributes {
        if let AttributeInner::Repr(repr_attr) = &attr.body.content {
            repr_type = match repr_attr.attr.content.to_string().as_str() {
                "u8" => "U8",
                "u16" => "U16",
                "u32" => "U32",
                "u64" => "U64",
                "usize" => "USize",
                "i8" => "I8",
                "i16" => "I16",
                "i32" => "I32",
                "i64" => "I64",
                "isize" => "ISize",
                _ => "Default", // Unknown repr type
            };
            break;
        }
    }

    // Generate the impl
    let output = format!(
        r#"
#[automatically_derived]
unsafe impl facet::Facet for {enum_name} {{
    const SHAPE: &'static facet::Shape = &const {{
        // Define all shadow structs at the beginning of the const block
        // to ensure they're in scope for offset_of! macros
        {shadow_structs}

        facet::Shape::builder()
            .id(facet::ConstTypeId::of::<{enum_name}>())
            .layout(core::alloc::Layout::new::<Self>())
            .vtable(facet::value_vtable!(
                {enum_name},
                |f, _opts| core::fmt::Write::write_str(f, "{enum_name}")
            ))
            .def(facet::Def::Enum(facet::EnumDef::builder()
                // Use variant expressions that just reference the shadow structs
                // which are now defined above
                .variants(facet::enum_variants!({enum_name}, [{variants}]))
                .repr(facet::EnumRepr::{repr_type})
                .build()))
            .build()
    }};
}}
        "#
    );

    // Output generated code
    // Don't use panic for debugging as it makes code unreachable

    // Return the generated code
    output.into_token_stream().into()
}
