use fund_types::CurvePoint;
use leptos::prelude::*;
use std::sync::Arc;

const W: f64 = 800.0;
const H: f64 = 300.0;
const PAD_L: f64 = 60.0;
const PAD_R: f64 = 20.0;
const PAD_T: f64 = 20.0;
const PAD_B: f64 = 40.0;

fn x_pos(i: usize, s: usize, e: usize) -> f64 {
    let n = (e - s).max(1) as f64;
    let t = if e == s { 0.0 } else { (i - s) as f64 / n };
    PAD_L + t * (W - PAD_L - PAD_R)
}

fn y_pos(v: f64, lo: f64, hi: f64) -> f64 {
    let t = (v - lo) / (hi - lo);
    PAD_T + (1.0 - t) * (H - PAD_T - PAD_B)
}

fn y_range(points: &[CurvePoint], s: usize, e: usize) -> (f64, f64) {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for p in &points[s..=e] {
        lo = lo.min(p.market_value);
        hi = hi.max(p.market_value);
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

#[component]
pub fn Chart(points: Vec<CurvePoint>) -> impl IntoView {
    let points = Arc::new(points);
    let count = points.len();
    // Visible index window.
    let start = RwSignal::new(0usize);
    let end = RwSignal::new(count.saturating_sub(1).max(1));
    // Hovered index.
    let hover = RwSignal::new(None::<usize>);
    // Drag panning state.
    let dragging = RwSignal::new(false);
    let drag_start = RwSignal::new(0.0f64);
    // A y-domain refresh key, bumped on zoom so the signal updates.
    let zoom_key = RwSignal::new(0u32);

    let viewbox_x = |ev: &leptos::ev::MouseEvent| -> Option<f64> {
        use wasm_bindgen::JsCast;
        let target = ev.current_target()?;
        let rect = target
            .dyn_into::<web_sys::Element>()
            .ok()?
            .get_bounding_client_rect();
        if rect.width() <= 0.0 {
            return None;
        }
        Some((ev.client_x() as f64 - rect.left()) * W / rect.width())
    };

    let on_mousemove = move |ev: leptos::ev::MouseEvent| {
        if dragging.get() {
            let Some(x) = viewbox_x(&ev) else {
                return;
            };
            let dx = x - drag_start.get();
            drag_start.set(x);
            let span = (end.get() - start.get()).max(2) as f64;
            let shift = (dx / (W - PAD_L - PAD_R) * span).round() as isize;
            let s = start.get();
            let e = end.get();
            let ns = (s as isize + shift).clamp(0, count as isize - 1) as usize;
            let ne = (e as isize + shift).clamp(0, count as isize - 1) as usize;
            if ne > ns {
                start.set(ns);
                end.set(ne);
                zoom_key.update(|k| *k += 1);
            }
        } else if let Some(x) = viewbox_x(&ev) {
            let s = start.get();
            let e = end.get();
            let t = ((x - PAD_L) / (W - PAD_L - PAD_R)).clamp(0.0, 1.0);
            let i = (s as f64 + t * (e - s) as f64).round() as usize;
            hover.set(Some(i.min(e)));
        }
    };

    let on_leave = move |_| {
        hover.set(None);
    };

    let on_wheel = move |ev: leptos::ev::WheelEvent| {
        let s = start.get();
        let e = end.get();
        let delta = if ev.delta_y() > 0.0 { 1 } else { -1 };
        let span = (e - s).max(2) as isize;
        let grow = (span / 4).max(2);
        let (ns, ne) = if delta > 0 {
            let ns = (s as isize + grow).min(e as isize - 2) as usize;
            (ns, (e as isize - grow).max(ns as isize + 2) as usize)
        } else {
            let ns = (s as isize - grow).max(0) as usize;
            let ne = (e as isize + grow).min(count as isize - 1) as usize;
            (ns, ne)
        };
        start.set(ns);
        end.set(ne);
        zoom_key.update(|k| *k += 1);
        hover.set(None);
        ev.prevent_default();
    };

    let on_mousedown = move |ev: leptos::ev::MouseEvent| {
        if let Some(x) = viewbox_x(&ev) {
            dragging.set(true);
            drag_start.set(x);
        }
        ev.prevent_default();
    };

    let on_mouseup = move |_| {
        dragging.set(false);
    };

    let on_reset = move |_| {
        start.set(0);
        end.set(count.saturating_sub(1).max(1));
        zoom_key.update(|k| *k += 1);
        hover.set(None);
    };

    let path_points = Arc::clone(&points);
    let path = move || {
        let (s, e) = (start.get(), end.get());
        let (lo, hi) = y_range(&path_points, s, e);
        path_d(&path_points, s, e, lo, hi)
    };

    let grid_points = Arc::clone(&points);
    let grid_path = move || {
        let (s, e) = (start.get(), end.get());
        let (lo, hi) = y_range(&grid_points, s, e);
        grid_d(lo, hi)
    };

    let x_points = Arc::clone(&points);
    let x_labels = move || {
        let s = start.get();
        let e = end.get();
        (0..=4)
            .map(|k| {
                let i = s + ((e - s) as f64 * k as f64 / 4.0) as usize;
                let i = i.min(e);
                (
                    x_pos(i, s, e),
                    x_points.get(i).map(|p| p.date.clone()).unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>()
    };

    let y_points = Arc::clone(&points);
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

    let hover_line = move || {
        hover.get().map(|i| {
            let x = x_pos(i, start.get(), end.get());
            view! { <line x1=x x2=x y1=PAD_T y2={H-PAD_B} stroke="#666" stroke-width="1"/> }
        })
    };

    let hover_points = Arc::clone(&points);
    let hover_label = move || {
        hover.get().and_then(|i| hover_points.get(i)).map(|p| {
            let i = hover.get().unwrap();
            let x = x_pos(i, start.get(), end.get());
            let (s, e) = (start.get(), end.get());
            let (lo, hi) = y_range(&hover_points, s, e);
            let y = y_pos(p.market_value, lo, hi);
            let x = if x > W - 160.0 { x - 160.0 } else { x + 10.0 };
            let y = if y < 40.0 { y + 20.0 } else { y - 10.0 };
            view! {
                <text x=x y=y class="tooltip">
                    {format!("{}  {:.4}", p.date, p.market_value)}
                </text>
            }
        })
    };

    view! {
        <div>
            <svg
                viewBox=format!("0 0 {W} {H}")
                style="width:100%;height:auto;background:#fafafa;touch-action:none"
                on:mousemove=on_mousemove
                on:mouseleave=on_leave
                on:wheel=on_wheel
                on:mousedown=on_mousedown
                on:mouseup=on_mouseup
            >
                <g class="grid" stroke="#ddd">
                    <path d=grid_path fill="none"/>
                </g>
                {move || x_labels().into_iter().map(|(x, label)| view! {
                    <text x=x y={H-12.0} class="axis" text-anchor="middle">{label}</text>
                }).collect_view()}
                {move || y_labels().into_iter().map(|(y, v)| view! {
                    <text x={PAD_L-8.0} y=y class="axis" text-anchor="end">{format!("{v:.0}")}</text>
                }).collect_view()}
                <path d=path fill="none" stroke="#2b6cb0" stroke-width="2"/>
                {hover_line}
                {hover_label}
            </svg>
            <button on:click=on_reset>"Reset view"</button>
            <p class="hint">"Scroll to zoom, drag to pan, hover for values."</p>
        </div>
    }
}
