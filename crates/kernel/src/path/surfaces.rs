//! Existing typed conductor and coat semantics, lowered into private Rust IR.
use super::*;
fn pow(x: Expr, p: f32) -> Expr {
    call("pow", Ty::F32, vec![x, f(p)])
}
fn scalar_vector(x: Expr) -> Expr {
    call(Ty::V3.text(), Ty::V3, vec![x])
}
fn frame_args() -> Vec<Expr> {
    vec![v("n"), v("view"), v("light")]
}
fn base_brdf() -> Expr {
    call(
        "brdf",
        Ty::V3,
        vec![
            v("color"),
            s("metal"),
            s("rough"),
            v("n"),
            v("view"),
            v("light"),
        ],
    )
}
fn fresnel(c: Expr) -> Expr {
    call("surface_fresnel", Ty::F32, vec![c, q("model").field("z")])
}
pub(super) fn functions() -> Vec<Function> {
    let frame = [("n", Ty::V3), ("view", Ty::V3), ("light", Ty::V3)];
    let mut ggx_args = vec![("fresnel", Ty::V3), ("rough", Ty::F32)];
    ggx_args.extend(frame);
    let mut pdf_args = frame.to_vec();
    pdf_args.push(("rough", Ty::F32));
    let mut brdf_args = vec![("color", Ty::V3), ("metal", Ty::F32), ("rough", Ty::F32)];
    brdf_args.extend(frame);
    brdf_args.push(("base", Ty::U32));
    let mut surface_pdf_args = pdf_args.clone();
    surface_pdf_args.push(("base", Ty::U32));
    vec![
        function(
            "surface_model",
            &[("base", Ty::U32)],
            Ty::V4,
            vec![
                let_("pointer", inst(i("base") + u(15)).field("w")),
                if_(
                    s("pointer").eq(f(0.)),
                    vec![ret(call(Ty::V4.text(), Ty::V4, vec![f(0.)]))],
                ),
                ret(data(cast(Ty::U32, s("pointer")) - u(1))),
            ],
        ),
        function(
            "surface_fresnel",
            &[("cosine", Ty::F32), ("ior", Ty::F32)],
            Ty::F32,
            vec![
                if_(s("ior").eq(f(1.)), vec![ret(f(0.))]),
                let_("c", min(max(abs(s("cosine")), f(0.)), f(1.))),
                let_("sin2", (f(1.) - s("c") * s("c")) / (s("ior") * s("ior"))),
                if_(s("sin2").ge(f(1.)), vec![ret(f(1.))]),
                let_("ct", sqrt(f(1.) - s("sin2"))),
                let_(
                    "rs",
                    (s("c") - s("ior") * s("ct")) / (s("c") + s("ior") * s("ct")),
                ),
                let_(
                    "rp",
                    (s("ior") * s("c") - s("ct")) / (s("ior") * s("c") + s("ct")),
                ),
                ret(f(0.5) * (s("rs") * s("rs") + s("rp") * s("rp"))),
            ],
        ),
        function(
            "surface_ggx",
            &ggx_args,
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
                let_("a2", pow(max(s("rough"), f(0.05)), 4.)),
                let_(
                    "vis",
                    f(1.)
                        / ((s("nl") + sqrt(s("a2") + (f(1.) - s("a2")) * s("nl") * s("nl")))
                            * (s("nv") + sqrt(s("a2") + (f(1.) - s("a2")) * s("nv") * s("nv")))),
                ),
                ret(v("fresnel")
                    * call(
                        "ggx",
                        Ty::F32,
                        vec![max(dot(v("n"), v("h")), f(0.)), s("rough")],
                    )
                    * s("vis")),
            ],
        ),
        function(
            "surface_ggx_pdf",
            &pdf_args,
            Ty::F32,
            vec![
                if_(
                    dot(v("n"), v("view"))
                        .le(f(0.))
                        .or(dot(v("n"), v("light")).le(f(0.)))
                        .or(dot(v("view") + v("light"), v("view") + v("light")).lt(f(1e-20))),
                    vec![ret(f(0.))],
                ),
                let_("h", norm(v("view") + v("light"))),
                ret(call("ggx", Ty::F32, vec![dot(v("n"), v("h")), s("rough")])
                    * max(dot(v("n"), v("h")), f(0.))
                    / (f(4.) * abs(dot(v("view"), v("h"))))),
            ],
        ),
        function(
            "surface_brdf",
            &brdf_args,
            Ty::V3,
            vec![
                let_("model", call("surface_model", Ty::V4, vec![i("base")])),
                if_(q("model").field("x").eq(f(0.)), vec![ret(base_brdf())]),
                if_(
                    dot(v("view") + v("light"), v("view") + v("light")).lt(f(1e-20)),
                    vec![ret(splat(0.))],
                ),
                let_("h", norm(v("view") + v("light"))),
                if_(
                    q("model").field("x").eq(f(1.)),
                    vec![
                        let_(
                            "f0",
                            xyz(data(cast(Ty::U32, inst(i("base") + u(15)).field("w")))),
                        ),
                        let_(
                            "fresnel",
                            v("f0")
                                + (splat(1.) - v("f0"))
                                    * pow(
                                        f(1.) - min(max(dot(v("view"), v("h")), f(0.)), f(1.)),
                                        5.,
                                    ),
                        ),
                        ret(call("surface_ggx", Ty::V3, {
                            let mut a = vec![v("fresnel"), s("rough")];
                            a.extend(frame_args());
                            a
                        })),
                    ],
                ),
                let_("weight", q("model").field("y")),
                ret(base_brdf()
                    * (f(1.) - s("weight") * fresnel(dot(v("n"), v("view"))))
                    * (f(1.) - s("weight") * fresnel(dot(v("n"), v("light"))))
                    + call("surface_ggx", Ty::V3, {
                        let mut a = vec![
                            scalar_vector(s("weight") * fresnel(dot(v("view"), v("h")))),
                            q("model").field("w"),
                        ];
                        a.extend(frame_args());
                        a
                    })),
            ],
        ),
        function(
            "surface_pdf",
            &surface_pdf_args,
            Ty::F32,
            vec![
                let_("model", call("surface_model", Ty::V4, vec![i("base")])),
                if_(
                    q("model").field("x").eq(f(1.)),
                    vec![ret(call("surface_ggx_pdf", Ty::F32, {
                        let mut a = frame_args();
                        a.push(s("rough"));
                        a
                    }))],
                ),
                let_(
                    "base_pdf",
                    call("pbr_pdf", Ty::F32, {
                        let mut a = frame_args();
                        a.push(s("rough"));
                        a
                    }),
                ),
                if_(q("model").field("x").eq(f(0.)), vec![ret(s("base_pdf"))]),
                let_("q", q("model").field("y") * f(0.5)),
                ret((f(1.) - s("q")) * s("base_pdf")
                    + s("q")
                        * call("surface_ggx_pdf", Ty::F32, {
                            let mut a = frame_args();
                            a.push(q("model").field("w"));
                            a
                        })),
            ],
        ),
        function(
            "surface_sample",
            &[
                ("n", Ty::V3),
                ("view", Ty::V3),
                ("rough", Ty::F32),
                ("selector", Ty::F32),
                ("ucoord", Ty::F32),
                ("vcoord", Ty::F32),
                ("base", Ty::U32),
                ("coat_selector", Ty::F32),
            ],
            Ty::V3,
            vec![
                let_("model", call("surface_model", Ty::V4, vec![i("base")])),
                var("r", s("rough")),
                var("choice", s("selector")),
                if_(
                    q("model").field("x").eq(f(1.)),
                    vec![set(s("choice"), f(1.))],
                ),
                if_(
                    q("model")
                        .field("x")
                        .eq(f(2.))
                        .and(s("coat_selector").lt(q("model").field("y") * f(0.5))),
                    vec![set(s("r"), q("model").field("w")), set(s("choice"), f(1.))],
                ),
                ret(call(
                    "pbr_sample",
                    Ty::V3,
                    vec![
                        v("n"),
                        v("view"),
                        s("r"),
                        s("choice"),
                        s("ucoord"),
                        s("vcoord"),
                    ],
                )),
            ],
        ),
    ]
}
