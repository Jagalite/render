//! Bounded coverage traversal, authored through the shared typed Rust IR.
use super::*;
pub(super) fn functions() -> Vec<Function> {
    vec![
        function(
            "opacity",
            &[("origin", Ty::V3), ("direction", Ty::V3), ("hit", Ty::V4)],
            Ty::F32,
            vec![
                let_("base", cast(Ty::U32, q("hit").field("y")) * u(16)),
                let_("coverage", inst(i("base") + u(15))),
                if_(q("coverage").field("x").eq(f(0.)), vec![ret(f(1.))]),
                let_("triangle", cast(Ty::U32, q("hit").field("z"))),
                let_(
                    "bary",
                    call(
                        "project_bary",
                        Ty::V3,
                        vec![i("base"), i("triangle"), v("origin"), v("direction")],
                    ),
                ),
                let_("binding", inst(i("base") + u(10))),
                let_(
                    "uv",
                    call(
                        "bary_uv_set",
                        Ty::V2,
                        vec![
                            i("triangle"),
                            v("bary"),
                            cast(Ty::U32, q("binding").field("y")),
                        ],
                    ),
                ),
                let_(
                    "alpha",
                    call(
                        "sample_map",
                        Ty::V4,
                        vec![
                            q("binding").field("x"),
                            Expr::var("uv", Ty::V2),
                            vec2(f(0.), f(0.)),
                            vec2(f(0.), f(0.)),
                        ],
                    )
                    .field("w")
                        * call("bary_color", Ty::V4, vec![i("triangle"), v("bary")]).field("w"),
                ),
                if_(
                    q("coverage").field("x").eq(f(1.)),
                    vec![
                        // A positive cutoff always rejects zero, even if a GPU
                        // flushes a packed subnormal threshold to zero.
                        if_(s("alpha").le(f(0.)), vec![ret(f(0.))]),
                        if_(s("alpha").ge(q("coverage").field("z")), vec![ret(f(1.))]),
                        ret(f(0.)),
                    ],
                ),
                ret(s("alpha") * q("coverage").field("y")),
            ],
        ),
        function(
            "covered_trace",
            &[
                ("origin", Ty::V3),
                ("direction", Ty::V3),
                ("limit", Ty::F32),
                ("minimum", Ty::F32),
                ("pixel", Ty::U32),
                ("sample", Ty::U32),
                ("bounce", Ty::U32),
            ],
            Ty::V4,
            vec![
                if_(
                    param(9).field("w").eq(f(0.)),
                    vec![ret(call(
                        "trace",
                        Ty::V4,
                        vec![v("origin"), v("direction"), s("limit"), s("minimum")],
                    ))],
                ),
                var("near", s("minimum")),
                var("transparent", u(0)),
                Stmt::While(
                    i("transparent").le(u(64)),
                    vec![
                        if_(
                            s("near").gt(s("limit")),
                            vec![ret(vec4(splat(-1.), f(-1.)))],
                        ),
                        let_(
                            "hit",
                            call(
                                "trace",
                                Ty::V4,
                                vec![v("origin"), v("direction"), s("limit"), s("near")],
                            ),
                        ),
                        if_(q("hit").field("x").lt(f(0.)), vec![ret(q("hit"))]),
                        let_(
                            "alpha",
                            call(
                                "opacity",
                                Ty::F32,
                                vec![v("origin"), v("direction"), q("hit")],
                            ),
                        ),
                        // The CPU midpoint RNG is strictly below one. f32 can
                        // round its largest midpoint to one; full coverage must
                        // remain unconditional, independent of that approximation.
                        if_(s("alpha").ge(f(1.)), vec![ret(q("hit"))]),
                        let_(
                            "choice",
                            call(
                                "random",
                                Ty::F32,
                                vec![
                                    i("pixel"),
                                    i("sample"),
                                    u(1024) + i("bounce") * u(128) + i("transparent"),
                                    cast(Ty::U32, param(5).field("w"))
                                        + cast(Ty::U32, param(7).field("w")) * u(65536),
                                ],
                            ),
                        ),
                        if_(s("choice").lt(s("alpha")), vec![ret(q("hit"))]),
                        set(i("transparent"), i("transparent") + u(1)),
                        set(s("near"), q("hit").field("x") + f(1e-5)),
                    ],
                ),
                ret(vec4(splat(-2.), f(-2.))),
            ],
        ),
        function(
            "alpha_visibility",
            &[
                ("origin", Ty::V3),
                ("direction", Ty::V3),
                ("limit", Ty::F32),
            ],
            Ty::F32,
            vec![
                var("near", f(1e-5)),
                var("transmission", f(1.)),
                var("crossings", u(0)),
                Stmt::While(
                    i("crossings").lt(u(64)),
                    vec![
                        if_(s("near").gt(s("limit")), vec![ret(s("transmission"))]),
                        let_(
                            "hit",
                            call(
                                "trace",
                                Ty::V4,
                                vec![v("origin"), v("direction"), s("limit"), s("near")],
                            ),
                        ),
                        if_(q("hit").field("x").lt(f(0.)), vec![ret(s("transmission"))]),
                        set(
                            s("transmission"),
                            s("transmission")
                                * (f(1.)
                                    - call(
                                        "opacity",
                                        Ty::F32,
                                        vec![v("origin"), v("direction"), q("hit")],
                                    )),
                        ),
                        if_(s("transmission").le(f(0.)), vec![ret(f(0.))]),
                        set(s("near"), q("hit").field("x") + f(1e-5)),
                        set(i("crossings"), i("crossings") + u(1)),
                    ],
                ),
                ret(f(-1.)),
            ],
        ),
    ]
}
/// Preserve the original opaque arithmetic and trace path byte-for-byte in IR.
pub(super) fn illuminate(
    extended: bool,
    origin: Expr,
    direction: Expr,
    limit: Expr,
    contribution: Expr,
) -> Stmt {
    if !extended {
        return if_(
            trace(origin, direction, limit).field("x").lt(f(0.)),
            vec![set(v("radiance"), v("radiance") + contribution)],
        );
    }
    Stmt::If(
        param(9).field("w").gt(f(0.)),
        vec![
            let_(
                "visibility",
                call(
                    "alpha_visibility",
                    Ty::F32,
                    vec![origin.clone(), direction.clone(), limit.clone()],
                ),
            ),
            Stmt::If(
                s("visibility").lt(f(0.)),
                vec![set(s("alpha_failed"), f(1.))],
                vec![set(
                    v("radiance"),
                    v("radiance") + contribution.clone() * s("visibility"),
                )],
            ),
        ],
        vec![if_(
            trace(origin, direction, limit).field("x").lt(f(0.)),
            vec![set(v("radiance"), v("radiance") + contribution)],
        )],
    )
}

pub(super) fn last_bounce(
    extended: bool,
    origin: Expr,
    direction: Expr,
    contribution: Expr,
) -> Stmt {
    let last = (i("bounce") + u(1)).eq(cast(Ty::U32, param(0).field("w")));
    if extended {
        if_(
            last,
            vec![illuminate(true, origin, direction, f(1e30), contribution)],
        )
    } else {
        if_(
            last.and(trace(origin, direction, f(1e30)).field("x").lt(f(0.))),
            vec![set(v("radiance"), v("radiance") + contribution)],
        )
    }
}
