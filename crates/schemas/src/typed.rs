use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum Type {
    Primitive(PrimitiveType),
    Nullable(Box<Type>),
    Record(Box<RecordType>),
}

#[derive(Debug, Clone, Serialize)]
pub enum PrimitiveType {
    Numeric(NumericType),
    Bool,
    String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordType {
    pub fields: Vec<(String, Type)>,
}

#[derive(Debug, Clone, Serialize)]
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
