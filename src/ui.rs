use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{Document, Element, Event, HtmlCanvasElement, HtmlInputElement, MouseEvent};

use crate::dice::DieType;
use crate::round::{
    Xorshift64, analytical_trick_count_distribution, exact_single_trick_distribution, expected_score_for_bid,
    expected_tricks,
    monte_carlo_special_capture_stats, monte_carlo_trick_count_distribution, optimal_bid,
    round_count, top_opponent_hand_patterns,
};

const DIE_ENTRIES: &[(&str, DieType)] = &[
    ("minotaur", DieType::Minotaur),
    ("griffin", DieType::Griffin),
    ("mermaid", DieType::Mermaid),
    ("red", DieType::Red),
    ("yellow", DieType::Yellow),
    ("purple", DieType::Purple),
    ("gray", DieType::Gray),
];

const ACTIVE_PC: &str = "px-4 py-1.5 rounded-lg text-sm font-medium bg-indigo-600 text-white";
const INACTIVE_PC: &str =
    "px-4 py-1.5 rounded-lg text-sm font-medium bg-slate-700 text-slate-300 hover:bg-slate-600";
const ACTIVE_TAB: &str = "px-4 py-2 rounded-xl text-sm font-semibold bg-amber-500 text-slate-950";
const INACTIVE_TAB: &str =
    "px-4 py-2 rounded-xl text-sm font-semibold bg-slate-700 text-slate-300 hover:bg-slate-600";

pub fn init_ui() -> Result<(), JsValue> {
    let doc = document()?;
    setup_tabs(&doc)?;
    setup_player_count(&doc)?;
    setup_play_order(&doc)?;
    setup_calc_method(&doc)?;
    setup_die_spinners(&doc)?;
    setup_calculate_btn(&doc)?;
    setup_chart_hover(&doc)?;
    setup_resize_handler()?;
    sync_play_order_controls(&doc);
    sync_calc_method_controls(&doc);
    Ok(())
}

fn document() -> Result<Document, JsValue> {
    web_sys::window()
        .ok_or_else(|| JsValue::from_str("no window"))?
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))
}

fn get_el(doc: &Document, id: &str) -> Result<Element, JsValue> {
    doc.get_element_by_id(id)
        .ok_or_else(|| JsValue::from_str(&format!("#{id} not found")))
}

fn set_text(doc: &Document, id: &str, text: &str) {
    if let Some(el) = doc.get_element_by_id(id) {
        el.set_text_content(Some(text));
    }
}

fn get_text_i32(doc: &Document, id: &str) -> i32 {
    doc.get_element_by_id(id)
        .and_then(|el| el.text_content())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

fn get_input(doc: &Document, id: &str) -> Option<HtmlInputElement> {
    doc.get_element_by_id(id)
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
}

fn get_input_usize(doc: &Document, id: &str) -> usize {
    get_input(doc, id)
        .and_then(|el| el.value().trim().parse().ok())
        .unwrap_or(0)
}

fn get_input_u64(doc: &Document, id: &str) -> Option<u64> {
    get_input(doc, id).and_then(|el| {
        let value = el.value();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            trimmed.parse().ok()
        }
    })
}

fn get_hidden_i32(doc: &Document, id: &str) -> i32 {
    get_input(doc, id)
        .and_then(|el| el.value().trim().parse().ok())
        .unwrap_or(0)
}

fn set_hidden_i32(doc: &Document, id: &str, val: i32) {
    if let Some(inp) = get_input(doc, id) {
        inp.set_value(&val.to_string());
    }
}

fn get_hidden_string(doc: &Document, id: &str) -> String {
    get_input(doc, id).map(|el| el.value()).unwrap_or_default()
}

fn set_hidden_string(doc: &Document, id: &str, value: &str) {
    if let Some(inp) = get_input(doc, id) {
        inp.set_value(value);
    }
}

fn build_hand_from_spinners(doc: &Document) -> Vec<DieType> {
    DIE_ENTRIES
        .iter()
        .flat_map(|&(name, die_type)| {
            let count = get_text_i32(doc, &format!("die-count-{name}")) as usize;
            std::iter::repeat_n(die_type, count)
        })
        .collect()
}

