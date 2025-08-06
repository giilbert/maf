use facet::{PointerType, StructKind, TextualType, UserType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSchema {
    pub stores: Vec<StoreSerialized>,
    pub rpcs: Vec<RpcSerialized>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoreSerialized {
    pub name: String,
    pub select: TypeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RpcSerialized {
    pub name: String,
    pub params: Option<TypeKind>,
    pub result: Option<TypeKind>,
}

/// A high-level representation of types commonly used in programming languages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum TypeKind {
    /// Numeric, boolean, character, string, etc.
    Primitive(PrimitiveType),
    /// Option<T> / `_ | null` types.
    Nullable(Box<TypeKind>),
    /// Record types, similar to structs or objects.
    Record(Box<RecordType>),
    /// A fixed-length array type.
    Tuple(Box<TupleType>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum PrimitiveType {
    Numeric(NumericType),
    Bool,
    Char,
    String,
    Unit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecordType {
    pub fields: Vec<(String, TypeKind)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MapType {
    pub key: TypeKind,
    pub value: TypeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum NumericType {
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
    F32,
    F64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TupleType {
    pub elements: Vec<TypeKind>,
}

impl From<&'static facet::Shape> for TypeKind {
    fn from(shape: &'static facet::Shape) -> Self {
        match shape.ty {
            facet::Type::Primitive(facet::PrimitiveType::Boolean) => {
                TypeKind::Primitive(PrimitiveType::Bool)
            }
            facet::Type::Primitive(facet::PrimitiveType::Textual(TextualType::Char)) => {
                TypeKind::Primitive(PrimitiveType::Char)
            }
            facet::Type::Primitive(facet::PrimitiveType::Textual(TextualType::Str)) => {
                TypeKind::Primitive(PrimitiveType::String)
            }
            facet::Type::Primitive(facet::PrimitiveType::Numeric(numeric)) => {
                let sized_layout = match shape.layout {
                    facet::ShapeLayout::Sized(layout) => layout,
                    _ => panic!("Numeric types must have a sized layout"),
                };

                match (numeric, sized_layout.size()) {
                    (facet::NumericType::Integer { signed: true }, 1) => {
                        TypeKind::Primitive(PrimitiveType::Numeric(NumericType::I8))
                    }
                    (facet::NumericType::Integer { signed: true }, 2) => {
                        TypeKind::Primitive(PrimitiveType::Numeric(NumericType::I16))
                    }
                    (facet::NumericType::Integer { signed: true }, 4) => {
                        TypeKind::Primitive(PrimitiveType::Numeric(NumericType::I32))
                    }
                    (facet::NumericType::Integer { signed: true }, 8) => {
                        TypeKind::Primitive(PrimitiveType::Numeric(NumericType::I64))
                    }
                    (facet::NumericType::Integer { signed: true }, 16) => {
                        TypeKind::Primitive(PrimitiveType::Numeric(NumericType::I128))
                    }
                    (facet::NumericType::Integer { signed: false }, 1) => {
                        TypeKind::Primitive(PrimitiveType::Numeric(NumericType::U8))
                    }
                    (facet::NumericType::Integer { signed: false }, 2) => {
                        TypeKind::Primitive(PrimitiveType::Numeric(NumericType::U16))
                    }
                    (facet::NumericType::Integer { signed: false }, 4) => {
                        TypeKind::Primitive(PrimitiveType::Numeric(NumericType::U32))
                    }
                    (facet::NumericType::Integer { signed: false }, 8) => {
                        TypeKind::Primitive(PrimitiveType::Numeric(NumericType::U64))
                    }
                    (facet::NumericType::Integer { signed: false }, 16) => {
                        TypeKind::Primitive(PrimitiveType::Numeric(NumericType::U128))
                    }
                    (facet::NumericType::Float, 4) => {
                        TypeKind::Primitive(PrimitiveType::Numeric(NumericType::F32))
                    }
                    (facet::NumericType::Float, 8) => {
                        TypeKind::Primitive(PrimitiveType::Numeric(NumericType::F64))
                    }
                    _ => panic!("unsupported numeric type or size"),
                }
            }
            // Special builtin types
            facet::Type::User(UserType::Opaque) if shape.type_identifier == "String" => {
                TypeKind::Primitive(PrimitiveType::String)
            }
            facet::Type::User(UserType::Opaque) if shape.type_identifier == "()" => {
                TypeKind::Primitive(PrimitiveType::Unit)
            }

            facet::Type::User(UserType::Opaque) if shape.type_identifier == "Option" => {
                TypeKind::Nullable(Box::new((shape.type_params[0].shape)().into()))
            }
            facet::Type::User(UserType::Enum(enum_type)) if shape.type_identifier == "Option" => {
                // Option is represented as an enum with two variants: Some(T) and None
                TypeKind::Nullable(Box::new(
                    (enum_type
                        .variants
                        .iter()
                        .find(|p| p.name == "Some")
                        .expect("Some(T) variant not found in Option<T> enum")
                        .data
                        .fields[0]
                        .shape)
                        .into(),
                ))
            }

            facet::Type::User(UserType::Struct(struct_type))
                if struct_type.kind == StructKind::Struct =>
            {
                TypeKind::Record(Box::new(RecordType {
                    fields: struct_type
                        .fields
                        .iter()
                        .map(|field| (field.name.to_string(), TypeKind::from(field.shape)))
                        .collect(),
                }))
            }
            facet::Type::User(UserType::Struct(struct_type))
                if struct_type.kind == StructKind::TupleStruct
                    || struct_type.kind == StructKind::Tuple =>
            {
                TypeKind::Tuple(Box::new(TupleType {
                    elements: struct_type
                        .fields
                        .iter()
                        .map(|field| TypeKind::from(field.shape))
                        .collect(),
                }))
            }

            facet::Type::Pointer(PointerType::Reference(reference)) => (reference.target)().into(),
            other => todo!("unsupported type: {:?}", other),
        }
    }
}

#[cfg(test)]
mod tests {
    use facet::Facet;

    use super::*;

    #[test]
    fn serialize_primitive_types() {
        assert_eq!(
            TypeKind::from(u8::SHAPE),
            TypeKind::Primitive(PrimitiveType::Numeric(NumericType::U8))
        );
        assert_eq!(
            TypeKind::from(i32::SHAPE),
            TypeKind::Primitive(PrimitiveType::Numeric(NumericType::I32))
        );
        assert_eq!(
            TypeKind::from(f64::SHAPE),
            TypeKind::Primitive(PrimitiveType::Numeric(NumericType::F64))
        );
        assert_eq!(
            TypeKind::from(bool::SHAPE),
            TypeKind::Primitive(PrimitiveType::Bool)
        );
        assert_eq!(
            TypeKind::from(char::SHAPE),
            TypeKind::Primitive(PrimitiveType::Char)
        );
        assert_eq!(
            TypeKind::from(str::SHAPE),
            TypeKind::Primitive(PrimitiveType::String)
        );
        assert_eq!(
            TypeKind::from(String::SHAPE),
            TypeKind::Primitive(PrimitiveType::String)
        );
    }

    #[test]
    fn serialize_record_types() {
        #[derive(Facet)]
        struct Person {
            pub name: String,
            pub age: u32,
        }

        assert_eq!(
            TypeKind::from(Person::SHAPE),
            TypeKind::Record(Box::new(RecordType {
                fields: vec![
                    (
                        "name".to_string(),
                        TypeKind::Primitive(PrimitiveType::String)
                    ),
                    (
                        "age".to_string(),
                        TypeKind::Primitive(PrimitiveType::Numeric(NumericType::U32))
                    ),
                ],
            }))
        );

        #[derive(Facet)]
        struct Student {
            pub person: Person,
            pub student_id: String,
        }

        assert_eq!(
            TypeKind::from(Student::SHAPE),
            TypeKind::Record(Box::new(RecordType {
                fields: vec![
                    (
                        "person".to_string(),
                        TypeKind::Record(Box::new(RecordType {
                            fields: vec![
                                (
                                    "name".to_string(),
                                    TypeKind::Primitive(PrimitiveType::String)
                                ),
                                (
                                    "age".to_string(),
                                    TypeKind::Primitive(PrimitiveType::Numeric(NumericType::U32))
                                ),
                            ],
                        }))
                    ),
                    (
                        "student_id".to_string(),
                        TypeKind::Primitive(PrimitiveType::String)
                    ),
                ],
            }))
        );
    }

    #[test]
    fn serialize_tuple_types() {
        #[derive(Facet)]
        struct Point(f64, f64);

        assert_eq!(
            TypeKind::from(Point::SHAPE),
            TypeKind::Tuple(Box::new(TupleType {
                elements: vec![
                    TypeKind::Primitive(PrimitiveType::Numeric(NumericType::F64)),
                    TypeKind::Primitive(PrimitiveType::Numeric(NumericType::F64)),
                ],
            }))
        );

        type A = (f64, f64);
        assert_eq!(
            TypeKind::from(A::SHAPE),
            TypeKind::Tuple(Box::new(TupleType {
                elements: vec![
                    TypeKind::Primitive(PrimitiveType::Numeric(NumericType::F64)),
                    TypeKind::Primitive(PrimitiveType::Numeric(NumericType::F64)),
                ],
            }))
        );
    }

    #[test]
    fn serialize_options() {
        assert_eq!(
            TypeKind::from(Option::<String>::SHAPE),
            TypeKind::Nullable(Box::new(TypeKind::Primitive(PrimitiveType::String)))
        );

        assert_eq!(
            TypeKind::from(Option::<i32>::SHAPE),
            TypeKind::Nullable(Box::new(TypeKind::Primitive(PrimitiveType::Numeric(
                NumericType::I32
            ))))
        );

        // Test null-pointer optimization types
        assert_eq!(
            TypeKind::from(Option::<&String>::SHAPE),
            TypeKind::Nullable(Box::new(TypeKind::Primitive(PrimitiveType::String)))
        )
    }
}
