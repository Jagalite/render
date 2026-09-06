//! Bounded sparse absorption/emission, expressed as Rust kernel IR.
use super::*;
fn pair(n: &str) -> Expr {
    Expr::var(n, Ty::V2)
}
fn exp3(e: Expr) -> Expr {
    call("exp", Ty::V3, vec![e])
}
fn count() -> Expr {
    cast(Ty::U32, param(11).field("y"))
}
fn base() -> Expr {
    cast(Ty::U32, param(11).field("x")) + i("cell") * u(7)
}
fn transformed(p: Expr) -> Expr {
    xyz(data(i("base"))) * p.clone().field("x")
        + xyz(data(i("base") + u(1))) * p.clone().field("y")
        + xyz(data(i("base") + u(2))) * p.field("z")
}
fn interval() -> Expr {
    call(
        "medium_interval",
        Ty::V2,
        vec![
            i("cell"),
            v("origin"),
            v("direction"),
            s("minimum"),
            s("limit"),
        ],
    )
}
pub(super) fn transmittance(origin: Expr, direction: Expr, minimum: Expr, limit: Expr) -> Expr {
    call(
        "medium_transmittance",
        Ty::V3,
        vec![origin, direction, minimum, limit],
    )
}
pub(super) fn radiance(origin: Expr, direction: Expr, minimum: Expr, limit: Expr) -> Expr {
    call(
        "medium_radiance",
        Ty::V3,
        vec![origin, direction, minimum, limit],
    )
}
pub(super) fn functions() -> Vec<Function> {
    let args = [
        ("origin", Ty::V3),
        ("direction", Ty::V3),
        ("minimum", Ty::F32),
        ("limit", Ty::F32),
    ];
    let mut intervals = vec![
        let_("base", base()),
        let_("ro", transformed(v("origin"))),
        let_("rd", transformed(v("direction"))),
        let_("lower", xyz(data(i("base") + u(3)))),
        let_("upper", xyz(data(i("base") + u(4)))),
        var("near", s("minimum")),
        var("far", s("limit")),
    ];
    for axis in ["x", "y", "z"] {
        intervals.push(Stmt::If(
            v("rd").field(axis).eq(f(0.)),
            vec![if_(
                v("ro")
                    .field(axis)
                    .lt(v("lower").field(axis))
                    .or(v("ro").field(axis).ge(v("upper").field(axis))),
                vec![ret(vec2(f(1.), f(0.)))],
            )],
            vec![
                let_(
                    "a",
                    (v("lower").field(axis) - v("ro").field(axis)) / v("rd").field(axis),
                ),
                let_(
                    "b",
                    (v("upper").field(axis) - v("ro").field(axis)) / v("rd").field(axis),
                ),
                set(s("near"), max(s("near"), min(s("a"), s("b")))),
                set(s("far"), min(s("far"), max(s("a"), s("b")))),
            ],
        ));
        intervals.push(if_(s("near").ge(s("far")), vec![ret(vec2(f(1.), f(0.)))]));
    }
    intervals.push(ret(vec2(s("near"), s("far"))));
    vec![
        function(
            "medium_interval",
            &[
                ("cell", Ty::U32),
                ("origin", Ty::V3),
                ("direction", Ty::V3),
                ("minimum", Ty::F32),
                ("limit", Ty::F32),
            ],
            Ty::V2,
            intervals,
        ),
        function(
            "medium_integral",
            &[("sigma", Ty::F32), ("distance", Ty::F32)],
            Ty::F32,
            vec![
                // The normalized Beer integral has alternating-series remainder
                // at most tau^5/720 <= 1.4e-8 for 0 <= tau <= 0.1.
                let_("tau", s("sigma") * s("distance")),
                if_(
                    s("tau").lt(f(0.1)),
                    vec![ret(s("distance")
                        * (f(1.)
                            + s("tau")
                                * (f(-0.5)
                                    + s("tau")
                                        * (f(1. / 6.)
                                            + s("tau")
                                                * (f(-1. / 24.)
                                                    + s("tau") * f(1. / 120.))))))],
                ),
                ret((f(1.) - call("exp", Ty::F32, vec![f(0.) - s("tau")])) / s("sigma")),
            ],
        ),
        function(
            "medium_transmittance",
            &args,
            Ty::V3,
            vec![
                var("tau", splat(0.)),
                var("cell", u(0)),
                Stmt::While(
                    i("cell").lt(count()),
                    vec![
                        let_("segment", interval()),
                        if_(
                            pair("segment").field("x").lt(pair("segment").field("y")),
                            vec![set(
                                v("tau"),
                                v("tau")
                                    + xyz(data(base() + u(5)))
                                        * (pair("segment").field("y") - pair("segment").field("x")),
                            )],
                        ),
                        set(i("cell"), i("cell") + u(1)),
                    ],
                ),
                ret(exp3(splat(0.) - v("tau"))),
            ],
        ),
        function(
            "medium_radiance",
            &args,
            Ty::V3,
            vec![
                var("at", s("minimum")),
                var("event", u(0)),
                var("through", splat(1.)),
                var("radiance", splat(0.)),
                Stmt::While(
                    i("event").le(count() * u(2)).and(s("at").lt(s("limit"))),
                    vec![
                        var("next", s("limit")),
                        var("sigma", splat(0.)),
                        var("emission", splat(0.)),
                        var("cell", u(0)),
                        Stmt::While(
                            i("cell").lt(count()),
                            vec![
                                let_("segment", interval()),
                                if_(
                                    pair("segment").field("x").lt(pair("segment").field("y")),
                                    vec![
                                        if_(
                                            pair("segment").field("x").gt(s("at")),
                                            vec![set(
                                                s("next"),
                                                min(s("next"), pair("segment").field("x")),
                                            )],
                                        ),
                                        if_(
                                            pair("segment").field("y").gt(s("at")),
                                            vec![set(
                                                s("next"),
                                                min(s("next"), pair("segment").field("y")),
                                            )],
                                        ),
                                        if_(
                                            pair("segment")
                                                .field("x")
                                                .le(s("at"))
                                                .and(pair("segment").field("y").gt(s("at"))),
                                            vec![
                                                set(
                                                    v("sigma"),
                                                    v("sigma") + xyz(data(base() + u(5))),
                                                ),
                                                set(
                                                    v("emission"),
                                                    v("emission") + xyz(data(base() + u(6))),
                                                ),
                                            ],
                                        ),
                                    ],
                                ),
                                set(i("cell"), i("cell") + u(1)),
                            ],
                        ),
                        let_("distance", s("next") - s("at")),
                        // Do not form an unbounded empty-tail product 0*infinity.
                        if_(
                            v("emission")
                                .field("x")
                                .gt(f(0.))
                                .or(v("emission").field("y").gt(f(0.)))
                                .or(v("emission").field("z").gt(f(0.))),
                            vec![
                                let_(
                                    "integral",
                                    vec3(
                                        call(
                                            "medium_integral",
                                            Ty::F32,
                                            vec![v("sigma").field("x"), s("distance")],
                                        ),
                                        call(
                                            "medium_integral",
                                            Ty::F32,
                                            vec![v("sigma").field("y"), s("distance")],
                                        ),
                                        call(
                                            "medium_integral",
                                            Ty::F32,
                                            vec![v("sigma").field("z"), s("distance")],
                                        ),
                                    ),
                                ),
                                set(
                                    v("radiance"),
                                    v("radiance") + v("through") * v("emission") * v("integral"),
                                ),
                            ],
                        ),
                        set(
                            v("through"),
                            v("through") * exp3(splat(0.) - v("sigma") * s("distance")),
                        ),
                        set(s("at"), s("next")),
                        set(i("event"), i("event") + u(1)),
                    ],
                ),
                ret(v("radiance")),
            ],
        ),
    ]
}