fn format_die(dt: DieType) -> &'static str {
    match dt {
        DieType::Minotaur => "Minotaur",
        DieType::Griffin => "Griffin",
        DieType::Mermaid => "Mermaid",
        DieType::Red => "Red",
        DieType::Yellow => "Yellow",
        DieType::Purple => "Purple",
        DieType::Gray => "Gray",
    }
}

fn format_hand_pattern(hand: &[DieType]) -> String {
    let mut parts = Vec::new();
    for &dt in DieType::ALL.iter() {
        let count = hand.iter().filter(|&&d| d == dt).count();
        if count > 0 {
            parts.push(format!("{}x{}", format_die(dt), count));
        }
    }
    parts.join(", ")
}

fn update_hand_size_display(doc: &Document) {
    let max_hand_size =
        round_count((get_hidden_i32(doc, "current-player-count") as usize).clamp(3, 6)) as i32;
    let total: i32 = DIE_ENTRIES
        .iter()
        .map(|(name, _)| get_text_i32(doc, &format!("die-count-{name}")))
        .sum();
    set_text(
        doc,
        "hand-size-display",
        &format!("{total} / {max_hand_size}"),
    );
    set_hidden_i32(doc, "current-hand-size", total);
}

fn clamp_hand_size_to_round_limit(doc: &Document) {
    let max_hand_size =
        round_count((get_hidden_i32(doc, "current-player-count") as usize).clamp(3, 6)) as i32;
    let mut total: i32 = DIE_ENTRIES
        .iter()
        .map(|(name, _)| get_text_i32(doc, &format!("die-count-{name}")))
        .sum();

    if total <= max_hand_size {
        return;
    }

    for &(name, _) in DIE_ENTRIES.iter().rev() {
        let id = format!("die-count-{name}");
        let mut count = get_text_i32(doc, &id);
        while total > max_hand_size && count > 0 {
            count -= 1;
            total -= 1;
        }
        set_text(doc, &id, &count.to_string());
        if total <= max_hand_size {
            break;
        }
    }
}

fn setup_tabs(doc: &Document) -> Result<(), JsValue> {
    let name = "calculator";
    let btn = get_el(doc, &format!("tab-btn-{name}"))?;
    let tab_name = name.to_string();
    let cb = Closure::<dyn FnMut(Event)>::wrap(Box::new(move |_: Event| {
        let d = match document() {
            Ok(d) => d,
            Err(_) => return,
        };
        activate_tab(&d, &tab_name);
    }));
    btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
    cb.forget();
    activate_tab(doc, "calculator");
    Ok(())
}

fn activate_tab(doc: &Document, active: &str) {
    let name = "calculator";
    if let Some(btn) = doc.get_element_by_id(&format!("tab-btn-{name}")) {
        let _ = btn.set_attribute(
            "class",
            if name == active {
                ACTIVE_TAB
            } else {
                INACTIVE_TAB
            },
        );
    }
    if let Some(panel) = doc.get_element_by_id(&format!("tab-panel-{name}")) {
        let class_list = panel.class_list();
        if name == active {
            let _ = class_list.remove_1("hidden");
        } else {
            let _ = class_list.add_1("hidden");
        }
    }
}

fn setup_player_count(doc: &Document) -> Result<(), JsValue> {
    for n in 3u8..=6 {
        let el = get_el(doc, &format!("pc-btn-{n}"))?;
        let cb = Closure::<dyn FnMut(Event)>::wrap(Box::new(move |_: Event| {
            let d = match document() {
                Ok(d) => d,
                Err(_) => return,
            };
            set_hidden_i32(&d, "current-player-count", n as i32);
            for other in 3u8..=6 {
                if let Some(btn) = d.get_element_by_id(&format!("pc-btn-{other}")) {
                    let _ = btn
                        .set_attribute("class", if other == n { ACTIVE_PC } else { INACTIVE_PC });
                }
            }
            sync_play_order_controls(&d);
            clamp_hand_size_to_round_limit(&d);
            update_hand_size_display(&d);
        }));
        el.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
        cb.forget();
    }
    Ok(())
}

fn setup_play_order(doc: &Document) -> Result<(), JsValue> {
    for n in 0u8..6 {
        let el = get_el(doc, &format!("po-btn-{n}"))?;
        let cb = Closure::<dyn FnMut(Event)>::wrap(Box::new(move |_: Event| {
            let d = match document() {
                Ok(d) => d,
                Err(_) => return,
            };
            let player_count = (get_hidden_i32(&d, "current-player-count") as usize).clamp(3, 6);
            if (n as usize) < player_count {
                set_hidden_i32(&d, "current-play-order", n as i32);
                sync_play_order_controls(&d);
            }
        }));
        el.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
        cb.forget();
    }
    Ok(())
}

