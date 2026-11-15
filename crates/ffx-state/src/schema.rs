//! The API's schema: each struct's fields and their types, and their
//! encoding. Types: integers and floats little-endian, `bool` one byte (0 or
//! 1), `String` a `u16` byte count then UTF-8, `bytes` a `u16` count then the
//! bytes, a list a `u16` count then its
//! items, `[f32; N]` N floats, a struct its fields in order.

/// A field's type.
#[derive(Debug, Clone, Copy)]
pub enum Ty {
    /// `u8`.
    U8,
    /// `u16`.
    U16,
    /// `u32`.
    U32,
    /// `i16`.
    I16,
    /// `i32`.
    I32,
    /// `f32`.
    F32,
    /// `f64`.
    F64,
    /// `bool`.
    Bool,
    /// `String`.
    Str,
    /// `Vec<u8>`: raw bytes (the game's encoded text).
    Bytes,
    /// `[f32; 3]`.
    Vec3,
    /// `[u8; 3]`: a colour.
    Rgb,
    /// `[f32; 12]`.
    Mat34,
    /// `[f32; 16]`.
    Mat4,
    /// Another struct.
    Struct(fn() -> &'static StructSchema),
    /// A list of a struct.
    List(fn() -> &'static StructSchema),
}

impl Ty {
    /// The type's name as the schema prints it.
    #[must_use]
    pub fn name(self) -> String {
        match self {
            Self::U8 => "u8".into(),
            Self::U16 => "u16".into(),
            Self::U32 => "u32".into(),
            Self::I16 => "i16".into(),
            Self::I32 => "i32".into(),
            Self::F32 => "f32".into(),
            Self::F64 => "f64".into(),
            Self::Bool => "bool".into(),
            Self::Str => "string".into(),
            Self::Bytes => "bytes".into(),
            Self::Vec3 => "vec3".into(),
            Self::Rgb => "rgb".into(),
            Self::Mat34 => "mat3x4".into(),
            Self::Mat4 => "mat4".into(),
            Self::Struct(s) => s().name.into(),
            Self::List(s) => format!("list<{}>", s().name),
        }
    }
}

/// One field.
#[derive(Debug, Clone, Copy)]
pub struct Field {
    /// Its name (`snake_case`).
    pub name: &'static str,
    /// Its type.
    pub ty: Ty,
    /// What it is.
    pub doc: &'static str,
}

/// One struct.
#[derive(Debug)]
pub struct StructSchema {
    /// Its name.
    pub name: &'static str,
    /// What it is.
    pub doc: &'static str,
    /// Its fields, in encoding order.
    pub fields: &'static [Field],
}

/// A struct of the API.
pub trait ApiStruct {
    /// Its schema.
    fn schema() -> &'static StructSchema;
    /// Its fields, encoded in order.
    fn encode_fields(&self, out: &mut Vec<u8>);
}

/// A field type of the API.
pub trait ApiType {
    /// Its schema type.
    const TY: Ty;
    /// Appends its encoding.
    fn encode(&self, out: &mut Vec<u8>);
}

macro_rules! numbers {
    ($($t:ty => $v:ident),*) => {$(
        impl ApiType for $t {
            const TY: Ty = Ty::$v;
            fn encode(&self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.to_le_bytes());
            }
        }
    )*};
}
numbers!(u8 => U8, u16 => U16, u32 => U32, i16 => I16, i32 => I32, f32 => F32, f64 => F64);

impl ApiType for bool {
    const TY: Ty = Ty::Bool;
    fn encode(&self, out: &mut Vec<u8>) {
        out.push(u8::from(*self));
    }
}

/// A count as the encoding's `u16` (longer lists and strings are cut).
fn count(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

impl ApiType for String {
    const TY: Ty = Ty::Str;
    fn encode(&self, out: &mut Vec<u8>) {
        let mut n = usize::from(count(self.len()));
        while !self.is_char_boundary(n) {
            n -= 1;
        }
        out.extend_from_slice(&count(n).to_le_bytes());
        out.extend_from_slice(&self.as_bytes()[..n]);
    }
}

impl ApiType for Vec<u8> {
    const TY: Ty = Ty::Bytes;
    fn encode(&self, out: &mut Vec<u8>) {
        let n = count(self.len());
        out.extend_from_slice(&n.to_le_bytes());
        out.extend_from_slice(&self[..usize::from(n)]);
    }
}

impl ApiType for [u8; 3] {
    const TY: Ty = Ty::Rgb;
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self);
    }
}

impl ApiType for [f32; 3] {
    const TY: Ty = Ty::Vec3;
    fn encode(&self, out: &mut Vec<u8>) {
        for v in self {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
}

impl ApiType for [f32; 12] {
    const TY: Ty = Ty::Mat34;
    fn encode(&self, out: &mut Vec<u8>) {
        for v in self {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
}

impl ApiType for [f32; 16] {
    const TY: Ty = Ty::Mat4;
    fn encode(&self, out: &mut Vec<u8>) {
        for v in self {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
}

impl<T: ApiStruct> ApiType for Vec<T> {
    const TY: Ty = Ty::List(T::schema);
    fn encode(&self, out: &mut Vec<u8>) {
        let n = count(self.len());
        out.extend_from_slice(&n.to_le_bytes());
        for item in self.iter().take(usize::from(n)) {
            item.encode_fields(out);
        }
    }
}

/// The schema as text: one line per struct and field (`name: type`), the
/// root first: what a frontend's generator reads, and a stable record of
/// the API's shape (docs/state-api.md).
#[must_use]
pub fn text() -> String {
    use std::fmt::Write as _;
    let mut s = format!("ffx-state API {}\n", crate::API_VERSION);
    for st in crate::structs() {
        let _ = writeln!(s, "struct {}", st.name);
        for f in st.fields {
            let _ = writeln!(s, "  {}: {}", f.name, f.ty.name());
        }
    }
    s
}
