//! Reviewed operation metadata is independent of serialized private Rust layouts.
use serde::Serialize;
#[derive(Serialize)]
pub struct Operation {
    pub name: &'static str,
    pub version: u32,
    pub mutation: bool,
    pub effects: &'static str,
    pub cancellation: &'static str,
}
pub fn registry() -> Vec<Operation> {
    vec![
        Operation {
            name: "inspect",
            version: 0,
            mutation: false,
            effects: "read a pinned snapshot and revision",
            cancellation: "immediate",
        },
        Operation {
            name: "put_mesh",
            version: 0,
            mutation: true,
            effects: "insert validated immutable geometry by SHA-256",
            cancellation: "before commit",
        },
        Operation {
            name: "put_material",
            version: 0,
            mutation: true,
            effects: "replace explicitly addressed material",
            cancellation: "before commit",
        },
        Operation {
            name: "create_entity",
            version: 0,
            mutation: true,
            effects: "insert entity with persistent ID and explicit references",
            cancellation: "before commit",
        },
        Operation {
            name: "rename",
            version: 0,
            mutation: true,
            effects: "replace entity label; preserve identity",
            cancellation: "before commit",
        },
        Operation {
            name: "reparent",
            version: 0,
            mutation: true,
            effects: "replace parent; preserve local authored transform; reject cycles",
            cancellation: "before commit",
        },
        Operation {
            name: "set_transform",
            version: 0,
            mutation: true,
            effects: "replace ordered transform channels and affine base",
            cancellation: "before commit",
        },
        Operation {
            name: "set_material",
            version: 0,
            mutation: true,
            effects: "replace entity material reference",
            cancellation: "before commit",
        },
        Operation {
            name: "set_material_scalar",
            version: 0,
            mutation: true,
            effects: "set roughness or metallic in [0,1]; renderer profile checked separately",
            cancellation: "before commit",
        },
        Operation {
            name: "put_layer",
            version: 0,
            mutation: true,
            effects: "replace or append named sparse override layer",
            cancellation: "before commit",
        },
    ]
}