fn setup_calc_method(doc: &Document) -> Result<(), JsValue> {
    for (value, label) in [("dp", "calc-method-btn-dp"), ("monte-carlo", "calc-method-btn-mc")] {
        let el = get_el(doc, label)?;
        let value = value.to_string();
        let cb = Closure::<dyn FnMut(Event)>::wrap(Box::new(move |_: Event| {
            let d = match document() {
                Ok(d) => d,
                Err(_) => return,
            };
            set_hidden_string(&d, "current-calc-method", &value);
            sync_calc_method_controls(&d);
        }));
        el.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
        cb.forget();
    }
    Ok(())
}

fn sync_calc_method_controls(doc: &Document) {
    let method = match get_hidden_string(doc, "current-calc-method").as_str() {
        "monte-carlo" => "monte-carlo",
        _ => "dp",
    };
    set_hidden_string(doc, "current-calc-method", method);

    if let Some(btn) = doc.get_element_by_id("calc-method-btn-dp") {
        let _ = btn.set_attribute("class", if method == "dp" { ACTIVE_PC } else { INACTIVE_PC });
    }
    if let Some(btn) = doc.get_element_by_id("calc-method-btn-mc") {
        let _ = btn.set_attribute(
            "class",
            if method == "monte-carlo" { ACTIVE_PC } else { INACTIVE_PC },
        );
    }
}

fn sync_play_order_controls(doc: &Document) {
    let player_count = (get_hidden_i32(doc, "current-player-count") as usize).clamp(3, 6);
    let current = (get_hidden_i32(doc, "current-play-order") as usize).min(player_count - 1);
    set_hidden_i32(doc, "current-play-order", current as i32);

    for idx in 0..6usize {
        if let Some(btn) = doc.get_element_by_id(&format!("po-btn-{idx}")) {
            let _ = btn.set_attribute(
                "class",
                if idx == current {
                    ACTIVE_PC
                } else {
                    INACTIVE_PC
                },
            );
            let class_list = btn.class_list();
            if idx < player_count {
                let _ = class_list.remove_1("hidden");
            } else {
                let _ = class_list.add_1("hidden");
            }
        }
    }
}

fn setup_die_spinners(doc: &Document) -> Result<(), JsValue> {
    for &(name, die_type) in DIE_ENTRIES {
        setup_spinner(doc, name, die_type.bag_count() as i32)?;
    }
    Ok(())
}

fn setup_spinner(doc: &Document, name: &str, max: i32) -> Result<(), JsValue> {
    let count_id_inc = format!("die-count-{name}");
    let count_id_dec = count_id_inc.clone();
    let inc_id = format!("die-inc-{name}");
    let dec_id = format!("die-dec-{name}");

    let inc_cb = Closure::<dyn FnMut(Event)>::wrap(Box::new(move |_: Event| {
        let d = match document() {
            Ok(d) => d,
            Err(_) => return,
        };
        let cur = get_text_i32(&d, &count_id_inc);
        let hand_total: i32 = DIE_ENTRIES
            .iter()
            .map(|(name, _)| get_text_i32(&d, &format!("die-count-{name}")))
            .sum();
        let max_hand_size =
            round_count((get_hidden_i32(&d, "current-player-count") as usize).clamp(3, 6)) as i32;
        if hand_total < max_hand_size {
            set_text(&d, &count_id_inc, &(cur + 1).min(max).to_string());
        }
        update_hand_size_display(&d);
    }));
    if let Some(el) = doc.get_element_by_id(&inc_id) {
        el.add_event_listener_with_callback("click", inc_cb.as_ref().unchecked_ref())?;
    }
    inc_cb.forget();

    let dec_cb = Closure::<dyn FnMut(Event)>::wrap(Box::new(move |_: Event| {
        let d = match document() {
            Ok(d) => d,
            Err(_) => return,
        };
        let cur = get_text_i32(&d, &count_id_dec);
        set_text(&d, &count_id_dec, &(cur - 1).max(0).to_string());
        update_hand_size_display(&d);
    }));
    if let Some(el) = doc.get_element_by_id(&dec_id) {
        el.add_event_listener_with_callback("click", dec_cb.as_ref().unchecked_ref())?;
    }
    dec_cb.forget();
    Ok(())
}

