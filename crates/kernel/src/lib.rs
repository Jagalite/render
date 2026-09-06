//! Private typed kernel IR. Rust builds trees; Naga validates generated artifacts.
use render_core::{Error, Result};
use std::ops::{Add, Div, Mul, Sub};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ty {
    F32,
    U32,
    Bool,
    V2,
    V3,
    V4,
    U3,
    RasterVertex,
}
impl Ty {
    fn text(self) -> &'static str {
        match self {
            Self::F32 => "f32",
            Self::U32 => "u32",
            Self::Bool => "bool",
            Self::V2 => "vec2<f32>",
            Self::V3 => "vec3<f32>",
            Self::V4 => "vec4<f32>",
            Self::U3 => "vec3<u32>",
            Self::RasterVertex => "RasterVertex",
        }
    }
}
#[derive(Clone, Debug)]
pub struct Expr {
    pub ty: Ty,
    node: Box<Node>,
}
#[derive(Clone, Debug)]
enum Node {
    Float(f32),
    Uint(u32),
    Bool(bool),
    Var(String),
    Binary(&'static str, Expr, Expr),
    Call(String, Vec<Expr>),
    Field(Expr, String),
    Read(String, Expr),
}
impl Expr {
    pub fn var(name: &str, ty: Ty) -> Self {
        Self {
            ty,
            node: Box::new(Node::Var(name.into())),
        }
    }
    pub fn field(self, name: &str) -> Self {
        let ty = match name.len() {
            1 => {
                if self.ty == Ty::U3 {
                    Ty::U32
                } else {
                    Ty::F32
                }
            }
            2 => Ty::V2,
            3 => Ty::V3,
            _ => Ty::V4,
        };
        Self {
            ty,
            node: Box::new(Node::Field(self, name.into())),
        }
    }
    fn binary(self, op: &'static str, rhs: Self, ty: Ty) -> Self {
        Self {
            ty,
            node: Box::new(Node::Binary(op, self, rhs)),
        }
    }
    pub fn lt(self, b: Self) -> Self {
        self.binary("<", b, Ty::Bool)
    }
    pub fn le(self, b: Self) -> Self {
        self.binary("<=", b, Ty::Bool)
    }
    pub fn gt(self, b: Self) -> Self {
        self.binary(">", b, Ty::Bool)
    }
    pub fn ge(self, b: Self) -> Self {
        self.binary(">=", b, Ty::Bool)
    }
    pub fn eq(self, b: Self) -> Self {
        self.binary("==", b, Ty::Bool)
    }
    pub fn ne(self, b: Self) -> Self {
        self.binary("!=", b, Ty::Bool)
    }
    pub fn and(self, b: Self) -> Self {
        self.binary("&&", b, Ty::Bool)
    }
    pub fn or(self, b: Self) -> Self {
        self.binary("||", b, Ty::Bool)
    }
    pub fn xor(self, b: Self) -> Self {
        self.binary("^", b, Ty::U32)
    }
    pub fn shift_right(self, b: Self) -> Self {
        self.binary(">>", b, Ty::U32)
    }
    pub fn remainder(self, b: Self) -> Self {
        let ty = self.ty;
        self.binary("%", b, ty)
    }
    fn emit(&self) -> String {
        match self.node.as_ref() {
            Node::Float(v) => format!("{v:?}"),
            Node::Uint(v) => format!("{v}u"),
            Node::Bool(v) => v.to_string(),
            Node::Var(v) => v.clone(),
            Node::Binary(op, a, b) => format!("({} {op} {})", a.emit(), b.emit()),
            Node::Call(name, args) => format!(
                "{name}({})",
                args.iter().map(Self::emit).collect::<Vec<_>>().join(", ")
            ),
            Node::Field(e, name) => format!("({}).{name}", e.emit()),
            Node::Read(name, i) => format!("{name}[{}]", i.emit()),
        }
    }
}
macro_rules! operator {
    ($trait:ident,$method:ident,$op:literal) => {
        impl $trait for Expr {
            type Output = Self;
            fn $method(self, rhs: Self) -> Self {
                let ty = if matches!(self.ty, Ty::F32 | Ty::U32) {
                    rhs.ty
                } else {
                    self.ty
                };
                self.binary($op, rhs, ty)
            }
        }
    };
}
operator!(Add, add, "+");
operator!(Sub, sub, "-");
operator!(Mul, mul, "*");
operator!(Div, div, "/");
pub fn f(v: f32) -> Expr {
    Expr {
        ty: Ty::F32,
        node: Box::new(Node::Float(v)),
    }
}
pub fn u(v: u32) -> Expr {
    Expr {
        ty: Ty::U32,
        node: Box::new(Node::Uint(v)),
    }
}
pub fn boolean(v: bool) -> Expr {
    Expr {
        ty: Ty::Bool,
        node: Box::new(Node::Bool(v)),
    }
}
pub fn call(name: &str, ty: Ty, args: Vec<Expr>) -> Expr {
    Expr {
        ty,
        node: Box::new(Node::Call(name.into(), args)),
    }
}
pub fn read(name: &str, index: Expr) -> Expr {
    Expr {
        ty: Ty::V4,
        node: Box::new(Node::Read(name.into(), index)),
    }
}
pub fn vec2(a: Expr, b: Expr) -> Expr {
    call(Ty::V2.text(), Ty::V2, vec![a, b])
}
pub fn vec3(a: Expr, b: Expr, c: Expr) -> Expr {
    call(Ty::V3.text(), Ty::V3, vec![a, b, c])
}
pub fn vec4(a: Expr, b: Expr) -> Expr {
    call(Ty::V4.text(), Ty::V4, vec![a, b])
}
pub fn splat(v: f32) -> Expr {
    call(Ty::V3.text(), Ty::V3, vec![f(v)])
}
pub fn cast(ty: Ty, e: Expr) -> Expr {
    call(ty.text(), ty, vec![e])
}
pub fn dot(a: Expr, b: Expr) -> Expr {
    call("dot", Ty::F32, vec![a, b])
}
pub fn cross(a: Expr, b: Expr) -> Expr {
    call("cross", Ty::V3, vec![a, b])
}
pub fn norm(a: Expr) -> Expr {
    call("normalize", Ty::V3, vec![a])
}
pub fn abs(a: Expr) -> Expr {
    call("abs", a.ty, vec![a])
}
pub fn max(a: Expr, b: Expr) -> Expr {
    call("max", a.ty, vec![a, b])
}
pub fn min(a: Expr, b: Expr) -> Expr {
    call("min", a.ty, vec![a, b])
}
pub fn sqrt(a: Expr) -> Expr {
    call("sqrt", a.ty, vec![a])
}
#[derive(Clone, Debug)]
pub enum Stmt {
    /// Generation-time composition without an additional shader scope.
    Sequence(Vec<Stmt>),
    Let(String, Expr),
    Var(String, Expr),
    Assign(Expr, Expr),
    If(Expr, Vec<Stmt>, Vec<Stmt>),
    While(Expr, Vec<Stmt>),
    Return(Option<Expr>),
    Break,
}
impl Stmt {
    fn emit(&self, out: &mut String) {
        match self {
            Self::Sequence(body) => {
                for statement in body {
                    statement.emit(out);
                }
            }
            Self::Let(n, e) => out.push_str(&format!("let {n}: {} = {};\n", e.ty.text(), e.emit())),
            Self::Var(n, e) => out.push_str(&format!("var {n}: {} = {};\n", e.ty.text(), e.emit())),
            Self::Assign(a, b) => out.push_str(&format!("{} = {};\n", a.emit(), b.emit())),
            Self::If(c, a, b) => {
                out.push_str(&format!("if ({}) {{\n", c.emit()));
                for s in a {
                    s.emit(out);
                }
                out.push_str("}\n");
                if !b.is_empty() {
                    out.push_str("else {\n");
                    for s in b {
                        s.emit(out);
                    }
                    out.push_str("}\n");
                }
            }
            Self::While(c, body) => {
                out.push_str(&format!("while ({}) {{\n", c.emit()));
                for s in body {
                    s.emit(out);
                }
                out.push_str("}\n");
            }
            Self::Return(e) => out.push_str(&format!(
                "return {};\n",
                e.as_ref().map(Expr::emit).unwrap_or_default()
            )),
            Self::Break => out.push_str("break;\n"),
        }
    }
}
pub fn let_(name: &str, e: Expr) -> Stmt {
    Stmt::Let(name.into(), e)
}
pub fn var(name: &str, e: Expr) -> Stmt {
    Stmt::Var(name.into(), e)
}
pub fn set(a: Expr, b: Expr) -> Stmt {
    Stmt::Assign(a, b)
}
pub fn if_(c: Expr, body: Vec<Stmt>) -> Stmt {
    Stmt::If(c, body, vec![])
}
pub fn ret(e: Expr) -> Stmt {
    Stmt::Return(Some(e))
}
pub struct Function {
    pub name: String,
    pub args: Vec<(String, Ty)>,
    pub returns: Ty,
    pub body: Vec<Stmt>,
}
pub struct Kernel {
    pub buffers: Vec<(String, bool)>,
    pub functions: Vec<Function>,
    pub body: Vec<Stmt>,
}
impl Kernel {
    pub fn generate(&self) -> Result<String> {
        let mut out = String::from("// Generated by render-kernel Rust IR. Do not edit.\n");
        for (i, (name, write)) in self.buffers.iter().enumerate() {
            out.push_str(&format!(
                "@group(0) @binding({i}) var<storage, {}> {name}: array<vec4<f32>>;\n",
                if *write { "read_write" } else { "read" }
            ));
        }
        for function in &self.functions {
            out.push_str(&format!(
                "fn {}({}) -> {} {{\n",
                function.name,
                function
                    .args
                    .iter()
                    .map(|(n, t)| format!("{n}: {}", t.text()))
                    .collect::<Vec<_>>()
                    .join(", "),
                function.returns.text()
            ));
            for s in &function.body {
                s.emit(&mut out);
            }
            out.push_str("}\n");
        }
        out.push_str("@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) gid: vec3<u32>) {\n");
        for s in &self.body {
            s.emit(&mut out);
        }
        out.push_str("}\n");
        let module = naga::front::wgsl::parse_str(&out)
            .map_err(|e| Error::new("kernel_parse", e.emit_to_string(&out)))?;
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .map_err(|e| Error::new("kernel_validation", format!("{e:?}")))?;
        Ok(out)
    }
}
pub mod path;
/// Stage declarations are compiler output; vertex/fragment behavior uses the same IR.
pub fn raster_source() -> Result<String> {
    let mut out = String::from("// Generated raster stage interfaces from Rust.\n");
    out.push_str("@group(0) @binding(0) var<storage, read> vertices: array<vec4<f32>>;\n");
    out.push_str("struct RasterVertex {\n");
    for (attribute, name) in [
        ("@builtin(position)", "position"),
        ("@location(0)", "color"),
    ] {
        out.push_str(&format!("{attribute} {name}: {},\n", Ty::V4.text()));
    }
    out.push_str(
        "};\n@vertex fn vertex_main(@builtin(vertex_index) index: u32) -> RasterVertex {\n",
    );
    let index = Expr::var("index", Ty::U32) * u(2);
    ret(call(
        "RasterVertex",
        Ty::RasterVertex,
        vec![
            read("vertices", index.clone()),
            read("vertices", index + u(1)),
        ],
    ))
    .emit(&mut out);
    out.push_str("}\n@fragment fn fragment_main(@location(0) color: vec4<f32>) -> @location(0) vec4<f32> {\n");
    ret(Expr::var("color", Ty::V4)).emit(&mut out);
    out.push_str("}\n");
    let module = naga::front::wgsl::parse_str(&out)
        .map_err(|e| Error::new("raster_kernel", e.emit_to_string(&out)))?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .map_err(|e| Error::new("raster_kernel", format!("{e:?}")))?;
    Ok(out)
}
