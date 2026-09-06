//! Ideal geometric-interface radiance transport, expressed as Rust kernel IR.
use super::*;
pub(super) fn functions() -> Vec<Function> {
    vec![function(
        "dielectric_fresnel",
        &[("cosine", Ty::F32), ("eta_i", Ty::F32), ("eta_t", Ty::F32)],
        Ty::F32,
        vec![
            if_(s("eta_i").eq(s("eta_t")), vec![ret(f(0.))]),
            let_("c", min(max(abs(s("cosine")), f(0.)), f(1.))),
            let_("eta", s("eta_i") / s("eta_t")),
            let_("sin2", s("eta") * s("eta") * (f(1.) - s("c") * s("c"))),
            if_(s("sin2").ge(f(1.)), vec![ret(f(1.))]),
            let_("ct", sqrt(f(1.) - s("sin2"))),
            let_(
                "rs",
                (s("eta_i") * s("c") - s("eta_t") * s("ct"))
                    / (s("eta_i") * s("c") + s("eta_t") * s("ct")),
            ),
            let_(
                "rp",
                (s("eta_t") * s("c") - s("eta_i") * s("ct"))
                    / (s("eta_t") * s("c") + s("eta_i") * s("ct")),
            ),
            ret(f(0.5) * (s("rs") * s("rs") + s("rp") * s("rp"))),
        ],
    )]
}
pub(super) fn body(ior: Expr, occlusion: Expr) -> Vec<Stmt> {
    vec![
        let_("ior", ior),
        var("eta_i", f(1.)),
        var("eta_t", s("ior")),
        if_(
            Expr::var("back", Ty::Bool),
            vec![set(s("eta_i"), s("ior")), set(s("eta_t"), f(1.))],
        ),
        let_("eta", s("eta_i") / s("eta_t")),
        let_("cosine", dot(v("view"), v("geometric"))),
        let_(
            "fresnel",
            call(
                "dielectric_fresnel",
                Ty::F32,
                vec![s("cosine"), s("eta_i"), s("eta_t")],
            ),
        ),
        let_(
            "sin2",
            s("eta") * s("eta") * max(f(1.) - s("cosine") * s("cosine"), f(0.)),
        ),
        var(
            "secondary",
            v("direction") + v("geometric") * (f(2.) * s("cosine")),
        ),
        var("transmission", splat(1.)),
        // Fresnel1 is deterministic even when the largest RNG midpoint rounds to1.
        if_(
            s("fresnel")
                .lt(f(1.))
                .and(path_random(6, true).ge(s("fresnel")))
                .and(s("sin2").lt(f(1.))),
            vec![
                set(
                    v("secondary"),
                    norm(
                        v("direction") * s("eta")
                            + v("geometric") * (s("eta") * s("cosine") - sqrt(f(1.) - s("sin2"))),
                    ),
                ),
                set(v("transmission"), v("color") * s("eta") * s("eta")),
            ],
        ),
        set(v("secondary"), norm(v("secondary"))),
        var("epsilon", f(1e-5)),
        if_(
            dot(v("secondary"), v("geometric")).le(f(0.)),
            vec![set(s("epsilon"), f(-1e-5))],
        ),
        let_(
            "transmission_origin",
            v("position") + v("geometric") * s("epsilon"),
        ),
        let_("occlusion", occlusion),
        set(v("next_origin"), v("transmission_origin")),
        set(v("next_direction"), v("secondary")),
        set(v("weight"), v("transmission") * s("occlusion")),
        alpha::last_bounce(
            true,
            v("transmission_origin"),
            v("secondary"),
            v("transmission") * xyz(param(7)) * s("occlusion"),
        ),
    ]
}