fn setup_calculate_btn(doc: &Document) -> Result<(), JsValue> {
    let btn = get_el(doc, "calc-btn")?;
    let cb = Closure::<dyn FnMut(Event)>::wrap(Box::new(move |_: Event| {
        let d = match document() {
            Ok(d) => d,
            Err(_) => return,
        };
        if let Err(e) = do_calculate(&d) {
            web_sys::console::error_1(&e);
        }
    }));
    btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
    cb.forget();
    Ok(())
}

fn setup_chart_hover(doc: &Document) -> Result<(), JsValue> {
    let canvas = get_el(doc, "chart")?;
    let move_cb = Closure::<dyn FnMut(MouseEvent)>::wrap(Box::new(move |event: MouseEvent| {
        let d = match document() {
            Ok(d) => d,
            Err(_) => return,
        };
        if let Err(e) = update_chart_tooltip(&d, &event) {
            web_sys::console::error_1(&e);
        }
    }));
    canvas.add_event_listener_with_callback("mousemove", move_cb.as_ref().unchecked_ref())?;
    move_cb.forget();

    let leave_cb = Closure::<dyn FnMut(MouseEvent)>::wrap(Box::new(move |_: MouseEvent| {
        let d = match document() {
            Ok(d) => d,
            Err(_) => return,
        };
        hide_chart_tooltip(&d);
    }));
    canvas.add_event_listener_with_callback("mouseleave", leave_cb.as_ref().unchecked_ref())?;
    leave_cb.forget();

    Ok(())
}

fn setup_resize_handler() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let cb = Closure::<dyn FnMut(Event)>::wrap(Box::new(move |_: Event| {
        let d = match document() {
            Ok(d) => d,
            Err(_) => return,
        };
        if let Err(e) = redraw_chart_from_state(&d) {
            web_sys::console::error_1(&e);
        }
    }));
    window.add_event_listener_with_callback("resize", cb.as_ref().unchecked_ref())?;
    cb.forget();
    Ok(())
}

fn do_calculate(doc: &Document) -> Result<(), JsValue> {
    let hand = build_hand_from_spinners(doc);
    if hand.is_empty() {
        let btn = get_el(doc, "calc-btn")?;
        btn.set_text_content(Some("Add dice to your hand first"));
        let btn_clone = btn.clone();
        let timeout_cb = Closure::once(Box::new(move || {
            btn_clone.set_text_content(Some("Calculate Distribution"));
        }) as Box<dyn FnOnce()>);
        web_sys::window()
            .ok_or_else(|| JsValue::from_str("no window"))?
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                timeout_cb.as_ref().unchecked_ref(),
                2000,
            )?;
        timeout_cb.forget();
        return Ok(());
    }

    let player_count = (get_hidden_i32(doc, "current-player-count") as usize).clamp(3, 6);
    let player_position =
        (get_hidden_i32(doc, "current-play-order") as usize).min(player_count - 1);
    let calc_method = get_hidden_string(doc, "current-calc-method");
    let replications = get_input_usize(doc, "calc-replications-input").max(1000);
    let hand_size = hand.len();
    let dist = if hand_size == 1 {
        exact_single_trick_distribution(hand[0], player_count, player_position)
    } else if calc_method == "monte-carlo" {
        let seed =
            get_input_u64(doc, "calc-seed-input").unwrap_or_else(|| js_sys::Date::now() as u64);
        let mut rng = Xorshift64::new(seed);
        monte_carlo_trick_count_distribution(
            &hand,
            player_count,
            player_position,
            replications,
            &mut rng,
        )
    } else {
        analytical_trick_count_distribution(&hand, player_count, player_position)
    };
    let special_capture_stats = if hand
        .iter()
        .any(|&die| matches!(die, DieType::Mermaid | DieType::Minotaur))
    {
        let seed = get_input_u64(doc, "calc-seed-input").unwrap_or(1);
        let mut rng = Xorshift64::new(seed.wrapping_add(0x9e37_79b9_7f4a_7c15));
        Some(monte_carlo_special_capture_stats(
            &hand,
            player_count,
            player_position,
            replications,
            &mut rng,
        ))
    } else {
        None
    };
    let bid = optimal_bid(&dist);
    get_el(doc, "results-section")?
        .class_list()
        .remove_1("hidden")?;
    set_text(doc, "bid-value", &bid.to_string());
    set_text(doc, "bid-details", "");
    store_chart_state(doc, &dist, bid);
    redraw_chart_from_state(doc)?;

    let tbody = get_el(doc, "dist-tbody")?;
    tbody.set_inner_html("");
    for k in 0..=hand_size {
        let p = dist[k];
        let score = expected_score_for_bid(k, &dist, hand_size);
        let row = doc.create_element("tr")?;
        let (row_class, star) = if k == bid {
            (
                "border-b border-slate-600 bg-amber-900/40 font-semibold",
                "★ ",
            )
        } else {
            ("border-b border-slate-700", "")
        };
        row.set_attribute("class", row_class)?;
        row.set_inner_html(&format!(
            "<td class=\"py-1 pl-3\">{star}{k}</td>\
             <td class=\"py-1 text-center\">{:.1}%</td>\
             <td class=\"py-1 text-right pr-3\">{:+.1}</td>",
            p * 100.0,
            score
        ));
        tbody.append_child(&row)?;
    }

    set_hidden_i32(doc, "current-hand-size", hand_size as i32);
    render_special_capture_stats(doc, &hand, special_capture_stats)?;
    render_top_patterns(doc, &hand)?;
    Ok(())
}

