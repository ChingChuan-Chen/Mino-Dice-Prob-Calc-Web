use plotters::prelude::*;
use plotters::style::text_anchor::{HPos, Pos, VPos};
use plotters_canvas::CanvasBackend;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

const CHART_BG: RGBColor = RGBColor(15, 23, 42);
const CHART_PANEL: RGBColor = RGBColor(30, 41, 59);
const CHART_GRID: RGBColor = RGBColor(71, 85, 105);
const CHART_TEXT: RGBColor = RGBColor(226, 232, 240);
const CHART_MUTED: RGBColor = RGBColor(148, 163, 184);

/// Colours for each trick count bar (cycles if more than 8 tricks).
const BAR_COLORS: &[RGBColor] = &[
    RGBColor(99, 102, 241), // indigo
    RGBColor(59, 130, 246), // blue
    RGBColor(16, 185, 129), // emerald
    RGBColor(245, 158, 11), // amber
    RGBColor(239, 68, 68),  // red
    RGBColor(168, 85, 247), // purple
    RGBColor(20, 184, 166), // teal
    RGBColor(251, 146, 60), // orange
    RGBColor(34, 197, 94),  // green
];

/// Draw a probability bar chart onto the canvas element with the given id.
///
/// * `canvas_id`  – DOM id of the `<canvas>` element.
/// * `dist`       – P(tricks = k) for k = 0..=hand_size; length = hand_size + 1.
/// * `optimal_bid` – highlighted bar index.
pub fn draw_trick_distribution(
    canvas_id: &str,
    dist: &[f64],
    optimal_bid: usize,
) -> Result<(), JsValue> {
    resize_canvas_to_client_width(canvas_id, 0.5)?;

    let backend = CanvasBackend::new(canvas_id)
        .ok_or_else(|| JsValue::from_str(&format!("Canvas '{canvas_id}' not found")))?;

    let root = backend.into_drawing_area();
    root.fill(&CHART_BG)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let max_p = dist.iter().cloned().fold(0.0f64, f64::max).max(0.01);
    let n = dist.len();

    let mut chart = ChartBuilder::on(&root)
        .caption(
            "Trick-Count Probability",
            ("sans-serif", 16).into_font().color(&CHART_TEXT),
        )
        .margin(16)
        .x_label_area_size(36)
        .y_label_area_size(50)
        .build_cartesian_2d(-0.5f64..(n as f64 - 0.5), 0f64..(max_p * 1.15))
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    chart
        .plotting_area()
        .fill(&CHART_PANEL)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    chart
        .configure_mesh()
        .x_desc("Tricks won")
        .y_desc("Probability")
        .x_labels(n)
        .axis_style(CHART_MUTED.stroke_width(1))
        .light_line_style(CHART_GRID.mix(0.18))
        .bold_line_style(CHART_GRID.mix(0.30))
        .label_style(("sans-serif", 12).into_font().color(&CHART_TEXT))
        .axis_desc_style(("sans-serif", 12).into_font().color(&CHART_TEXT))
        .x_label_formatter(&|v: &f64| {
            let idx = v.round() as isize;
            if idx >= 0 && (idx as usize) < n {
                idx.to_string()
            } else {
                String::new()
            }
        })
        .y_label_formatter(&|v| format!("{:.0}%", v * 100.0))
        .draw()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Draw bars.
    for (k, &p) in dist.iter().enumerate() {
        let color = if k == optimal_bid {
            RGBColor(234, 88, 12) // orange highlight for optimal bid
        } else {
            BAR_COLORS[k % BAR_COLORS.len()]
        };
        let bar_color = color.mix(0.85);

        chart
            .draw_series(std::iter::once(Rectangle::new(
                [(k as f64 - 0.4, 0.0), (k as f64 + 0.4, p)],
                bar_color.filled(),
            )))
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Probability label above bar.
        if p > 0.005 {
            chart
                .draw_series(std::iter::once(Text::new(
                    format!("{:.1}%", p * 100.0),
                    (k as f64, p + max_p * 0.035),
                    ("sans-serif", 11)
                        .into_font()
                        .color(&CHART_TEXT)
                        .pos(Pos::new(HPos::Center, VPos::Bottom)),
                )))
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
        }
    }

    root.present()
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(())
}

fn resize_canvas_to_client_width(canvas_id: &str, aspect_ratio: f64) -> Result<(), JsValue> {
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("document unavailable"))?;
    let canvas: HtmlCanvasElement = document
        .get_element_by_id(canvas_id)
        .ok_or_else(|| JsValue::from_str(&format!("Canvas '{canvas_id}' not found")))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("element is not a canvas"))?;

    let client_width = canvas.client_width().max(320) as u32;
    let height = ((client_width as f64) * aspect_ratio).round().max(180.0) as u32;
    if canvas.width() != client_width {
        canvas.set_width(client_width);
    }
    if canvas.height() != height {
        canvas.set_height(height);
    }
    Ok(())
}
