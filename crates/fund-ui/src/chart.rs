use fund_types::CurvePoint;
use leptos::prelude::*;
use std::sync::Arc;

const W: f64 = 800.0;
const H: f64 = 320.0;
const PAD_R: f64 = 20.0;
const PAD_T: f64 = 36.0;
const PAD_B: f64 = 48.0;

/// A line to draw on the chart.
#[derive(Debug, Clone)]
pub struct Series {
    pub points: Vec<CurvePoint>,
    pub color: &'static str,
    pub name: &'static str,
    /// Decimal places used for axis and crosshair labels.
    pub decimals: u32,
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

/// Format a value with the series' decimal precision.
fn fmt_val(v: f64, decimals: u32) -> String {
    format!("{v:.decimals$}", decimals = decimals as usize)
}

/// The widest decimals among all series.
fn max_decimals(series: &[Series]) -> u32 {
    series.iter().map(|ser| ser.decimals).max().unwrap_or(2)
}

/// Width of the widest y-label for the range, used as the left padding.
fn pad_l_for(lo: f64, hi: f64, decimals: u32) -> f64 {
    let mut max: f64 = 34.0;
    for k in 0..=4 {
        let v = lo + (hi - lo) * k as f64 / 4.0;
        let text = fmt_val(v, decimals);
        max = max.max(text.len() as f64 * 7.0 + 10.0);
    }
    max
}

fn x_pos(i: usize, s: usize, e: usize, pad_l: f64) -> f64 {
    let n = (e - s).max(1) as f64;
    let t = if e == s { 0.0 } else { (i - s) as f64 / n };
    pad_l + t * (W - pad_l - PAD_R)
}

fn index_at_x(x: f64, s: usize, e: usize, pad_l: f64) -> usize {
    let t = ((x - pad_l) / (W - pad_l - PAD_R)).clamp(0.0, 1.0);
    let i = (s as f64 + t * (e - s) as f64).round() as usize;
    i.min(e)
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

fn path_d(points: &[CurvePoint], s: usize, e: usize, lo: f64, hi: f64, pad_l: f64) -> String {
    let mut d = String::new();
    for (k, p) in points[s..=e].iter().enumerate() {
        let x = x_pos(s + k, s, e, pad_l);
        let y = y_pos(p.market_value, lo, hi);
        if k == 0 {
            d.push_str(&format!("M {x:.1} {y:.1}"));
        } else {
            d.push_str(&format!(" L {x:.1} {y:.1}"));
        }
    }
    d
}

fn grid_d(lo: f64, hi: f64, pad_l: f64) -> String {
    let mut d = String::new();
    for k in 0..=4 {
        let v = lo + (hi - lo) * k as f64 / 4.0;
        let y = y_pos(v, lo, hi);
        d.push_str(&format!("M {pad_l:.1} {y:.1} L {:.1} {y:.1}", W - PAD_R));
    }
    d
}

/// Map a mouse client position to viewBox units using the element's bounding rect.
fn viewbox_xy(
    client_x: f64,
    client_y: f64,
    target: Option<&web_sys::EventTarget>,
) -> Option<(f64, f64)> {
    use wasm_bindgen::JsCast;
    let rect = target?
        .dyn_ref::<web_sys::Element>()?
        .get_bounding_client_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return None;
    }
    Some((
        (client_x - rect.left()) * W / rect.width(),
        (client_y - rect.top()) * H / rect.height(),
    ))
}

#[component]
pub fn Chart(
    series: Vec<Series>,
    #[prop(optional)] title: Option<String>,
    #[prop(optional)] x_label: Option<String>,
    #[prop(optional)] y_label: Option<String>,
) -> impl IntoView {
    let title = title.unwrap_or_default();
    let x_label = x_label.unwrap_or_else(|| "Date".to_string());
    let y_label = y_label.unwrap_or_else(|| "Value".to_string());
    let series = Arc::new(series);
    let count = series.first().map(|ser| ser.points.len()).unwrap_or(0);
    // Visible index window.
    let start = RwSignal::new(0usize);
    let end = RwSignal::new(count.saturating_sub(1).max(1));
    // Hovered index + the series nearest the cursor.
    let hover = RwSignal::new(None::<usize>);
    let hover_series = RwSignal::new(0usize);
    // Pointer inside the figure.
    let hovering = RwSignal::new(false);
    // Range-selection drag state (data indices).
    let dragging = RwSignal::new(false);
    let sel_start_i = RwSignal::new(None::<usize>);
    let sel_end_i = RwSignal::new(None::<usize>);
    // A y-domain refresh key, bumped on zoom so the signal updates.
    let zoom_key = RwSignal::new(0u32);

    // Complete a drag selection by zooming the window to the selected range.
    let finalize_selection: Arc<dyn Fn()> = Arc::new(move || {
        dragging.set(false);
        let (Some(a), Some(b)) = (sel_start_i.get_untracked(), sel_end_i.get_untracked()) else {
            return;
        };
        sel_start_i.set(None);
        sel_end_i.set(None);
        let (lo, hi) = (a.min(b), a.max(b));
        if hi - lo < 1 {
            return;
        }
        start.set(lo);
        end.set(hi);
        zoom_key.update(|k| *k += 1);
    });

    let mm_series = Arc::clone(&series);
    let on_mousemove = move |ev: leptos::ev::MouseEvent| {
        let Some((x, y)) = viewbox_xy(
            ev.client_x() as f64,
            ev.client_y() as f64,
            ev.current_target().as_ref(),
        ) else {
            return;
        };
        let s = start.get();
        let e = end.get();
        let (lo, hi) = y_range(&mm_series, s, e);
        let pad_l = pad_l_for(lo, hi, max_decimals(&mm_series));
        let i = index_at_x(x, s, e, pad_l);
        if dragging.get() {
            sel_end_i.set(Some(i));
            return;
        }
        // Choose the series whose y at this index is closest to the cursor.
        let mut best: Option<(usize, f64)> = None;
        for (k, ser) in mm_series.iter().enumerate() {
            if let Some(p) = ser.points.get(i) {
                let dy = (y_pos(p.market_value, lo, hi) - y).abs();
                if best.is_none_or(|(_, bd)| dy < bd) {
                    best = Some((k, dy));
                }
            }
        }
        if let Some((k, _)) = best {
            hover_series.set(k);
        }
        hover.set(Some(i));
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

    let wheel_series = Arc::clone(&series);
    let on_wheel = move |ev: leptos::ev::WheelEvent| {
        let s = start.get();
        let e = end.get();
        let (lo, hi) = y_range(&wheel_series, s, e);
        let pad_l = pad_l_for(lo, hi, max_decimals(&wheel_series));
        let xy = viewbox_xy(
            ev.client_x() as f64,
            ev.client_y() as f64,
            ev.current_target().as_ref(),
        );
        let t = xy
            .map(|(x, _)| ((x - pad_l) / (W - pad_l - PAD_R)).clamp(0.0, 1.0))
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

    let md_series = Arc::clone(&series);
    let on_mousedown = move |ev: leptos::ev::MouseEvent| {
        let Some((x, _)) = viewbox_xy(
            ev.client_x() as f64,
            ev.client_y() as f64,
            ev.current_target().as_ref(),
        ) else {
            return;
        };
        let s = start.get();
        let e = end.get();
        let (lo, hi) = y_range(&md_series, s, e);
        let pad_l = pad_l_for(lo, hi, max_decimals(&md_series));
        let i = index_at_x(x, s, e, pad_l);
        hover.set(None);
        dragging.set(true);
        sel_start_i.set(Some(i));
        sel_end_i.set(Some(i));
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
        sel_start_i.set(None);
        sel_end_i.set(None);
    };

    let all_points = Arc::clone(&series);
    let paths = move || {
        let (s, e) = (start.get(), end.get());
        let (lo, hi) = y_range(&all_points, s, e);
        let pad_l = pad_l_for(lo, hi, max_decimals(&all_points));
        all_points
            .iter()
            .map(|ser| {
                let d = path_d(&ser.points, s, e, lo, hi, pad_l);
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
        grid_d(lo, hi, pad_l_for(lo, hi, max_decimals(&grid_points)))
    };

    let x_points = Arc::clone(&series);
    let x_labels = move || {
        let s = start.get();
        let e = end.get();
        let Some(points) = x_points.first().map(|ser| &ser.points) else {
            return Vec::new();
        };
        let (lo, hi) = y_range(&x_points, s, e);
        let pad_l = pad_l_for(lo, hi, max_decimals(&x_points));
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
                    x_pos(i, s, e, pad_l),
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
        let decimals = max_decimals(&y_points);
        let pad_l = pad_l_for(lo, hi, decimals);
        (0..=4)
            .map(|k| {
                let v = lo + (hi - lo) * k as f64 / 4.0;
                (y_pos(v, lo, hi), v, pad_l, decimals)
            })
            .collect::<Vec<_>>()
    };

    let marker_points = Arc::clone(&series);
    let markers = move || {
        let (s, e) = (start.get(), end.get());
        let (lo, hi) = y_range(&marker_points, s, e);
        let pad_l = pad_l_for(lo, hi, max_decimals(&marker_points));
        marker_points
            .iter()
            .flat_map(|ser| {
                ser.markers.iter().filter_map(move |marker| {
                    if marker.index < s || marker.index > e {
                        return None;
                    }
                    let p = ser.points.get(marker.index)?;
                    let x = x_pos(marker.index, s, e, pad_l);
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

    let legend_series = Arc::clone(&series);
    let legend = move || {
        let mut items = Vec::new();
        let n = legend_series.len();
        for (k, ser) in legend_series.iter().enumerate() {
            let x = W - 8.0;
            let y = 16.0 + k as f64 * 16.0;
            items.push(view! {
                <g class="legend">
                    <rect x={x-80.0} y={y-9.0} width="9" height="9" fill=ser.color/>
                    <text x={x-64.0} y=y class="axis" text-anchor="start">{ser.name}</text>
                </g>
            });
        }
        let _ = n;
        items.collect_view()
    };

    let hover_series_clone = hover_series;
    let hov_series = Arc::clone(&series);
    let hover_line = move || {
        hover.get().map(|i| {
            let (s, e) = (start.get(), end.get());
            let (lo, hi) = y_range(&hov_series, s, e);
            let pad_l = pad_l_for(lo, hi, max_decimals(&hov_series));
            let x = x_pos(i, s, e, pad_l);
            let k = hover_series_clone.get().min(hov_series.len().saturating_sub(1));
            let y = hov_series
                .get(k)
                .and_then(|ser| ser.points.get(i))
                .map(|p| y_pos(p.market_value, lo, hi))
                .unwrap_or(PAD_T);
            view! {
                <line x1=x x2=x y1=PAD_T y2={H-PAD_B} stroke="#666" stroke-width="1"/>
                <line x1={pad_l} x2={W-PAD_R} y1=y y2=y stroke="#666" stroke-width="1" stroke-dasharray="3,3"/>
            }
        })
    };

    let hov_points = Arc::clone(&series);
    let hover_labels = move || {
        hover.get().and_then(|i| {
            let (s, e) = (start.get(), end.get());
            let (lo, hi) = y_range(&hov_points, s, e);
            let decimals = max_decimals(&hov_points);
            let pad_l = pad_l_for(lo, hi, decimals);
            let k = hover_series.get().min(hov_points.len().saturating_sub(1));
            let primary = hov_points.get(k).and_then(|ser| ser.points.get(i))?;
            let x = x_pos(i, s, e, pad_l);
            let y = y_pos(primary.market_value, lo, hi);
            // X (date) label near the bottom of the vertical line.
            let (x_anchor, x_x) = if x > W - 40.0 {
                ("end", x - 6.0)
            } else if x < pad_l + 40.0 {
                ("start", x + 6.0)
            } else {
                ("middle", x)
            };
            let x_label = view! {
                <text x=x_x y={H-PAD_B+14.0} class="crosshair" text-anchor=x_anchor>
                    {primary.date.clone()}
                </text>
            };
            // Y (value) label to the left of the horizontal line, using the hovered series' precision.
            let pk = hover_series.get().min(hov_points.len().saturating_sub(1));
            let pv = hov_points.get(pk).and_then(|ser| ser.points.get(i))?;
            let pdec = hov_points
                .get(pk)
                .map(|ser| ser.decimals)
                .unwrap_or(decimals);
            let y_y = if y < PAD_T + 14.0 { y + 14.0 } else { y - 8.0 };
            let y_label = view! {
                <text x={pad_l-8.0} y=y_y class="crosshair" text-anchor="end">
                    {fmt_val(pv.market_value, pdec)}
                </text>
            };
            Some(view! {
                <circle cx=x cy=y r="3" fill="#333"/>
                {x_label}
                {y_label}
            })
        })
    };

    let sel_points = Arc::clone(&series);
    let sel_rect = move || {
        let (Some(a), Some(b)) = (sel_start_i.get(), sel_end_i.get()) else {
            return None;
        };
        let (s, e) = (start.get(), end.get());
        let (lo, hi) = y_range(&sel_points, s, e);
        let pad_l = pad_l_for(lo, hi, max_decimals(&sel_points));
        let (i1, i2) = (a.min(b), a.max(b));
        if i2 - i1 < 1 {
            return None;
        }
        let points = sel_points.first().map(|ser| &ser.points)?;
        let x1 = x_pos(i1, s, e, pad_l);
        let x2 = x_pos(i2, s, e, pad_l);
        let d1 = points.get(i1).map(|p| p.date.clone()).unwrap_or_default();
        let d2 = points.get(i2).map(|p| p.date.clone()).unwrap_or_default();
        let label = |x: f64, text: String| {
            let (anchor, xx) = if x > W - 40.0 {
                ("end", x - 6.0)
            } else if x < pad_l + 40.0 {
                ("start", x + 6.0)
            } else {
                ("middle", x)
            };
            view! {
                <text x=xx y={H-PAD_B+14.0} class="crosshair" text-anchor=anchor>{text}</text>
            }
        };
        Some(view! {
            <rect
                x=x1
                y=PAD_T
                width={x2-x1}
                height={H-PAD_T-PAD_B}
                fill="#888"
                opacity="0.30"
            />
            <line x1=x1 x2=x1 y1=PAD_T y2={H-PAD_B} stroke="#666" stroke-width="1"/>
            <line x1=x2 x2=x2 y1=PAD_T y2={H-PAD_B} stroke="#666" stroke-width="1"/>
            {label(x1, d1)}
            {label(x2, d2)}
        })
    };

    let show_reset = move || {
        let zoomed = start.get() > 0 || end.get() < count.saturating_sub(1).max(1);
        hovering.get() || zoomed
    };

    view! {
        <div class="chart">
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
                    {(!title.is_empty()).then(|| view! {
                        <text x={W/2.0} y=18.0 class="chart-title" text-anchor="middle">{title.clone()}</text>
                    })}
                    {legend}
                    <g class="grid" stroke="#ddd">
                        <path d=grid_path fill="none"/>
                    </g>
                    {move || x_labels().into_iter().map(|(x, label, anchor)| view! {
                        <text x=x y={H-16.0} class="axis" text-anchor=anchor>{label}</text>
                    }).collect_view()}
                    {move || y_labels().into_iter().map(|(y, v, pad_l, decimals)| view! {
                        <text x={pad_l-8.0} y=y class="axis" text-anchor="end">{fmt_val(v, decimals)}</text>
                    }).collect_view()}
                    {move || if !x_label.is_empty() {
                        view! { <text x={W/2.0} y={H-4.0} class="axis" text-anchor="middle">{x_label.clone()}</text> }.into_any()
                    } else { ().into_any() }}
                    {move || if !y_label.is_empty() {
                        view! {
                            <text
                                transform=format!("rotate(-90 {left} {cy})", left=PAD_T, cy=H/2.0)
                                x={PAD_T}
                                y={H/2.0}
                                class="axis"
                                text-anchor="middle"
                            >{y_label.clone()}</text>
                        }.into_any()
                    } else { ().into_any() }}
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