fn render_top_patterns(doc: &Document, hand: &[DieType]) -> Result<(), JsValue> {
    let list = get_el(doc, "top-opponent-patterns")?;
    list.set_inner_html("");
    if hand.is_empty() {
        list.set_inner_html(
            "<li class=\"text-slate-500\">Select a hand to inspect opponent patterns.</li>",
        );
        return Ok(());
    }

    let patterns = top_opponent_hand_patterns(hand, hand.len(), 3);
    for pattern in patterns {
        let li = doc.create_element("li")?;
        li.set_attribute(
            "class",
            "flex items-start justify-between gap-4 rounded-lg border border-slate-700 bg-slate-800/70 px-3 py-2",
        )?;
        li.set_inner_html(&format!(
            "<span class=\"text-slate-200\">{}</span><span class=\"text-amber-400 font-semibold\">{:.2}%</span>",
            format_hand_pattern(&pattern.hand),
            pattern.probability * 100.0
        ));
        list.append_child(&li)?;
    }
    Ok(())
}

fn render_special_capture_stats(
    doc: &Document,
    hand: &[DieType],
    stats: Option<crate::round::SpecialCaptureStats>,
) -> Result<(), JsValue> {
    let section = get_el(doc, "special-capture-section")?;
    let list = get_el(doc, "special-capture-list")?;
    list.set_inner_html("");

    let has_mermaid = hand.contains(&DieType::Mermaid);
    let has_minotaur = hand.contains(&DieType::Minotaur);
    if !(has_mermaid || has_minotaur) {
        section.class_list().add_1("hidden")?;
        return Ok(());
    }

    section.class_list().remove_1("hidden")?;
    if let Some(stats) = stats {
        if has_mermaid {
            let item = doc.create_element("li")?;
            item.set_attribute("class", "flex items-start justify-between gap-4 rounded-lg border border-slate-700 bg-slate-800/70 px-3 py-2")?;
            item.set_inner_html(&format!(
                "<span class=\"text-slate-200\">Mermaid captures a Minotaur</span><span class=\"text-amber-400 font-semibold\">{:.2}%</span>",
                stats.mermaid_captures_minotaur_prob * 100.0
            ));
            list.append_child(&item)?;
        }
        if has_minotaur {
            let item = doc.create_element("li")?;
            item.set_attribute("class", "flex items-start justify-between gap-4 rounded-lg border border-slate-700 bg-slate-800/70 px-3 py-2")?;
            item.set_inner_html(&format!(
                "<span class=\"text-slate-200\">Minotaur captures a Griffin</span><span class=\"text-amber-400 font-semibold\">{:.2}%</span>",
                stats.minotaur_captures_griffin_prob * 100.0
            ));
            list.append_child(&item)?;
        }

        let bonus = doc.create_element("li")?;
        bonus.set_attribute("class", "flex items-start justify-between gap-4 rounded-lg border border-slate-700 bg-slate-800/70 px-3 py-2")?;
        bonus.set_inner_html(&format!(
            "<span class=\"text-slate-200\">Expected bonus points from special captures</span><span class=\"text-amber-400 font-semibold\">{:+.2}</span>",
            stats.expected_bonus_points
        ));
        list.append_child(&bonus)?;
    }

    Ok(())
}

