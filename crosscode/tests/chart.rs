//! The cross-code chart, drawn again from the points the studies wrote to `crosscode/results/`,
//! without running either study: `just crosscode-chart`.

#[test]
#[ignore = "rewrites docs/cross-code-chart*.svg"]
fn redraw_the_chart() {
    photonoxide_crosscode::chart::redraw().expect("the chart");
}
