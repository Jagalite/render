//! Texturing, camera projection and GGX built as typed Rust IR.
use super::*;
fn vv(n: &str) -> Expr {
    Expr::var(n, Ty::V2)
}
fn floor(x: Expr) -> Expr {
    call("floor", x.ty, vec![x])
}
fn clamp(x: Expr, a: Expr, b: Expr) -> Expr {
    min(max(x, a), b)
}
fn mix(a: Expr, b: Expr, t: Expr) -> Expr {
    a.clone() + (b - a) * t
}
fn pow(x: Expr, p: f32) -> Expr {
    call("pow", Ty::F32, vec![x, f(p)])
}
fn length(x: Expr) -> Expr {
    sqrt(dot(x.clone(), x))
}
fn sample_map(slot: u32) -> Expr {
    call(
        "sample_map",
        Ty::V4,
        vec![
            inst(i("base") + u(10 + slot)).field("x"),
            vv("uv"),
            vv("dx"),
            vv("dy"),
        ],
    )
}
pub(super) fn functions() -> Vec<Function> {
    let mut result = vec![
        function(
            "camera_origin",
            &[("pixel_uv", Ty::V2)],
            Ty::V3,
            vec![
                if_(
                    param(8).field("x").eq(f(2.)),
                    vec![ret(xyz(param(3))
                        * (f(2.) * vv("pixel_uv").field("x") / param(0).field("x")
                            - f(1.))
                        * param(8).field("y")
                        + xyz(param(4))
                            * (f(1.)
                                - f(2.) * vv("pixel_uv").field("y")
                                    / param(0).field("y"))
                            * param(8).field("z"))],
                ),
                ret(splat(0.)),
            ],
        ),
        function(
            "camera_direction",
            &[("pixel_uv", Ty::V2)],
            Ty::V3,
            vec![
                if_(param(8).field("x").eq(f(2.)), vec![ret(xyz(param(2)))]),
                let_(
                    "sx",
                    (f(2.) * vv("pixel_uv").field("x") / param(0).field("x") - f(1.))
                        * param(8).field("w")
                        * param(2).field("w"),
                ),
                let_(
                    "sy",
                    (f(1.) - f(2.) * vv("pixel_uv").field("y") / param(0).field("y"))
                        * param(2).field("w"),
                ),
                ret(norm(
                    xyz(param(2)) + xyz(param(3)) * s("sx") + xyz(param(4)) * s("sy"),
                )),
            ],
        ),
        function(
            "wrap_coord",
            &[("x", Ty::F32), ("size", Ty::F32), ("mode", Ty::F32)],
            Ty::F32,
            vec![
                if_(
                    s("mode").eq(f(1.)),
                    vec![ret(clamp(s("x"), f(0.), s("size") - f(1.)))],
                ),
                if_(
                    s("mode").eq(f(2.)),
                    vec![
                        let_(
                            "n",
                            s("x") - floor(s("x") / (f(2.) * s("size"))) * (f(2.) * s("size")),
                        ),
                        ret(min(s("n"), f(2.) * s("size") - f(1.) - s("n"))),
                    ],
                ),
                ret(s("x") - floor(s("x") / s("size")) * s("size")),
            ],
        ),
        function(
            "texel",
            &[("level", Ty::V4), ("wrap", Ty::V2), ("xy", Ty::V2)],
            Ty::V4,
            vec![
                let_(
                    "x",
                    call(
                        "wrap_coord",
                        Ty::F32,
                        vec![
                            vv("xy").field("x"),
                            q("level").field("y"),
                            vv("wrap").field("x"),
                        ],
                    ),
                ),
                let_(
                    "y",
                    call(
                        "wrap_coord",
                        Ty::F32,
                        vec![
                            vv("xy").field("y"),
                            q("level").field("z"),
                            vv("wrap").field("y"),
                        ],
                    ),
                ),
                let_(
                    "address",
                    cast(Ty::U32, q("level").field("x"))
                        + cast(Ty::U32, s("y")) * cast(Ty::U32, q("level").field("y"))
                        + cast(Ty::U32, s("x")),
                ),
                let_("record", read("texels", i("address") / u(2))),
                var("pair", q("record").field("xy")),
                if_(
                    i("address").remainder(u(2)).eq(u(1)),
                    vec![set(vv("pair"), q("record").field("zw"))],
                ),
                ret(call(
                    Ty::V4.text(),
                    Ty::V4,
                    vec![
                        call(
                            "unpack2x16float",
                            Ty::V2,
                            vec![call("bitcast<u32>", Ty::U32, vec![vv("pair").field("x")])],
                        ),
                        call(
                            "unpack2x16float",
                            Ty::V2,
                            vec![call("bitcast<u32>", Ty::U32, vec![vv("pair").field("y")])],
                        ),
                    ],
                )),
            ],
        ),
    ];
    let texel = |xy| call("texel", Ty::V4, vec![q("level"), vv("wrap"), xy]);
    let mut body = vec![var("coord", vv("uv"))];
    for axis in ["x", "y"] {
        body.push(Stmt::If(
            vv("wrap").field(axis).eq(f(1.)),
            vec![set(
                vv("coord").field(axis),
                clamp(vv("coord").field(axis), f(0.), f(1.)),
            )],
            vec![
                var("period", f(1.)),
                if_(
                    vv("wrap").field(axis).eq(f(2.)),
                    vec![set(s("period"), f(2.))],
                ),
                set(
                    vv("coord").field(axis),
                    vv("coord").field(axis)
                        - floor(vv("coord").field(axis) / s("period")) * s("period"),
                ),
            ],
        ));
    }
    body.extend([
        set(vv("coord"), vv("coord") * q("level").field("yz")),
        if_(s("linear").eq(f(0.)), vec![ret(texel(floor(vv("coord"))))]),
        set(vv("coord"), vv("coord") - vec2(f(0.5), f(0.5))),
        let_("xy", floor(vv("coord"))),
        let_("fraction", vv("coord") - vv("xy")),
        ret(mix(
            mix(
                texel(vv("xy")),
                texel(vv("xy") + vec2(f(1.), f(0.))),
                vv("fraction").field("x"),
            ),
            mix(
                texel(vv("xy") + vec2(f(0.), f(1.))),
                texel(vv("xy") + vec2(f(1.), f(1.))),
                vv("fraction").field("x"),
            ),
            vv("fraction").field("y"),
        )),
    ]);
    result.push(function(
        "sample_level",
        &[
            ("level", Ty::V4),
            ("wrap", Ty::V2),
            ("uv", Ty::V2),
            ("linear", Ty::F32),
        ],
        Ty::V4,
        body,
    ));
    let level = |lod: Expr, filter: Expr| {
        call(
            "sample_level",
            Ty::V4,
            vec![
                data(cast(Ty::U32, q("desc").field("x")) + cast(Ty::U32, lod)),
                q("desc").field("zw"),
                vv("uv"),
                filter,
            ],
        )
    };
    result.push(function(
        "sample_map",
        &[
            ("descriptor", Ty::F32),
            ("uv", Ty::V2),
            ("dx", Ty::V2),
            ("dy", Ty::V2),
        ],
        Ty::V4,
        vec![
            if_(s("descriptor").lt(f(0.)), vec![ret(vec4(splat(1.), f(1.)))]),
            let_("desc", data(cast(Ty::U32, s("descriptor")))),
            let_("filters", data(cast(Ty::U32, s("descriptor")) + u(1))),
            let_(
                "dims",
                data(cast(Ty::U32, q("desc").field("x"))).field("yz"),
            ),
            var(
                "lod",
                call(
                    "log2",
                    Ty::F32,
                    vec![max(
                        max(length(vv("dx") * vv("dims")), length(vv("dy") * vv("dims"))),
                        f(1e-8),
                    )],
                ),
            ),
            if_(
                s("lod").le(f(0.)),
                vec![ret(level(f(0.), q("filters").field("x")))],
            ),
            let_("mode", q("filters").field("y")),
            let_("linear", s("mode") - floor(s("mode") / f(2.)) * f(2.)),
            if_(s("mode").lt(f(2.)), vec![ret(level(f(0.), s("linear")))]),
            set(
                s("lod"),
                clamp(s("lod"), f(0.), q("desc").field("y") - f(1.)),
            ),
            if_(
                s("mode").lt(f(4.)),
                vec![ret(level(floor(s("lod") + f(0.5)), s("linear")))],
            ),
            let_("lo", floor(s("lod"))),
            ret(mix(
                level(s("lo"), s("linear")),
                level(
                    min(s("lo") + f(1.), q("desc").field("y") - f(1.)),
                    s("linear"),
                ),
                s("lod") - s("lo"),
            )),
        ],
    ));
    result.push(function(
        "project_bary",
        &[
            ("base", Ty::U32),
            ("triangle", Ty::U32),
            ("origin", Ty::V3),
            ("direction", Ty::V3),
        ],
        Ty::V3,
        vec![
            let_(
                "ro",
                call("transform", Ty::V3, vec![i("base"), v("origin"), f(1.)]),
            ),
            let_(
                "rd",
                call("transform", Ty::V3, vec![i("base"), v("direction"), f(0.)]),
            ),
            let_("a", xyz(data(i("triangle")))),
            let_("e1", xyz(data(i("triangle") + u(1))) - v("a")),
            let_("e2", xyz(data(i("triangle") + u(2))) - v("a")),
            let_("p", cross(v("rd"), v("e2"))),
            let_("det", dot(v("e1"), v("p"))),
            if_(
                abs(s("det")).lt(f(1e-20)),
                vec![ret(vec3(f(1.), f(0.), f(0.)))],
            ),
            let_("delta", v("ro") - v("a")),
            let_("b", dot(v("delta"), v("p")) / s("det")),
            let_("c", dot(v("rd"), cross(v("delta"), v("e1"))) / s("det")),
            ret(vec3(f(1.) - s("b") - s("c"), s("b"), s("c"))),
        ],
    ));
    result.push(function(
        "bary_uv",
        &[("triangle", Ty::U32), ("bary", Ty::V3)],
        Ty::V2,
        vec![ret(data(i("triangle") + u(3)).field("xy")
            * v("bary").field("x")
            + data(i("triangle") + u(3)).field("zw")
                * v("bary").field("y")
            + data(i("triangle") + u(4)).field("xy")
                * v("bary").field("z"))],
    ));
    result.push(function(
        "normal_world",
        &[("base", Ty::U32), ("n", Ty::V3)],
        Ty::V3,
        vec![ret(norm(vec3(
            dot(xyz(inst(i("base"))), v("n")),
            dot(xyz(inst(i("base") + u(1))), v("n")),
            dot(xyz(inst(i("base") + u(2))), v("n")),
        )))],
    ));
    result.push(function(
        "vector_world",
        &[("base", Ty::U32), ("t", Ty::V3)],
        Ty::V3,
        vec![
            let_("a", xyz(inst(i("base")))),
            let_("b", xyz(inst(i("base") + u(1)))),
            let_("c", xyz(inst(i("base") + u(2)))),
            ret(vec3(
                dot(cross(v("b"), v("c")), v("t")),
                dot(cross(v("c"), v("a")), v("t")),
                dot(cross(v("a"), v("b")), v("t")),
            ) / dot(v("a"), cross(v("b"), v("c")))),
        ],
    ));
    result.push(function(
        "basis_tangent",
        &[("n", Ty::V3)],
        Ty::V3,
        vec![
            var("helper", vec3(f(0.), f(0.), f(1.))),
            if_(
                abs(v("n").field("z")).ge(f(0.999)),
                vec![set(v("helper"), vec3(f(1.), f(0.), f(0.)))],
            ),
            ret(norm(cross(v("helper"), v("n")))),
        ],
    ));
    result.push(function(
        "ggx",
        &[("nh", Ty::F32), ("rough", Ty::F32)],
        Ty::F32,
        vec![
            let_("a2", pow(max(s("rough"), f(0.05)), 4.)),
            let_("d", s("nh") * s("nh") * (s("a2") - f(1.)) + f(1.)),
            ret(s("a2") / (f(std::f32::consts::PI) * s("d") * s("d"))),
        ],
    ));
    result.push(function(
        "brdf",
        &[
            ("color", Ty::V3),
            ("metal", Ty::F32),
            ("rough", Ty::F32),
            ("n", Ty::V3),
            ("view", Ty::V3),
            ("light", Ty::V3),
        ],
        Ty::V3,
        vec![
            let_("nv", dot(v("n"), v("view"))),
            let_("nl", dot(v("n"), v("light"))),
            if_(
                s("nv").le(f(0.)).or(s("nl").le(f(0.))).or(dot(
                    v("view") + v("light"),
                    v("view") + v("light"),
                )
                .lt(f(1e-20))),
                vec![ret(splat(0.))],
            ),
            let_("h", norm(v("view") + v("light"))),
            let_("f0", mix(splat(0.04), v("color"), s("metal"))),
            let_(
                "fresnel",
                v("f0")
                    + (splat(1.) - v("f0"))
                        * pow(f(1.) - clamp(dot(v("view"), v("h")), f(0.), f(1.)), 5.),
            ),
            let_("a2", pow(max(s("rough"), f(0.05)), 4.)),
            let_(
                "vis",
                f(1.)
                    / ((s("nl") + sqrt(s("a2") + (f(1.) - s("a2")) * s("nl") * s("nl")))
                        * (s("nv") + sqrt(s("a2") + (f(1.) - s("a2")) * s("nv") * s("nv")))),
            ),
            ret((splat(1.) - v("fresnel"))
                * v("color")
                * ((f(1.) - s("metal")) / f(std::f32::consts::PI))
                + v("fresnel")
                    * call(
                        "ggx",
                        Ty::F32,
                        vec![max(dot(v("n"), v("h")), f(0.)), s("rough")],
                    )
                    * s("vis")),
        ],
    ));
    result.push(function(
        "pbr_pdf",
        &[
            ("n", Ty::V3),
            ("view", Ty::V3),
            ("light", Ty::V3),
            ("rough", Ty::F32),
        ],
        Ty::F32,
        vec![
            if_(
                dot(v("n"), v("light"))
                    .le(f(0.))
                    .or(dot(v("n"), v("view")).le(f(0.)))
                    .or(dot(v("view") + v("light"), v("view") + v("light")).lt(f(1e-20))),
                vec![ret(f(0.))],
            ),
            let_("h", norm(v("view") + v("light"))),
            let_("vh", dot(v("view"), v("h"))),
            if_(s("vh").le(f(0.)), vec![ret(f(0.))]),
            ret(f(0.5) * dot(v("n"), v("light")) / f(std::f32::consts::PI)
                + f(0.5)
                    * call(
                        "ggx",
                        Ty::F32,
                        vec![max(dot(v("n"), v("h")), f(0.)), s("rough")],
                    )
                    * max(dot(v("n"), v("h")), f(0.))
                    / (f(4.) * s("vh"))),
        ],
    ));
    result.push(function(
        "pbr_sample",
        &[
            ("n", Ty::V3),
            ("view", Ty::V3),
            ("rough", Ty::F32),
            ("selector", Ty::F32),
            ("ucoord", Ty::F32),
            ("vcoord", Ty::F32),
        ],
        Ty::V3,
        vec![
            if_(
                s("selector").lt(f(0.5)),
                vec![ret(call(
                    "cosine_direction",
                    Ty::V3,
                    vec![v("n"), s("ucoord"), s("vcoord")],
                ))],
            ),
            let_(
                "c",
                sqrt(
                    (f(1.) - s("ucoord"))
                        / (f(1.) + (pow(max(s("rough"), f(0.05)), 4.) - f(1.)) * s("ucoord")),
                ),
            ),
            let_("sine", sqrt(max(f(1.) - s("c") * s("c"), f(0.)))),
            let_("phi", f(std::f32::consts::TAU) * s("vcoord")),
            let_("t", call("basis_tangent", Ty::V3, vec![v("n")])),
            let_(
                "h",
                norm(
                    v("t") * (s("sine") * call("cos", Ty::F32, vec![s("phi")]))
                        + cross(v("n"), v("t"))
                            * (s("sine") * call("sin", Ty::F32, vec![s("phi")]))
                        + v("n") * s("c"),
                ),
            ),
            ret(norm(v("h") * (f(2.) * dot(v("view"), v("h"))) - v("view"))),
        ],
    ));
    result
}
pub(super) fn body() -> Vec<Stmt> {
    let bary = |origin, direction| {
        call(
            "project_bary",
            Ty::V3,
            vec![i("base"), i("triangle"), origin, direction],
        )
    };
    let uv = |b| call("bary_uv", Ty::V2, vec![i("triangle"), b]);
    let ray = |name, px, py| call(name, Ty::V3, vec![vec2(px, py)]);
    let brdf = |l| {
        call(
            "brdf",
            Ty::V3,
            vec![
                v("color"),
                s("metal"),
                s("rough"),
                v("normal"),
                v("view"),
                l,
            ],
        )
    };
    let mut out = vec![
        let_("geometric", v("normal")),
        let_(
            "back",
            dot(
                call("normal_world", Ty::V3, vec![i("base"), v("local_normal")]),
                v("direction"),
            )
            .gt(f(0.)),
        ),
        let_("bary", bary(v("origin"), v("direction"))),
        let_("uv", uv(v("bary"))),
        let_(
            "dx",
            uv(bary(
                ray("camera_origin", s("px") + f(1.), s("py")),
                ray("camera_direction", s("px") + f(1.), s("py")),
            )) - vv("uv"),
        ),
        let_(
            "dy",
            uv(bary(
                ray("camera_origin", s("px"), s("py") + f(1.)),
                ray("camera_direction", s("px"), s("py") + f(1.)),
            )) - vv("uv"),
        ),
        var("local_shading", norm(v("local_normal"))),
        if_(
            data(i("triangle") + u(4)).field("z").gt(f(0.)),
            vec![
                let_(
                    "interpolated",
                    xyz(data(i("triangle") + u(5))) * v("bary").field("x")
                        + xyz(data(i("triangle") + u(6))) * v("bary").field("y")
                        + xyz(data(i("triangle") + u(7))) * v("bary").field("z"),
                ),
                if_(
                    dot(v("interpolated"), v("interpolated")).gt(f(1e-20)),
                    vec![set(v("local_shading"), norm(v("interpolated")))],
                ),
            ],
        ),
        set(
            v("normal"),
            call("normal_world", Ty::V3, vec![i("base"), v("local_shading")]),
        ),
        if_(
            dot(v("normal"), v("geometric")).lt(f(0.)),
            vec![set(v("normal"), v("normal") * f(-1.))],
        ),
        var(
            "local_tangent",
            call("basis_tangent", Ty::V3, vec![v("local_shading")]),
        ),
        var("hand", f(1.)),
        Stmt::If(
            data(i("triangle") + u(4)).field("w").gt(f(0.)),
            vec![
                let_(
                    "t",
                    data(i("triangle") + u(8)) * v("bary").field("x")
                        + data(i("triangle") + u(9)) * v("bary").field("y")
                        + data(i("triangle") + u(10)) * v("bary").field("z"),
                ),
                set(v("local_tangent"), xyz(q("t"))),
                if_(q("t").field("w").lt(f(0.)), vec![set(s("hand"), f(-1.))]),
            ],
            vec![
                let_(
                    "ua",
                    data(i("triangle") + u(3)).field("zw") - data(i("triangle") + u(3)).field("xy"),
                ),
                let_(
                    "ub",
                    data(i("triangle") + u(4)).field("xy") - data(i("triangle") + u(3)).field("xy"),
                ),
                let_(
                    "det",
                    vv("ua").field("x") * vv("ub").field("y")
                        - vv("ua").field("y") * vv("ub").field("x"),
                ),
                if_(
                    abs(s("det")).gt(f(1e-12)),
                    vec![
                        set(
                            v("local_tangent"),
                            (v("e1") * vv("ub").field("y") - v("e2") * vv("ua").field("y"))
                                / s("det"),
                        ),
                        let_(
                            "bt",
                            (v("e2") * vv("ua").field("x") - v("e1") * vv("ub").field("x"))
                                / s("det"),
                        ),
                        if_(
                            dot(cross(v("local_shading"), v("local_tangent")), v("bt")).lt(f(0.)),
                            vec![set(s("hand"), f(-1.))],
                        ),
                    ],
                ),
            ],
        ),
        var(
            "tangent",
            call("vector_world", Ty::V3, vec![i("base"), v("local_tangent")]),
        ),
        set(
            v("tangent"),
            v("tangent") - v("normal") * dot(v("normal"), v("tangent")),
        ),
        if_(
            dot(v("tangent"), v("tangent")).lt(f(1e-20)),
            vec![set(
                v("tangent"),
                call("basis_tangent", Ty::V3, vec![v("normal")]),
            )],
        ),
        set(v("tangent"), norm(v("tangent"))),
        if_(
            Expr::var("back", Ty::Bool),
            vec![
                set(v("tangent"), v("tangent") * f(-1.)),
                set(s("hand"), s("hand") * f(-1.)),
            ],
        ),
        let_(
            "bitangent",
            cross(v("normal"), v("tangent")) * s("hand") * inst(i("base") + u(3)).field("w"),
        ),
        let_("color", xyz(inst(i("base") + u(6))) * xyz(sample_map(0))),
        let_("orm", sample_map(1)),
        let_(
            "metal",
            inst(i("base") + u(9)).field("y") * q("orm").field("z"),
        ),
        let_(
            "rough",
            inst(i("base") + u(9)).field("x") * q("orm").field("y"),
        ),
        if_(
            inst(i("base") + u(12)).field("x").ge(f(0.)),
            vec![
                let_("map", xyz(sample_map(2)) * f(2.) - splat(1.)),
                let_(
                    "mapped",
                    v("tangent") * v("map").field("x") * inst(i("base") + u(9)).field("z")
                        + v("bitangent") * v("map").field("y") * inst(i("base") + u(9)).field("z")
                        + v("normal") * v("map").field("z"),
                ),
                if_(
                    dot(v("mapped"), v("mapped")).gt(f(1e-20)),
                    vec![set(v("normal"), norm(v("mapped")))],
                ),
            ],
        ),
        if_(
            i("sample").eq(cast(Ty::U32, param(6).field("w"))),
            vec![set(v("first_normal"), v("normal"))],
        ),
        set(
            v("sum"),
            v("sum") + xyz(inst(i("base") + u(7))) * xyz(sample_map(3)),
        ),
        let_("offset", v("position") + v("geometric") * f(1e-5)),
        let_("view", v("direction") * f(-1.)),
        let_("to_light", xyz(param(5)) - v("position")),
        let_("distance", length(v("to_light"))),
        if_(
            s("distance").gt(f(1e-5)),
            vec![
                let_("light_dir", v("to_light") / s("distance")),
                let_("cosine", dot(v("normal"), v("light_dir"))),
                if_(
                    s("cosine")
                        .gt(f(0.))
                        .and(dot(v("geometric"), v("light_dir")).gt(f(0.))),
                    vec![if_(
                        trace(v("offset"), v("light_dir"), s("distance") - f(2e-5))
                            .field("x")
                            .lt(f(0.)),
                        vec![set(
                            v("sum"),
                            v("sum")
                                + brdf(v("light_dir"))
                                    * xyz(param(6))
                                    * (s("cosine") / (s("distance") * s("distance"))),
                        )],
                    )],
                ),
            ],
        ),
        let_(
            "secondary",
            call(
                "pbr_sample",
                Ty::V3,
                vec![
                    v("normal"),
                    v("view"),
                    s("rough"),
                    random(4),
                    random(2),
                    random(3),
                ],
            ),
        ),
        let_(
            "pdf",
            call(
                "pbr_pdf",
                Ty::F32,
                vec![v("normal"), v("view"), v("secondary"), s("rough")],
            ),
        ),
        if_(
            s("pdf")
                .gt(f(0.))
                .and(dot(v("geometric"), v("secondary")).gt(f(0.))),
            vec![if_(
                trace(v("offset"), v("secondary"), f(1e30))
                    .field("x")
                    .lt(f(0.)),
                vec![
                    let_(
                        "occlusion",
                        f(1.)
                            + inst(i("base") + u(9)).field("w")
                                * (sample_map(4).field("x") - f(1.)),
                    ),
                    set(
                        v("sum"),
                        v("sum")
                            + brdf(v("secondary"))
                                * xyz(param(7))
                                * (dot(v("normal"), v("secondary")) * s("occlusion") / s("pdf")),
                    ),
                ],
            )],
        ),
    ];
    out.shrink_to_fit();
    out
}