fn store_chart_state(doc: &Document, dist: &[f64], optimal_bid: usize) {
    let encoded = dist
        .iter()
        .map(|value| format!("{value:.12}"))
        .collect::<Vec<_>>()
        .join(",");
    set_hidden_string(doc, "last-dist", &encoded);
    set_hidden_string(doc, "last-optimal-bid", &optimal_bid.to_string());
}

fn redraw_chart_from_state(doc: &Document) -> Result<(), JsValue> {
    let dist_encoded = get_hidden_string(doc, "last-dist");
    if dist_encoded.trim().is_empty() {
        return Ok(());
    }
    let dist: Vec<f64> = dist_encoded
        .split(',')
        .filter_map(|s| s.parse::<f64>().ok())
        .collect();
    if dist.is_empty() {
        return Ok(());
    }
    let optimal_bid = get_hidden_string(doc, "last-optimal-bid")
        .parse::<usize>()
        .unwrap_or(0);
    let exp_tricks = expected_tricks(&dist);
    hide_chart_tooltip(doc);
    crate::chart::draw_trick_distribution("chart", &dist, optimal_bid, exp_tricks)
}

fn update_chart_tooltip(doc: &Document, event: &MouseEvent) -> Result<(), JsValue> {
    let dist_encoded = get_hidden_string(doc, "last-dist");
    if dist_encoded.trim().is_empty() {
        hide_chart_tooltip(doc);
        return Ok(());
    }

    let dist: Vec<f64> = dist_encoded
        .split(',')
        .filter_map(|s| s.parse::<f64>().ok())
        .collect();
    if dist.is_empty() {
        hide_chart_tooltip(doc);
        return Ok(());
    }

    let canvas: HtmlCanvasElement = get_el(doc, "chart")?
        .dyn_into()
        .map_err(|_| JsValue::from_str("#chart is not a canvas"))?;
    let tooltip = get_el(doc, "chart-tooltip")?;

    let width = canvas.client_width() as f64;
    let height = canvas.client_height() as f64;
    if width <= 0.0 || height <= 0.0 {
        hide_chart_tooltip(doc);
        return Ok(());
    }

    let plot_left = 66.0;
    let plot_right = (width - 16.0).max(plot_left + 1.0);
    let plot_width = plot_right - plot_left;
    let x = event.offset_x() as f64;
    let y = event.offset_y() as f64;
    if x < plot_left || x > plot_right || y < 20.0 || y > height - 36.0 {
        hide_chart_tooltip(doc);
        return Ok(());
    }

    let n = dist.len() as f64;
    let hovered = (((x - plot_left) / plot_width) * n).floor() as isize;
    if hovered < 0 || hovered as usize >= dist.len() {
        hide_chart_tooltip(doc);
        return Ok(());
    }
    let hovered = hovered as usize;
    let optimal_bid = get_hidden_string(doc, "last-optimal-bid")
        .parse::<usize>()
        .unwrap_or(0);

    tooltip.set_inner_html(&format!(
        "<div class=\"font-semibold text-amber-300\">Tricks: {}</div>\
         <div>Probability: {:.2}%</div>\
         <div>{}</div>",
        hovered,
        dist[hovered] * 100.0,
        if hovered == optimal_bid {
            "Recommended bid"
        } else {
            "Not the recommended bid"
        }
    ));

    let left = (x + 14.0).min(width - 120.0).max(8.0);
    let top = (y + 14.0).min(height - 70.0).max(8.0);
    tooltip.set_attribute("style", &format!("left: {left}px; top: {top}px;"))?;
    tooltip.class_list().remove_1("hidden")?;
    Ok(())
}

fn hide_chart_tooltip(doc: &Document) {
    if let Some(tooltip) = doc.get_element_by_id("chart-tooltip") {
        let _ = tooltip.class_list().add_1("hidden");
    }
}
