use fund_types::CurvePoint;
use leptos::prelude::*;
use std::sync::Arc;

const W: f64 = 800.0;
const H: f64 = 300.0;
const PAD_L: f64 = 60.0;
const PAD_R: f64 = 20.0;
const PAD_T: f64 = 20.0;
const PAD_B: f64 = 40.0;

/// A line to draw on the chart.
#[derive(Debug, Clone)]
pub struct Series {
    pub points: Vec<CurvePoint>,
    pub color: &'static str,
    pub name: &'static str,
    /// Indices into `points` to mark (buy/sell), optional.
    pub markers: Vec<ChartMarker>,
}

/// A marker drawn on a series point.
#[derive(Debug, Clone)]
pub struct ChartMarker {
    pub index: usize,
    pub kind: MarkerKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerKind {
    Buy,
    Sell,
}

fn x_pos(i: usize, s: usize, e: usize) -> f64 {
    let n = (e - s).max(1) as f64;
    let t = if e == s { 0.0 } else { (i - s) as f64 / n };
    PAD_L + t * (W - PAD_L - PAD_R)
}

fn y_pos(v: f64, lo: f64, hi: f64) -> f64 {
    let t = (v - lo) / (hi - lo);
    PAD_T + (1.0 - t) * (H - PAD_T - PAD_B)
}

fn y_range(series: &[Series], s: usize, e: usize) -> (f64, f64) {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for point in series
        .iter()
        .flat_map(|ser| ser.points.iter())
        .take(e + 1)
        .skip(s)
    {
        lo = lo.min(point.market_value);
        hi = hi.max(point.market_value);
    }
    if !lo.is_finite() || (hi - lo).abs() < f64::EPSILON {
        return (0.0, 1.0);
    }
    let pad = (hi - lo) * 0.1;
    (lo - pad, hi + pad)
}

fn path_d(points: &[CurvePoint], s: usize, e: usize, lo: f64, hi: f64) -> String {
    let mut d = String::new();
    for (k, p) in points[s..=e].iter().enumerate() {
        let x = x_pos(s + k, s, e);
        let y = y_pos(p.market_value, lo, hi);
        if k == 0 {
            d.push_str(&format!("M {x:.1} {y:.1}"));
        } else {
            d.push_str(&format!(" L {x:.1} {y:.1}"));
        }
    }
    d
}

fn grid_d(lo: f64, hi: f64) -> String {
    let mut d = String::new();
    for k in 0..=4 {
        let v = lo + (hi - lo) * k as f64 / 4.0;
        let y = y_pos(v, lo, hi);
        d.push_str(&format!("M {PAD_L:.1} {y:.1} L {:.1} {y:.1}", W - PAD_R));
    }
    d
}

/// Map a mouse client x to viewBox units using the element's bounding rect.
fn viewbox_x(client_x: f64, target: Option<&web_sys::EventTarget>) -> Option<f64> {
    use wasm_bindgen::JsCast;
    let rect = target?
        .dyn_ref::<web_sys::Element>()?
        .get_bounding_client_rect();
    if rect.width() <= 0.0 {
        return None;
    }
    Some((client_x - rect.left()) * W / rect.width())
}

#[component]
pub fn Chart(series: Vec<Series>) -> impl IntoView {
    let series = Arc::new(series);
    let count = series.first().map(|ser| ser.points.len()).unwrap_or(0);
    // Visible index window.
    let start = RwSignal::new(0usize);
    let end = RwSignal::new(count.saturating_sub(1).max(1));
    // Hovered index.
    let hover = RwSignal::new(None::<usize>);
    // Pointer inside the figure.
    let hovering = RwSignal::new(false);
    // Range-selection drag state (viewBox x units).
    let dragging = RwSignal::new(false);
    let sel_start = RwSignal::new(None::<f64>);
    let sel_end = RwSignal::new(None::<f64>);
    // A y-domain refresh key, bumped on zoom so the signal updates.
    let zoom_key = RwSignal::new(0u32);

    // Complete a drag selection by zooming the window to the selected range.
    let finalize_selection: Arc<dyn Fn()> = Arc::new(move || {
        dragging.set(false);
        let (Some(x1), Some(x2)) = (sel_start.get_untracked(), sel_end.get_untracked()) else {
            return;
        };
        sel_start.set(None);
        sel_end.set(None);
        if (x2 - x1).abs() < 6.0 {
            return;
        }
        let s = start.get_untracked();
        let e = end.get_untracked();
        let t1 = ((x1 - PAD_L) / (W - PAD_L - PAD_R)).clamp(0.0, 1.0);
        let t2 = ((x2 - PAD_L) / (W - PAD_L - PAD_R)).clamp(0.0, 1.0);
        let i1 = (s as f64 + t1 * (e - s) as f64).round() as usize;
        let i2 = (s as f64 + t2 * (e - s) as f64).round() as usize;
        let (lo, hi) = (i1.min(i2), i1.max(i2));
        if hi > lo {
            start.set(lo);
            end.set(hi);
            zoom_key.update(|k| *k += 1);
        }
    });

    let on_mousemove = move |ev: leptos::ev::MouseEvent| {
        if dragging.get() {
            if let Some(x) = viewbox_x(ev.client_x() as f64, ev.current_target().as_ref()) {
                sel_end.set(Some(x));
            }
        } else if let Some(x) = viewbox_x(ev.client_x() as f64, ev.current_target().as_ref()) {
            let s = start.get();
            let e = end.get();
            let t = ((x - PAD_L) / (W - PAD_L - PAD_R)).clamp(0.0, 1.0);
            let i = (s as f64 + t * (e - s) as f64).round() as usize;
            hover.set(Some(i.min(e)));
        }
    };

    let on_enter = move |_| {
        hovering.set(true);
    };

    let finalize_leave = Arc::clone(&finalize_selection);
    let on_leave = move |_| {
        hovering.set(false);
        hover.set(None);
        if dragging.get_untracked() {
            finalize_leave();
        }
    };

    let on_wheel = move |ev: leptos::ev::WheelEvent| {
        let s = start.get();
        let e = end.get();
        let x = viewbox_x(ev.client_x() as f64, ev.current_target().as_ref());
        let t = x
            .map(|x| ((x - PAD_L) / (W - PAD_L - PAD_R)).clamp(0.0, 1.0))
            .unwrap_or(0.5);
        let span = (e - s).max(2) as f64;
        let max = (count - 1).max(1) as f64;
        let min_span = 2.0;
        // Scroll up zooms in, scroll down zooms out, centered on the mouse.
        let new_span = if ev.delta_y() < 0.0 {
            (span * 0.5).max(min_span)
        } else {
            (span * 2.0).min(max)
        };
        let mouse_index = s as f64 + t * (e - s) as f64;
        let ns = (mouse_index - t * new_span).clamp(0.0, max - new_span);
        let ne = ns + new_span;
        start.set(ns.round() as usize);
        end.set(ne.round() as usize);
        zoom_key.update(|k| *k += 1);
        hover.set(None);
        ev.prevent_default();
    };

    let on_mousedown = move |ev: leptos::ev::MouseEvent| {
        if let Some(x) = viewbox_x(ev.client_x() as f64, ev.current_target().as_ref()) {
            dragging.set(true);
            sel_start.set(Some(x));
            sel_end.set(Some(x));
        }
        ev.prevent_default();
    };

    let finalize_up = Arc::clone(&finalize_selection);
    let on_mouseup = move |_| {
        if dragging.get_untracked() {
            finalize_up();
        }
    };

    let on_reset = move |_| {
        start.set(0);
        end.set(count.saturating_sub(1).max(1));
        zoom_key.update(|k| *k += 1);
        hover.set(None);
        sel_start.set(None);
        sel_end.set(None);
    };

    let all_points = Arc::clone(&series);
    let paths = move || {
        let (s, e) = (start.get(), end.get());
        let (lo, hi) = y_range(&all_points, s, e);
        all_points
            .iter()
            .map(|ser| {
                let d = path_d(&ser.points, s, e, lo, hi);
                view! {
                    <path d=d fill="none" stroke=ser.color stroke-width="2"/>
                }
            })
            .collect_view()
    };

    let grid_points = Arc::clone(&series);
    let grid_path = move || {
        let (s, e) = (start.get(), end.get());
        let (lo, hi) = y_range(&grid_points, s, e);
        grid_d(lo, hi)
    };

    let x_points = Arc::clone(&series);
    let x_labels = move || {
        let s = start.get();
        let e = end.get();
        let Some(points) = x_points.first().map(|ser| &ser.points) else {
            return Vec::new();
        };
        (0..=4)
            .map(|k| {
                let i = s + ((e - s) as f64 * k as f64 / 4.0) as usize;
                let i = i.min(e);
                let anchor = if k == 0 {
                    "start"
                } else if k == 4 {
                    "end"
                } else {
                    "middle"
                };
                (
                    x_pos(i, s, e),
                    points.get(i).map(|p| p.date.clone()).unwrap_or_default(),
                    anchor,
                )
            })
            .collect::<Vec<_>>()
    };

    let y_points = Arc::clone(&series);
    let y_labels = move || {
        let (s, e) = (start.get(), end.get());
        let (lo, hi) = y_range(&y_points, s, e);
        (0..=4)
            .map(|k| {
                let v = lo + (hi - lo) * k as f64 / 4.0;
                (y_pos(v, lo, hi), v)
            })
            .collect::<Vec<_>>()
    };

    let marker_points = Arc::clone(&series);
    let markers = move || {
        let (s, e) = (start.get(), end.get());
        let (lo, hi) = y_range(&marker_points, s, e);
        marker_points
            .iter()
            .flat_map(|ser| {
                ser.markers.iter().filter_map(move |marker| {
                    if marker.index < s || marker.index > e {
                        return None;
                    }
                    let p = ser.points.get(marker.index)?;
                    let x = x_pos(marker.index, s, e);
                    let y = y_pos(p.market_value, lo, hi);
                    let color = match marker.kind {
                        MarkerKind::Buy => "#d63a3a",
                        MarkerKind::Sell => "#2b6cb0",
                    };
                    Some(view! {
                        <circle cx=x cy=y r="3.5" fill=color stroke="#fff" stroke-width="1"/>
                    })
                })
            })
            .collect_view()
    };

    let hover_series = Arc::clone(&series);
    let hover_line = move || {
        hover.get().map(|i| {
            let x = x_pos(i, start.get(), end.get());
            let y = {
                let (s, e) = (start.get(), end.get());
                let (lo, hi) = y_range(&hover_series, s, e);
                hover_series
                    .first()
                    .and_then(|ser| ser.points.get(i))
                    .map(|p| y_pos(p.market_value, lo, hi))
                    .unwrap_or(PAD_T)
            };
            view! {
                <line x1=x x2=x y1=PAD_T y2={H-PAD_B} stroke="#666" stroke-width="1"/>
                <line x1={PAD_L} x2={W-PAD_R} y1=y y2=y stroke="#666" stroke-width="1" stroke-dasharray="3,3"/>
            }
        })
    };

    let hover_points = Arc::clone(&series);
    let hover_labels = move || {
        hover.get().and_then(|i| {
            let (s, e) = (start.get(), end.get());
            let (lo, hi) = y_range(&hover_points, s, e);
            let primary = hover_points.first().and_then(|ser| ser.points.get(i))?;
            let x = x_pos(i, s, e);
            let y = y_pos(primary.market_value, lo, hi);
            // X (date) label near the bottom of the vertical line.
            let (x_anchor, x_x) = if x > W - 40.0 {
                ("end", x - 6.0)
            } else if x < PAD_L + 40.0 {
                ("start", x + 6.0)
            } else {
                ("middle", x)
            };
            let x_label = view! {
                <text x=x_x y={H-PAD_B+14.0} class="crosshair" text-anchor=x_anchor>
                    {primary.date.clone()}
                </text>
            };
            // Y (value) label to the left of the horizontal line.
            let y_y = if y < PAD_T + 14.0 { y + 14.0 } else { y - 8.0 };
            let y_label = view! {
                <text x={PAD_L-8.0} y=y_y class="crosshair" text-anchor="end">
                    {format!("{:.4}", primary.market_value)}
                </text>
            };
            Some(view! {
                <circle cx=x cy=y r="3" fill="#333"/>
                {x_label}
                {y_label}
            })
        })
    };

    let sel_rect = move || {
        let (Some(a), Some(b)) = (sel_start.get(), sel_end.get()) else {
            return None;
        };
        let (x1, x2) = if a <= b { (a, b) } else { (b, a) };
        let x1 = x1.clamp(PAD_L, W - PAD_R);
        let x2 = x2.clamp(PAD_L, W - PAD_R);
        if x2 - x1 < 0.5 {
            return None;
        }
        Some(view! {
            <rect
                x=x1
                y=PAD_T
                width={x2-x1}
                height={H-PAD_T-PAD_B}
                fill="#888"
                opacity="0.25"
            />
        })
    };

    let show_reset = move || {
        let zoomed = start.get() > 0 || end.get() < count.saturating_sub(1).max(1);
        hovering.get() || zoomed
    };

    view! {
        <div>
            <div
                style="position:relative"
                on:mouseenter=on_enter
                on:mouseleave=on_leave
            >
                <svg
                    viewBox=format!("0 0 {W} {H}")
                    style="width:100%;height:auto;background:#fafafa;touch-action:none"
                    on:mousemove=on_mousemove
                    on:wheel=on_wheel
                    on:mousedown=on_mousedown
                    on:mouseup=on_mouseup
                >
                    <g class="grid" stroke="#ddd">
                        <path d=grid_path fill="none"/>
                    </g>
                    {move || x_labels().into_iter().map(|(x, label, anchor)| view! {
                        <text x=x y={H-12.0} class="axis" text-anchor=anchor>{label}</text>
                    }).collect_view()}
                    {move || y_labels().into_iter().map(|(y, v)| view! {
                        <text x={PAD_L-8.0} y=y class="axis" text-anchor="end">{format!("{v:.4}")}</text>
                    }).collect_view()}
                    {paths}
                    {markers}
                    {sel_rect}
                    {hover_line}
                    {hover_labels}
                </svg>
                {move || if show_reset() {
                    view! {
                        <button class="reset-btn" on:click=on_reset>"Reset view"</button>
                    }.into_any()
                } else {
                    ().into_any()
                }}
            </div>
            <p class="hint">"Scroll to zoom, drag to select a range, hover for values."</p>
        </div>
    }
}