pub(super) fn illuminate(
    extended: bool,
    enabled: bool,
    origin: Expr,
    direction: Expr,
    limit: Expr,
    contribution: Expr,
) -> Stmt {
    let contribution = if enabled {
        contribution * transmittance(origin.clone(), direction.clone(), f(1e-5), limit.clone())
    } else {
        contribution
    };
    alpha::illuminate(extended, origin, direction, limit, contribution)
}
pub(super) fn last_bounce(
    extended: bool,
    enabled: bool,
    origin: Expr,
    direction: Expr,
    contribution: Expr,
) -> Stmt {
    let contribution = if enabled {
        contribution * transmittance(origin.clone(), direction.clone(), f(1e-5), f(1e30))
    } else {
        contribution
    };
    alpha::last_bounce(extended, origin, direction, contribution)
}
pub(super) fn integrate() -> Stmt {
    Stmt::Sequence(vec![
        var("medium_limit", s("far")),
        if_(
            q("hit").field("x").ge(f(0.)),
            vec![set(s("medium_limit"), q("hit").field("x"))],
        ),
        set(
            v("sum"),
            v("sum")
                + v("throughput")
                    * radiance(v("origin"), v("direction"), s("near"), s("medium_limit")),
        ),
        set(
            v("throughput"),
            v("throughput")
                * transmittance(v("origin"), v("direction"), s("near"), s("medium_limit")),
        ),
    ])
}
