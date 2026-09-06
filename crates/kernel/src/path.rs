//! Portable diffuse transport built from typed Rust IR nodes.
use super::*;
mod pbr;
fn v(n: &str) -> Expr {
    Expr::var(n, Ty::V3)
}
fn q(n: &str) -> Expr {
    Expr::var(n, Ty::V4)
}
fn s(n: &str) -> Expr {
    Expr::var(n, Ty::F32)
}
fn i(n: &str) -> Expr {
    Expr::var(n, Ty::U32)
}
fn xyz(e: Expr) -> Expr {
    e.field("xyz")
}
fn data(index: Expr) -> Expr {
    read("geometry", index)
}
fn inst(index: Expr) -> Expr {
    read("instances", index)
}
fn param(n: u32) -> Expr {
    read("params", u(n))
}
fn function(name: &str, args: &[(&str, Ty)], returns: Ty, body: Vec<Stmt>) -> Function {
    Function {
        name: name.into(),
        args: args.iter().map(|(n, t)| (n.to_string(), *t)).collect(),
        returns,
        body,
    }
}
fn trace(origin: Expr, direction: Expr, max_distance: Expr) -> Expr {
    call(
        "trace",
        Ty::V4,
        vec![origin, direction, max_distance, f(0.00001)],
    )
}
fn random(dim: u32) -> Expr {
    call(
        "random",
        Ty::F32,
        vec![
            i("pixel"),
            i("sample"),
            u(dim),
            cast(Ty::U32, param(5).field("w")) + cast(Ty::U32, param(7).field("w")) * u(65536),
        ],
    )
}
pub fn kernel() -> Kernel {
    let random_fn = function(
        "random",
        &[
            ("pixel", Ty::U32),
            ("sample", Ty::U32),
            ("dimension", Ty::U32),
            ("seed", Ty::U32),
        ],
        Ty::F32,
        vec![
            var(
                "x",
                (i("pixel") * u(747796405))
                    .xor(i("sample") * u(2891336453))
                    .xor(i("dimension") * u(277803737))
                    .xor(i("seed")),
            ),
            set(
                i("x"),
                i("x")
                    .shift_right(i("x").shift_right(u(28)) + u(4))
                    .xor(i("x"))
                    * u(277803737),
            ),
            set(i("x"), i("x").shift_right(u(22)).xor(i("x"))),
            ret((cast(Ty::F32, i("x").shift_right(u(8))) + f(0.5)) / f(16777216.)),
        ],
    );
    let transform_fn = function(
        "transform",
        &[("base", Ty::U32), ("p", Ty::V3), ("w", Ty::F32)],
        Ty::V3,
        vec![ret(xyz(inst(i("base"))) * v("p").field("x")
            + xyz(inst(i("base") + u(1))) * v("p").field("y")
            + xyz(inst(i("base") + u(2))) * v("p").field("z")
            + xyz(inst(i("base") + u(3))) * s("w"))],
    );
    let mut bounds_body = vec![var("near", f(0.00001)), var("far", s("limit"))];
    for axis in ["x", "y", "z"] {
        bounds_body.push(Stmt::If(
            v("rd").field(axis).eq(f(0.)),
            vec![if_(
                v("ro")
                    .field(axis)
                    .lt(v("low").field(axis))
                    .or(v("ro").field(axis).gt(v("high").field(axis))),
                vec![ret(boolean(false))],
            )],
            vec![
                let_(
                    "a",
                    (v("low").field(axis) - v("ro").field(axis)) / v("rd").field(axis),
                ),
                let_(
                    "b",
                    (v("high").field(axis) - v("ro").field(axis)) / v("rd").field(axis),
                ),
                set(s("near"), max(s("near"), min(s("a"), s("b")))),
                set(s("far"), min(s("far"), max(s("a"), s("b")))),
                if_(s("far").lt(s("near")), vec![ret(boolean(false))]),
            ],
        ));
    }
    bounds_body.push(ret(boolean(true)));
    let bounds_fn = function(
        "bounds_hit",
        &[
            ("ro", Ty::V3),
            ("rd", Ty::V3),
            ("low", Ty::V3),
            ("high", Ty::V3),
            ("limit", Ty::F32),
        ],
        Ty::Bool,
        bounds_body,
    );
    let tri_fn = function(
        "triangle_hit",
        &[
            ("ro", Ty::V3),
            ("rd", Ty::V3),
            ("t", Ty::U32),
            ("limit", Ty::F32),
        ],
        Ty::V3,
        vec![
            let_("a", xyz(data(i("t")))),
            let_("e1", xyz(data(i("t") + u(1))) - v("a")),
            let_("e2", xyz(data(i("t") + u(2))) - v("a")),
            let_("p", cross(v("rd"), v("e2"))),
            let_("det", dot(v("e1"), v("p"))),
            if_(
                abs(s("det")).le(f(1e-7)
                    * sqrt(dot(v("e1"), v("e1")) * dot(v("e2"), v("e2")) * dot(v("rd"), v("rd")))),
                vec![ret(splat(-1.))],
            ),
            let_("delta", v("ro") - v("a")),
            let_("ucoord", dot(v("delta"), v("p")) / s("det")),
            if_(
                s("ucoord").lt(f(0.)).or(s("ucoord").gt(f(1.))),
                vec![ret(splat(-1.))],
            ),
            let_("qvec", cross(v("delta"), v("e1"))),
            let_("vcoord", dot(v("rd"), v("qvec")) / s("det")),
            if_(
                s("vcoord")
                    .lt(f(0.))
                    .or((s("ucoord") + s("vcoord")).gt(f(1.))),
                vec![ret(splat(-1.))],
            ),
            let_("distance", dot(v("e2"), v("qvec")) / s("det")),
            if_(
                s("distance")
                    .lt(f(0.00001))
                    .or(s("distance").gt(s("limit"))),
                vec![ret(splat(-1.))],
            ),
            ret(vec3(s("distance"), s("ucoord"), s("vcoord"))),
        ],
    );
    let trace_fn = function(
        "trace",
        &[
            ("origin", Ty::V3),
            ("direction", Ty::V3),
            ("limit", Ty::F32),
            ("minimum", Ty::F32),
        ],
        Ty::V4,
        vec![
            var("closest", s("limit")),
            var("result", vec4(splat(-1.), f(-1.))),
            var("instance_index", u(0)),
            Stmt::While(
                i("instance_index").lt(cast(Ty::U32, param(1).field("w"))),
                vec![
                    let_("base", i("instance_index") * u(16)),
                    if_(
                        call(
                            "bounds_hit",
                            Ty::Bool,
                            vec![
                                v("origin"),
                                v("direction"),
                                xyz(inst(i("base") + u(4))),
                                xyz(inst(i("base") + u(5))),
                                s("closest"),
                            ],
                        ),
                        vec![
                            let_(
                                "ro",
                                call("transform", Ty::V3, vec![i("base"), v("origin"), f(1.)]),
                            ),
                            let_(
                                "rd",
                                call("transform", Ty::V3, vec![i("base"), v("direction"), f(0.)]),
                            ),
                            var("node", cast(Ty::U32, inst(i("base") + u(6)).field("w"))),
                            let_("end", cast(Ty::U32, data(i("node") + u(2)).field("x"))),
                            Stmt::While(
                                i("node").lt(i("end")),
                                vec![
                                    let_("low", data(i("node"))),
                                    let_("high", data(i("node") + u(1))),
                                    let_(
                                        "escape",
                                        cast(Ty::U32, data(i("node") + u(2)).field("x")),
                                    ),
                                    Stmt::If(
                                        call(
                                            "bounds_hit",
                                            Ty::Bool,
                                            vec![
                                                v("ro"),
                                                v("rd"),
                                                xyz(q("low")),
                                                xyz(q("high")),
                                                s("closest"),
                                            ],
                                        ),
                                        vec![Stmt::If(
                                            q("low").field("w").gt(f(0.)),
                                            vec![
                                                var("j", u(0)),
                                                Stmt::While(
                                                    i("j").lt(cast(Ty::U32, q("low").field("w"))),
                                                    vec![
                                                        let_(
                                                            "triangle",
                                                            cast(Ty::U32, q("high").field("w"))
                                                                + i("j") * u(11),
                                                        ),
                                                        let_(
                                                            "hit",
                                                            call(
                                                                "triangle_hit",
                                                                Ty::V3,
                                                                vec![
                                                                    v("ro"),
                                                                    v("rd"),
                                                                    i("triangle"),
                                                                    s("closest"),
                                                                ],
                                                            ),
                                                        ),
                                                        if_(
                                                            v("hit")
                                                                .field("x")
                                                                .ge(s("minimum"))
                                                                .and(
                                                                    inst(i("base") + u(8))
                                                                        .field("w")
                                                                        .gt(f(0.))
                                                                        .or(dot(
                                                                            cross(
                                                                                xyz(data(
                                                                                    i("triangle")
                                                                                        + u(1),
                                                                                )) - xyz(data(i(
                                                                                    "triangle",
                                                                                ))),
                                                                                xyz(data(
                                                                                    i("triangle")
                                                                                        + u(2),
                                                                                )) - xyz(data(i(
                                                                                    "triangle",
                                                                                ))),
                                                                            ),
                                                                            v("rd"),
                                                                        )
                                                                        .lt(f(0.))),
                                                                ),
                                                            vec![
                                                                set(
                                                                    s("closest"),
                                                                    v("hit").field("x"),
                                                                ),
                                                                set(
                                                                    q("result"),
                                                                    call(
                                                                        Ty::V4.text(),
                                                                        Ty::V4,
                                                                        vec![
                                                                            s("closest"),
                                                                            cast(
                                                                                Ty::F32,
                                                                                i("instance_index"),
                                                                            ),
                                                                            cast(
                                                                                Ty::F32,
                                                                                i("triangle"),
                                                                            ),
                                                                            f(0.),
                                                                        ],
                                                                    ),
                                                                ),
                                                            ],
                                                        ),
                                                        set(i("j"), i("j") + u(1)),
                                                    ],
                                                ),
                                                set(i("node"), i("escape")),
                                            ],
                                            vec![set(i("node"), i("node") + u(3))],
                                        )],
                                        vec![set(i("node"), i("escape"))],
                                    ),
                                ],
                            ),
                        ],
                    ),
                    set(i("instance_index"), i("instance_index") + u(1)),
                ],
            ),
            ret(q("result")),
        ],
    );
    let cosine_fn = function(
        "cosine_direction",
        &[("normal", Ty::V3), ("ucoord", Ty::F32), ("vcoord", Ty::F32)],
        Ty::V3,
        vec![
            var("helper", vec3(f(0.), f(0.), f(1.))),
            if_(
                abs(v("normal").field("z")).ge(f(0.999)),
                vec![set(v("helper"), vec3(f(1.), f(0.), f(0.)))],
            ),
            let_("tangent", norm(cross(v("helper"), v("normal")))),
            let_("bitangent", cross(v("normal"), v("tangent"))),
            let_("r", sqrt(s("ucoord"))),
            let_("phi", f(std::f32::consts::TAU) * s("vcoord")),
            ret(norm(
                v("tangent") * (s("r") * call("cos", Ty::F32, vec![s("phi")]))
                    + v("bitangent") * (s("r") * call("sin", Ty::F32, vec![s("phi")]))
                    + v("normal") * sqrt(f(1.) - s("ucoord")),
            )),
        ],
    );
    let mut body = vec![
        let_("pixel", Expr::var("gid", Ty::U3).field("x")),
        let_("width", cast(Ty::U32, param(0).field("x"))),
        let_("height", cast(Ty::U32, param(0).field("y"))),
        if_(
            i("pixel").ge(i("width") * i("height")),
            vec![Stmt::Return(None)],
        ),
        var("sum", splat(0.)),
        var("first_depth", f(0.)),
        var("first_normal", splat(0.)),
        var("first_instance", f(-1.)),
        var("sample", cast(Ty::U32, param(6).field("w"))),
    ];
    body.push(Stmt::While(
        i("sample").lt(cast(Ty::U32, param(6).field("w")) + cast(Ty::U32, param(0).field("z"))),
        vec![
            let_(
                "px",
                cast(Ty::F32, i("pixel").remainder(i("width"))) + random(0),
            ),
            let_("py", cast(Ty::F32, i("pixel") / i("width")) + random(1)),
            let_(
                "origin",
                call("camera_origin", Ty::V3, vec![vec2(s("px"), s("py"))]),
            ),
            let_(
                "direction",
                call("camera_direction", Ty::V3, vec![vec2(s("px"), s("py"))]),
            ),
            var("clip_factor", f(1.)),
            if_(
                param(8).field("x").eq(f(1.)),
                vec![set(
                    s("clip_factor"),
                    f(1.) / dot(v("direction"), xyz(param(2))),
                )],
            ),
            let_(
                "hit",
                call(
                    "trace",
                    Ty::V4,
                    vec![
                        v("origin"),
                        v("direction"),
                        param(9).field("y") * s("clip_factor"),
                        param(9).field("x") * s("clip_factor"),
                    ],
                ),
            ),
            Stmt::If(
                q("hit").field("x").lt(f(0.)),
                vec![set(v("sum"), v("sum") + xyz(param(7)))],
                vec![
                    let_("base", cast(Ty::U32, q("hit").field("y")) * u(16)),
                    let_("triangle", cast(Ty::U32, q("hit").field("z"))),
                    let_(
                        "position",
                        v("origin") + v("direction") * q("hit").field("x"),
                    ),
                    let_("a", xyz(data(i("triangle")))),
                    let_("e1", xyz(data(i("triangle") + u(1))) - v("a")),
                    let_("e2", xyz(data(i("triangle") + u(2))) - v("a")),
                    let_("local_normal", cross(v("e1"), v("e2"))),
                    var(
                        "normal",
                        norm(vec3(
                            dot(xyz(inst(i("base"))), v("local_normal")),
                            dot(xyz(inst(i("base") + u(1))), v("local_normal")),
                            dot(xyz(inst(i("base") + u(2))), v("local_normal")),
                        )),
                    ),
                    if_(
                        dot(v("normal"), v("direction")).gt(f(0.)),
                        vec![set(v("normal"), v("normal") * f(-1.))],
                    ),
                    if_(
                        i("sample").eq(cast(Ty::U32, param(6).field("w"))),
                        vec![
                            set(s("first_depth"), q("hit").field("x")),
                            set(v("first_normal"), v("normal")),
                            set(s("first_instance"), q("hit").field("y")),
                        ],
                    ),
                    Stmt::If(
                        inst(i("base") + u(8)).field("z").gt(f(0.)),
                        pbr::body(),
                        vec![
                            var("color", xyz(inst(i("base") + u(6)))),
                            let_("texwidth", cast(Ty::U32, inst(i("base") + u(8)).field("x"))),
                            let_(
                                "texheight",
                                cast(Ty::U32, inst(i("base") + u(8)).field("y")),
                            ),
                            if_(
                                i("texwidth").gt(u(0)),
                                vec![
                                    let_(
                                        "ro",
                                        call(
                                            "transform",
                                            Ty::V3,
                                            vec![i("base"), v("origin"), f(1.)],
                                        ),
                                    ),
                                    let_(
                                        "rd",
                                        call(
                                            "transform",
                                            Ty::V3,
                                            vec![i("base"), v("direction"), f(0.)],
                                        ),
                                    ),
                                    let_(
                                        "bary",
                                        call(
                                            "triangle_hit",
                                            Ty::V3,
                                            vec![v("ro"), v("rd"), i("triangle"), f(1e30)],
                                        ),
                                    ),
                                    let_("uvpair", data(i("triangle") + u(3))),
                                    let_("lastuv", data(i("triangle") + u(4))),
                                    let_(
                                        "uv",
                                        q("uvpair").field("xy")
                                            * (f(1.) - v("bary").field("y") - v("bary").field("z"))
                                            + q("uvpair").field("zw") * v("bary").field("y")
                                            + q("lastuv").field("xy") * v("bary").field("z"),
                                    ),
                                    let_(
                                        "tx",
                                        min(
                                            cast(
                                                Ty::U32,
                                                call(
                                                    "fract",
                                                    Ty::F32,
                                                    vec![Expr::var("uv", Ty::V2).field("x")],
                                                ) * cast(Ty::F32, i("texwidth")),
                                            ),
                                            i("texwidth") - u(1),
                                        ),
                                    ),
                                    let_(
                                        "ty",
                                        min(
                                            cast(
                                                Ty::U32,
                                                call(
                                                    "fract",
                                                    Ty::F32,
                                                    vec![Expr::var("uv", Ty::V2).field("y")],
                                                ) * cast(Ty::F32, i("texheight")),
                                            ),
                                            i("texheight") - u(1),
                                        ),
                                    ),
                                    set(
                                        v("color"),
                                        v("color")
                                            * xyz(data(
                                                cast(Ty::U32, inst(i("base") + u(7)).field("w"))
                                                    + i("ty") * i("texwidth")
                                                    + i("tx"),
                                            )),
                                    ),
                                ],
                            ),
                            set(v("sum"), v("sum") + xyz(inst(i("base") + u(7)))),
                            let_("offset", v("position") + v("normal") * f(0.00001)),
                            let_("to_light", xyz(param(5)) - v("position")),
                            let_("distance", sqrt(dot(v("to_light"), v("to_light")))),
                            let_("light_dir", v("to_light") / s("distance")),
                            let_("cosine", max(dot(v("normal"), v("light_dir")), f(0.))),
                            if_(
                                s("cosine").gt(f(0.)).and(s("distance").gt(f(0.00001))),
                                vec![if_(
                                    trace(v("offset"), v("light_dir"), s("distance") - f(0.00002))
                                        .field("x")
                                        .lt(f(0.)),
                                    vec![set(
                                        v("sum"),
                                        v("sum")
                                            + v("color")
                                                * xyz(param(6))
                                                * (s("cosine")
                                                    / (f(std::f32::consts::PI)
                                                        * s("distance")
                                                        * s("distance"))),
                                    )],
                                )],
                            ),
                            let_(
                                "secondary",
                                call(
                                    "cosine_direction",
                                    Ty::V3,
                                    vec![v("normal"), random(2), random(3)],
                                ),
                            ),
                            if_(
                                trace(v("offset"), v("secondary"), f(1e30))
                                    .field("x")
                                    .lt(f(0.)),
                                vec![set(v("sum"), v("sum") + v("color") * xyz(param(7)))],
                            ),
                        ],
                    ),
                ],
            ),
            set(i("sample"), i("sample") + u(1)),
        ],
    ));
    body.push(set(
        read("output", i("pixel") * u(2)),
        vec4(v("sum") / param(0).field("z"), s("first_depth")),
    ));
    body.push(set(
        read("output", i("pixel") * u(2) + u(1)),
        vec4(v("first_normal"), s("first_instance")),
    ));
    Kernel {
        buffers: vec![
            ("geometry".into(), false),
            ("instances".into(), false),
            ("params".into(), false),
            ("output".into(), true),
            ("texels".into(), false),
        ],
        functions: {
            let mut functions = vec![
                random_fn,
                transform_fn,
                bounds_fn,
                tri_fn,
                trace_fn,
                cosine_fn,
            ];
            functions.extend(pbr::functions());
            functions
        },
        body,
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn generated_traversal_and_shading_validate() {
        let source = super::kernel().generate().unwrap();
        let module = naga::front::wgsl::parse_str(&source).unwrap();
        assert_eq!(module.global_variables.len(), 5);
        assert_eq!(module.entry_points[0].workgroup_size, [64, 1, 1]);
        for name in ["trace", "triangle_hit", "cosine_direction", "random"] {
            assert!(
                module
                    .functions
                    .iter()
                    .any(|(_, f)| f.name.as_deref() == Some(name))
            );
        }
    }
}
