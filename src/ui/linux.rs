//! Native GTK4 interface for Linux.
//!
//! The Linux build deliberately uses gtk4-rs directly.  It does not route
//! through wxWidgets, raw GTK FFI, compatibility shims, or fallback widgets.
//! Fixed GTK layouts preserve the recovered Windows 95 Calculator geometry,
//! while one application CSS provider supplies the classic palette and bevels.

use crate::calc::{Base, BinaryOp, Calculator, Mode};
use crate::calculation_log::CalculationLog;
use crate::expr::AngleMode;
use crate::graph::{format_root_values, ExportFormat, GraphModel, RootResult, Viewport};
use crate::history::History;
use crate::i18n::{Language, Strings};
use crate::platform;
use crate::settings::{DecimalSeparator, Settings};
use crate::tooltip::TooltipCatalog;
use ashpd::desktop::file_chooser::{FileFilter as PortalFileFilter, SelectedFiles};
use ashpd::desktop::ResponseError as PortalResponseError;
use ashpd::{Error as PortalError, PortalError as PortalServiceError, WindowIdentifier};
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use plotters::drawing::IntoDrawingArea;
use plotters_cairo::CairoBackend;
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use x11rb::connection::Connection as _;
use x11rb::protocol::xproto::{ConfigureWindowAux, ConnectionExt as _};

const SOURCE_DPI: i32 = 96;
const DESIGN_DPI: i32 = 120;
const fn dp(value: i32) -> i32 {
    (value * DESIGN_DPI + SOURCE_DPI / 2) / SOURCE_DPI
}

const STD_W: i32 = dp(260);
const STD_H: i32 = dp(204);
const SCI_W: i32 = dp(500);
const SCI_H: i32 = dp(304);
const GRAPH_W: i32 = dp(330);
const HISTORY_W: i32 = dp(210);
const PANEL_SEPARATOR_W: i32 = 2;

const SCI_DISPLAY_Y: i32 = 24;
const SCI_SELECTOR_BOX_Y: i32 = 58;
const SCI_SELECTOR_Y: i32 = 66;
const SCI_COMMAND_BOX_Y: i32 = 94;
const SCI_COMMAND_Y: i32 = 99;
const SCI_CHECK_Y: i32 = 103;
const SCI_CHECK_H: i32 = 18;
const SCI_CHECK_W: i32 = 48;
const SCI_KEYPAD_Y: i32 = 133;
const SCI_KEYPAD_STEP: i32 = 34;

const CLASSIC_CSS: &str = r#"
window.opencalc-window,
box.opencalc-root,
box.calc-content,
fixed.calc-surface,
box.side-panel,
box.classic-menu-bar,
popover.classic-menu-popover > contents,
popover.classic-context-menu > contents {
    background: #f0f0f0;
    color: #000000;
    font-family: "Liberation Sans", sans-serif;
    font-size: 12px;
}

headerbar.opencalc-titlebar {
    min-height: 24px;
    padding: 0 4px;
    background: #c0c0c0;
    background-image: none;
    color: #000000;
    border: 0;
    border-radius: 0;
    box-shadow: none;
}

headerbar.opencalc-titlebar label.title {
    font-weight: 700;
}

headerbar.opencalc-titlebar windowcontrols button {
    min-width: 24px;
    min-height: 20px;
    margin: 2px 1px;
    padding: 0;
    border-radius: 0;
}

box.classic-menu-bar {
    min-height: 24px;
    padding: 0 2px;
    border-bottom: 1px solid #808080;
}

menubutton.classic-menu-button > button,
button.classic-menu-item {
    background: transparent;
    background-image: none;
    border: 0;
    border-radius: 0;
    box-shadow: none;
    color: #000000;
    padding: 3px 8px;
    min-height: 18px;
}

menubutton.classic-menu-button > button:hover,
menubutton.classic-menu-button > button:checked,
button.classic-menu-item:hover {
    background: #000080;
    color: #ffffff;
}

button.classic-button,
button.classic-button:hover,
button.classic-button:focus,
button.classic-button:focus-visible,
button.classic-button:active,
button.classic-button:disabled {
    background: #f0f0f0;
    background-color: #f0f0f0;
    background-image: none;
    border-radius: 0;
    border-style: solid;
    border-width: 1px;
    border-color: #ffffff #808080 #808080 #ffffff;
    box-shadow: inset -1px -1px #404040, inset 1px 1px #ffffff;
    padding: 0;
    min-height: 0;
    min-width: 0;
    font-weight: 700;
    opacity: 1;
}
button.classic-button:active {
    border-color: #808080 #ffffff #ffffff #808080;
    box-shadow: inset 1px 1px #404040;
    padding: 1px 0 0 1px;
}
button.tone-red { color: #ff0000; }
button.tone-blue { color: #0000ff; }
button.tone-navy { color: #000080; }
button.tone-magenta { color: #800080; }
button.tone-maroon { color: #800000; }
button.classic-button:disabled { color: #808080; }

frame.classic-field,
frame.classic-indicator,
frame.graph-frame {
    background: #ffffff;
    border-radius: 0;
    border-style: solid;
    border-width: 1px;
    border-color: #808080 #ffffff #ffffff #808080;
    box-shadow: inset 1px 1px #404040, inset -1px -1px #dfdfdf;
    padding: 0;
}
frame.classic-indicator { background: #f0f0f0; }
label.classic-display {
    background: #ffffff;
    color: #000000;
    padding: 2px 5px;
    font-size: 15px;
}
label.classic-indicator-label { padding: 0; }

entry.graph-entry {
    background: #ffffff;
    color: #000000;
    border-radius: 0;
    border-style: solid;
    border-width: 1px;
    border-color: #808080 #ffffff #ffffff #808080;
    box-shadow: inset 1px 1px #404040, inset -1px -1px #dfdfdf;
    padding: 2px 5px;
}

frame.classic-group {
    background: transparent;
    border-radius: 0;
    border: 1px solid;
    border-color: #808080 #ffffff #ffffff #808080;
    padding: 0;
}

separator.classic-separator {
    background: #808080;
    border: 0;
    min-height: 1px;
}

box.side-panel {
    padding: 8px;
}
separator.panel-separator {
    background: #808080;
    border: 0;
    box-shadow: inset -1px 0 #ffffff;
    min-width: 2px;
}
label.panel-title { font-weight: 700; }
label.graph-status { font-size: 11px; }

scrolledwindow.classic-scroll,
listbox.classic-list {
    background: #ffffff;
    color: #000000;
    border-radius: 0;
}
listbox.classic-list row {
    padding: 5px;
    border-bottom: 1px solid #d0d0d0;
}
listbox.classic-list row:selected {
    background: #000080;
    color: #ffffff;
}

checkbutton.classic-check,
checkbutton.classic-radio {
    color: #000000;
    padding: 0;
}

popover.classic-menu-popover,
popover.classic-context-menu,
popover.classic-help-popup {
    background: transparent;
    margin: 0;
    padding: 0;
}

popover.classic-menu-popover > contents,
popover.classic-context-menu > contents {
    border-radius: 0;
    border: 2px solid;
    border-color: #ffffff #404040 #404040 #ffffff;
    box-shadow: inset -1px -1px #808080;
    margin: 0;
    padding: 2px;
}

button.classic-context-item {
    background: transparent;
    background-image: none;
    border: 0;
    border-radius: 0;
    box-shadow: none;
    color: #000000;
    padding: 3px 14px;
    min-height: 18px;
}
button.classic-context-item:hover {
    background: #000080;
    color: #ffffff;
}


popover.classic-help-popup > contents {
    background: #ffffe1;
    border: 1px solid #000000;
    border-radius: 0;
    box-shadow: none;
    padding: 0;
}
label.classic-help-label {
    background: #ffffe1;
    color: #000000;
    padding: 4px 6px;
}
"#;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Digit(char),
    Dot,
    Back,
    CE,
    C,
    Sign,
    Eq,
    Percent,
    Bin(BinaryOp),
    KeyboardStar,
    Unary(&'static str),
    MemC,
    MemR,
    MemS,
    MemAdd,
    Pi,
    Open,
    Close,
    StatsOpen,
    StatsDat,
    StatsAvg,
    StatsSum,
    StatsDev,
    ToggleFE,
    Copy,
    Paste,
    About,
    Help,
}

#[derive(Clone, Copy)]
enum Tone {
    Red,
    Blue,
    Navy,
    Magenta,
    Maroon,
}

#[derive(Clone, Copy)]
struct ButtonDef {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    label: &'static str,
    action: Action,
    tone: Tone,
}

struct MenuHandles {
    edit_button: gtk::MenuButton,
    edit_label: gtk::Label,
    view_button: gtk::MenuButton,
    view_label: gtk::Label,
    help_button: gtk::MenuButton,
    help_label: gtk::Label,
    edit_popover: gtk::Popover,
    view_popover: gtk::Popover,
    help_popover: gtk::Popover,
    undo: gtk::Button,
    redo: gtk::Button,
    copy: gtk::Button,
    paste: gtk::Button,
    standard: gtk::Button,
    scientific: gtk::Button,
    graph: gtk::Button,
    history: gtk::Button,
    period: gtk::Button,
    comma: gtk::Button,
    english: gtk::Button,
    portuguese: gtk::Button,
    spanish: gtk::Button,
    help_topics: gtk::Button,
    about: gtk::Button,
}

struct CalculatorPanels {
    standard: gtk::Fixed,
    scientific: gtk::Fixed,
    standard_display: gtk::Label,
    scientific_display: gtk::Label,
    standard_memory: gtk::Label,
    scientific_memory: gtk::Label,
    scientific_parens: gtk::Label,
    inv: gtk::CheckButton,
    hyp: gtk::CheckButton,
    base_radios: [gtk::CheckButton; 4],
    angle_radios: [gtk::CheckButton; 3],
    action_buttons: Vec<(gtk::Button, Action)>,
    tooltip_targets: Vec<(gtk::Widget, &'static str)>,
}

struct GraphPanel {
    panel: gtk::Box,
    function_label: gtk::Label,
    expression: gtk::Entry,
    plot_button: gtk::Button,
    canvas: gtk::DrawingArea,
    roots: gtk::Label,
    reset_button: gtk::Button,
    export_button: gtk::Button,
    model: Rc<RefCell<GraphModel>>,
    drag_initial: Rc<RefCell<Option<Viewport>>>,
    pointer: Rc<Cell<(f64, f64)>>,
}

struct HistoryPanel {
    panel: gtk::Box,
    title: gtk::Label,
    list: gtk::ListBox,
    clear_button: gtk::Button,
}

struct StatsBox {
    window: gtk::Window,
    list: gtk::ListBox,
    count: gtk::Label,
}

struct Ui {
    window: gtk::ApplicationWindow,
    root: gtk::Box,
    content: gtk::Box,
    graph_separator: gtk::Separator,
    history_separator: gtk::Separator,
    calculator_host: gtk::Box,
    standard_panel: gtk::Fixed,
    scientific_panel: gtk::Fixed,
    standard_display: gtk::Label,
    scientific_display: gtk::Label,
    standard_memory: gtk::Label,
    scientific_memory: gtk::Label,
    scientific_parens: gtk::Label,
    inv: gtk::CheckButton,
    hyp: gtk::CheckButton,
    base_radios: [gtk::CheckButton; 4],
    angle_radios: [gtk::CheckButton; 3],
    action_buttons: Vec<(gtk::Button, Action)>,
    graph_panel: GraphPanel,
    history_panel: HistoryPanel,
    menus: MenuHandles,
    calc: RefCell<Calculator>,
    history: RefCell<History<Calculator>>,
    calculation_log: RefCell<CalculationLog>,
    settings: RefCell<Settings>,
    tooltips: TooltipCatalog,
    tooltip_targets: Vec<(gtk::Widget, &'static str)>,
    stats_box: RefCell<Option<StatsBox>>,
}

pub fn run() -> Result<(), String> {
    let app = gtk::Application::builder()
        .application_id("io.github.opencalc.OpenCalc")
        .build();

    app.connect_activate(|app| build_application(app));
    let _ = app.run();
    Ok(())
}

fn build_application(app: &gtk::Application) {
    configure_font_rendering();
    install_classic_theme();

    let mut calculator = Calculator::default();
    let settings = Settings::load(calculator.decimal_separator());
    calculator.set_decimal_separator(settings.decimal_separator.as_char());
    calculator.set_mode(settings.mode);
    let strings = Strings::new(settings.language);
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title(strings.calculator_title())
        .resizable(false)
        .build();
    window.add_css_class("opencalc-window");

    // Use GTK's own title bar so the fixed-size policy is enforced by GTK on
    // both Wayland and X11 instead of depending on server-side decorations.
    let title_bar = gtk::HeaderBar::new();
    title_bar.add_css_class("opencalc-titlebar");
    title_bar.set_show_title_buttons(true);
    title_bar.set_decoration_layout(Some("icon:minimize,close"));
    window.set_titlebar(Some(&title_bar));

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.add_css_class("opencalc-root");
    root.set_focusable(true);
    window.set_child(Some(&root));

    let (menu_bar, menus) = build_menu_bar(strings);
    root.append(&menu_bar);

    let content = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    content.add_css_class("calc-content");
    content.set_halign(gtk::Align::Center);
    content.set_valign(gtk::Align::Center);
    content.set_vexpand(true);
    root.append(&content);

    let tooltips = TooltipCatalog::load_default();
    let mut calculator_panels = build_calculator_panels(&calculator, settings.decimal_separator);
    let graph_panel = build_graph_panel(strings);
    let history_panel = build_history_panel(strings);

    let calculator_host = gtk::Box::new(gtk::Orientation::Vertical, 0);
    calculator_host.append(&calculator_panels.standard);
    calculator_host.append(&calculator_panels.scientific);

    let graph_separator = panel_separator();
    let history_separator = panel_separator();

    content.append(&graph_panel.panel);
    content.append(&graph_separator);
    content.append(&calculator_host);
    content.append(&history_separator);
    content.append(&history_panel.panel);

    let mut tooltip_targets = Vec::new();
    tooltip_targets.append(&mut calculator_panels.tooltip_targets);
    tooltip_targets.extend([
        (graph_panel.expression.clone().upcast(), "graph_function"),
        (graph_panel.plot_button.clone().upcast(), "graph_plot"),
        (graph_panel.reset_button.clone().upcast(), "graph_reset"),
        (graph_panel.export_button.clone().upcast(), "graph_export"),
        (history_panel.clear_button.clone().upcast(), "history_clear"),
    ]);

    let ui = Rc::new(Ui {
        window,
        root,
        content,
        graph_separator,
        history_separator,
        calculator_host,
        standard_panel: calculator_panels.standard,
        scientific_panel: calculator_panels.scientific,
        standard_display: calculator_panels.standard_display,
        scientific_display: calculator_panels.scientific_display,
        standard_memory: calculator_panels.standard_memory,
        scientific_memory: calculator_panels.scientific_memory,
        scientific_parens: calculator_panels.scientific_parens,
        inv: calculator_panels.inv,
        hyp: calculator_panels.hyp,
        base_radios: calculator_panels.base_radios,
        angle_radios: calculator_panels.angle_radios,
        action_buttons: calculator_panels.action_buttons,
        graph_panel,
        history_panel,
        menus,
        calc: RefCell::new(calculator),
        history: RefCell::new(History::default()),
        calculation_log: RefCell::new(CalculationLog::default()),
        settings: RefCell::new(settings),
        tooltips,
        tooltip_targets,
        stats_box: RefCell::new(None),
    });

    ui.window.connect_close_request(|window| {
        window.destroy();
        glib::Propagation::Stop
    });

    bind_action_buttons(&ui);
    bind_selectors(&ui);
    bind_menu(&ui);
    bind_keyboard(&ui, &ui.window);
    bind_graph(&ui);
    bind_history(&ui);
    bind_context_help(&ui);
    bind_outside_entry_focus(&ui);

    apply_language(&ui);
    refresh(&ui);
    refresh_calculation_history(&ui);
    let mode = ui.calc.borrow().mode;
    sync_surface(&ui, mode);

    // GTK owns the title bar and resize controls, so resizable(false) is the
    // single fixed-size policy on every backend. X11 positioning remains a
    // separate one-shot operation; Wayland deliberately owns placement.
    install_startup_centering(&ui.window);
    ui.window.unmaximize();
    ui.window.present();
    let root = ui.root.clone();
    glib::idle_add_local_once(move || {
        root.grab_focus();
    });
}

fn install_startup_centering(window: &gtk::ApplicationWindow) {
    let centered = Rc::new(Cell::new(false));
    let centered_on_map = Rc::clone(&centered);

    window.connect_map(move |window| {
        if centered_on_map.replace(true) {
            return;
        }

        // Run after the map cycle so the final client-decorated size can be
        // queried before positioning it on X11. Wayland owns positioning.
        let window = window.clone();
        glib::idle_add_local_once(move || center_mapped_x11_window(&window));
    });
}

fn center_mapped_x11_window(window: &gtk::ApplicationWindow) {
    let Some(surface) = window.surface() else {
        return;
    };
    let Ok(x11_surface) = surface.clone().downcast::<gdk4_x11::X11Surface>() else {
        // Wayland deliberately offers no global top-level coordinates.
        return;
    };
    let Some(monitor) = surface.display().monitor_at_surface(&surface) else {
        return;
    };
    let Ok(xid) = u32::try_from(x11_surface.xid()) else {
        return;
    };
    let Ok((connection, _)) = x11rb::connect(None) else {
        return;
    };

    // Most X11 window managers reparent an application's client window into a
    // decorated frame. Walk to the highest ancestor below the root so the
    // complete decorated window, rather than only its client area, is centered.
    let mut frame = xid;
    loop {
        let Ok(cookie) = connection.query_tree(frame) else {
            return;
        };
        let Ok(tree) = cookie.reply() else {
            return;
        };
        if tree.parent == tree.root || tree.parent == 0 {
            break;
        }
        frame = tree.parent;
    }

    let Ok(cookie) = connection.get_geometry(frame) else {
        return;
    };
    let Ok(frame_geometry) = cookie.reply() else {
        return;
    };

    // GDK reports monitor geometry in application pixels; X11 positions use
    // device pixels, so account for the monitor's integer scale factor.
    let monitor_geometry = monitor.geometry();
    let scale = monitor.scale_factor().max(1);
    let monitor_x = monitor_geometry.x().saturating_mul(scale);
    let monitor_y = monitor_geometry.y().saturating_mul(scale);
    let monitor_width = monitor_geometry.width().saturating_mul(scale);
    let monitor_height = monitor_geometry.height().saturating_mul(scale);
    let frame_width = i32::from(frame_geometry.width)
        + i32::from(frame_geometry.border_width).saturating_mul(2);
    let frame_height = i32::from(frame_geometry.height)
        + i32::from(frame_geometry.border_width).saturating_mul(2);

    let x = monitor_x + (monitor_width - frame_width).max(0) / 2;
    let y = monitor_y + (monitor_height - frame_height).max(0) / 2;
    let values = ConfigureWindowAux::new().x(x).y(y);

    let Ok(cookie) = connection.configure_window(frame, &values) else {
        return;
    };
    let _ = cookie.check();
    let _ = connection.flush();
}

fn configure_font_rendering() {
    let Some(settings) = gtk::Settings::default() else {
        return;
    };

    // GTK4/GSK renders text with grayscale antialiasing rather than LCD
    // subpixel colour filtering. Force antialiasing on, keep only light outline
    // hinting, and leave glyph metrics unhinted so Pango can position glyphs at
    // fractional coordinates instead of snapping every advance to whole pixels.
    settings.set_gtk_xft_antialias(1);
    settings.set_gtk_xft_hinting(1);
    settings.set_gtk_xft_hintstyle(Some("hintslight"));
    settings.set_gtk_hint_font_metrics(false);
}

fn install_classic_theme() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(CLASSIC_CSS);
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_menu_bar(strings: Strings) -> (gtk::Box, MenuHandles) {
    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    bar.add_css_class("classic-menu-bar");

    let (edit_button, edit_label) = menu_button(menu_text(strings.edit_menu()));
    let (view_button, view_label) = menu_button(menu_text(strings.view_menu()));
    let (help_button, help_label) = menu_button(menu_text(strings.help_menu()));
    bar.append(&edit_button);
    bar.append(&view_button);
    bar.append(&help_button);

    let edit_popover = classic_menu_popover();
    let edit_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let undo = menu_item("");
    let redo = menu_item("");
    let copy = menu_item("");
    let paste = menu_item("");
    edit_box.append(&undo);
    edit_box.append(&redo);
    edit_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    edit_box.append(&copy);
    edit_box.append(&paste);
    edit_popover.set_child(Some(&edit_box));
    edit_button.set_popover(Some(&edit_popover));

    let view_popover = classic_menu_popover();
    let view_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let standard = menu_item("");
    let scientific = menu_item("");
    let graph = menu_item("");
    let history = menu_item("");
    let period = menu_item("");
    let comma = menu_item("");
    let english = menu_item("");
    let portuguese = menu_item("");
    let spanish = menu_item("");
    view_box.append(&standard);
    view_box.append(&scientific);
    view_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    view_box.append(&graph);
    view_box.append(&history);
    view_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    view_box.append(&period);
    view_box.append(&comma);
    view_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    view_box.append(&english);
    view_box.append(&portuguese);
    view_box.append(&spanish);
    view_popover.set_child(Some(&view_box));
    view_button.set_popover(Some(&view_popover));

    let help_popover = classic_menu_popover();
    let help_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let help_topics = menu_item("");
    let about = menu_item("");
    help_box.append(&help_topics);
    help_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    help_box.append(&about);
    help_popover.set_child(Some(&help_box));
    help_button.set_popover(Some(&help_popover));

    bind_menu_hover(&[
        edit_button.clone(),
        view_button.clone(),
        help_button.clone(),
    ]);

    (
        bar,
        MenuHandles {
            edit_button,
            edit_label,
            view_button,
            view_label,
            help_button,
            help_label,
            edit_popover,
            view_popover,
            help_popover,
            undo,
            redo,
            copy,
            paste,
            standard,
            scientific,
            graph,
            history,
            period,
            comma,
            english,
            portuguese,
            spanish,
            help_topics,
            about,
        },
    )
}

fn menu_button(text: String) -> (gtk::MenuButton, gtk::Label) {
    let button = gtk::MenuButton::builder()
        .direction(gtk::ArrowType::Down)
        .always_show_arrow(false)
        .build();
    let label = gtk::Label::new(Some(&text));
    button.set_child(Some(&label));
    button.add_css_class("classic-menu-button");
    (button, label)
}

fn classic_menu_popover() -> gtk::Popover {
    let popover = gtk::Popover::new();
    popover.add_css_class("classic-menu-popover");
    popover.set_has_arrow(false);
    popover.set_position(gtk::PositionType::Bottom);
    popover.set_halign(gtk::Align::Start);
    popover.set_offset(0, -1);
    popover
}

fn bind_menu_hover(buttons: &[gtk::MenuButton]) {
    let buttons = Rc::new(buttons.to_vec());
    for button in buttons.iter() {
        let target = button.clone();
        let peers = Rc::clone(&buttons);
        let motion = gtk::EventControllerMotion::new();
        motion.connect_enter(move |_, _, _| {
            if target.is_active() || !peers.iter().any(|button| button.is_active()) {
                return;
            }
            for button in peers.iter() {
                if button != &target {
                    button.set_active(false);
                }
            }
            target.set_active(true);
        });
        button.add_controller(motion);
    }
}

fn panel_separator() -> gtk::Separator {
    let separator = gtk::Separator::new(gtk::Orientation::Vertical);
    separator.add_css_class("panel-separator");
    separator.set_size_request(PANEL_SEPARATOR_W, -1);
    separator
}

fn menu_item(label: &str) -> gtk::Button {
    let button = gtk::Button::with_label(label);
    button.add_css_class("classic-menu-item");
    button.set_halign(gtk::Align::Fill);
    button
}

fn menu_text(text: &str) -> String {
    text.replace('&', "")
}

fn build_calculator_panels(
    calculator: &Calculator,
    separator: DecimalSeparator,
) -> CalculatorPanels {
    let standard = gtk::Fixed::new();
    standard.add_css_class("calc-surface");
    standard.set_size_request(STD_W, STD_H);

    let scientific = gtk::Fixed::new();
    scientific.add_css_class("calc-surface");
    scientific.set_size_request(SCI_W, SCI_H);
    scientific.set_visible(false);

    let standard_display = make_display(&standard, 10, 5, 245, &calculator.display);
    let scientific_display = make_display(&scientific, 244, SCI_DISPLAY_Y, 240, &calculator.display);
    let standard_memory = make_indicator(&standard, 10, 39, 38, 27);
    let scientific_parens = make_indicator(&scientific, 145, SCI_COMMAND_Y + 2, 35, 27);
    let scientific_memory = make_indicator(&scientific, 197, SCI_COMMAND_Y + 2, 35, 27);

    let separator_line = gtk::Separator::new(gtk::Orientation::Horizontal);
    separator_line.add_css_class("classic-separator");
    fixed_put(&scientific, &separator_line, 0, 16, 500, 1);

    make_group_box(&scientific, 13, SCI_SELECTOR_BOX_Y, 266, 34);
    make_group_box(&scientific, 286, SCI_SELECTOR_BOX_Y, 198, 34);
    make_group_box(&scientific, 13, SCI_COMMAND_BOX_Y, 127, 36);

    let base_radios = [
        make_radio(&scientific, 20, SCI_SELECTOR_Y, 58, "Hex", None),
        make_radio(&scientific, 84, SCI_SELECTOR_Y, 58, "Dec", None),
        make_radio(&scientific, 148, SCI_SELECTOR_Y, 58, "Oct", None),
        make_radio(&scientific, 212, SCI_SELECTOR_Y, 50, "Bin", None),
    ];
    for radio in base_radios.iter().skip(1) {
        radio.set_group(Some(&base_radios[0]));
    }

    let angle_radios = [
        make_radio(&scientific, 292, SCI_SELECTOR_Y, 60, "Deg", None),
        make_radio(&scientific, 359, SCI_SELECTOR_Y, 60, "Rad", None),
        make_radio(&scientific, 425, SCI_SELECTOR_Y, 52, "Grad", None),
    ];
    for radio in angle_radios.iter().skip(1) {
        radio.set_group(Some(&angle_radios[0]));
    }

    let inv = gtk::CheckButton::with_label("Inv");
    inv.add_css_class("classic-check");
    fixed_put(&scientific, &inv, 20, SCI_CHECK_Y, SCI_CHECK_W, SCI_CHECK_H);
    let hyp = gtk::CheckButton::with_label("Hyp");
    hyp.add_css_class("classic-check");
    fixed_put(&scientific, &hyp, 86, SCI_CHECK_Y, SCI_CHECK_W, SCI_CHECK_H);

    let mut action_buttons = Vec::new();
    let mut tooltip_targets = Vec::new();
    for def in standard_button_defs() {
        let button = make_calc_button(&standard, def, separator);
        tooltip_targets.push((button.clone().upcast(), action_help_key(def.action)));
        action_buttons.push((button, def.action));
    }
    for def in scientific_button_defs() {
        let button = make_calc_button(&scientific, def, separator);
        tooltip_targets.push((button.clone().upcast(), action_help_key(def.action)));
        action_buttons.push((button, def.action));
    }

    tooltip_targets.extend([
        (standard_display.clone().upcast(), "display"),
        (scientific_display.clone().upcast(), "display"),
        (standard_memory.clone().upcast(), "memory_indicator"),
        (scientific_memory.clone().upcast(), "memory_indicator"),
        (scientific_parens.clone().upcast(), "paren_indicator"),
        (inv.clone().upcast(), "inv"),
        (hyp.clone().upcast(), "hyp"),
    ]);

    tooltip_targets.extend(
        base_radios
            .iter()
            .zip(["hex", "dec", "oct", "bin"])
            .map(|(radio, key)| (radio.clone().upcast(), key)),
    );
    tooltip_targets.extend(
        angle_radios
            .iter()
            .zip(["deg", "rad", "grad"])
            .map(|(radio, key)| (radio.clone().upcast(), key)),
    );

    CalculatorPanels {
        standard,
        scientific,
        standard_display,
        scientific_display,
        standard_memory,
        scientific_memory,
        scientific_parens,
        inv,
        hyp,
        base_radios,
        angle_radios,
        action_buttons,
        tooltip_targets,
    }
}

fn fixed_put<W: IsA<gtk::Widget>>(
    panel: &gtk::Fixed,
    widget: &W,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    widget.set_size_request(dp(width), dp(height));
    panel.put(widget, f64::from(dp(x)), f64::from(dp(y)));
}

fn make_display(
    panel: &gtk::Fixed,
    x: i32,
    y: i32,
    width: i32,
    initial: &str,
) -> gtk::Label {
    let frame = gtk::Frame::new(None);
    frame.add_css_class("classic-field");
    let label = gtk::Label::new(Some(initial));
    label.add_css_class("classic-display");
    label.set_xalign(1.0);
    label.set_yalign(0.5);
    label.set_selectable(false);
    frame.set_child(Some(&label));
    fixed_put(panel, &frame, x, y, width, 24);
    label
}

fn make_indicator(
    panel: &gtk::Fixed,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> gtk::Label {
    let frame = gtk::Frame::new(None);
    frame.add_css_class("classic-indicator");
    let label = gtk::Label::new(None);
    label.add_css_class("classic-indicator-label");
    frame.set_child(Some(&label));
    fixed_put(panel, &frame, x, y, width, height);
    label
}

fn make_group_box(panel: &gtk::Fixed, x: i32, y: i32, width: i32, height: i32) {
    let frame = gtk::Frame::new(None);
    frame.add_css_class("classic-group");
    frame.set_can_target(false);
    fixed_put(panel, &frame, x, y, width, height);
}

fn make_radio(
    panel: &gtk::Fixed,
    x: i32,
    y: i32,
    width: i32,
    label: &str,
    group: Option<&gtk::CheckButton>,
) -> gtk::CheckButton {
    let radio = gtk::CheckButton::with_label(label);
    radio.add_css_class("classic-radio");
    if let Some(group) = group {
        radio.set_group(Some(group));
    }
    fixed_put(panel, &radio, x, y, width, 18);
    radio
}

fn make_calc_button(
    panel: &gtk::Fixed,
    def: ButtonDef,
    separator: DecimalSeparator,
) -> gtk::Button {
    let label = if matches!(def.action, Action::Dot) {
        separator.as_char().to_string()
    } else {
        def.label.to_string()
    };
    let button = gtk::Button::with_label(&label);
    button.add_css_class("classic-button");
    button.add_css_class(match def.tone {
        Tone::Red => "tone-red",
        Tone::Blue => "tone-blue",
        Tone::Navy => "tone-navy",
        Tone::Magenta => "tone-magenta",
        Tone::Maroon => "tone-maroon",
    });
    fixed_put(panel, &button, def.x, def.y, def.w, def.h);
    button
}

fn build_graph_panel(strings: Strings) -> GraphPanel {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, dp(6));
    panel.add_css_class("side-panel");
    panel.add_css_class("graph-panel");
    panel.set_size_request(GRAPH_W, STD_H);

    let function_label = gtk::Label::new(Some(strings.graph_function()));
    function_label.set_xalign(0.0);
    panel.append(&function_label);

    let input_row = gtk::Box::new(gtk::Orientation::Horizontal, dp(5));
    let expression = gtk::Entry::new();
    expression.add_css_class("graph-entry");
    expression.set_hexpand(true);
    let plot_button = gtk::Button::with_label(strings.graph_plot());
    plot_button.add_css_class("classic-button");
    plot_button.add_css_class("tone-navy");
    plot_button.set_size_request(dp(62), dp(24));
    input_row.append(&expression);
    input_row.append(&plot_button);
    panel.append(&input_row);

    let frame = gtk::Frame::new(None);
    frame.add_css_class("graph-frame");
    frame.set_hexpand(true);
    frame.set_vexpand(true);
    let canvas = gtk::DrawingArea::new();
    canvas.set_content_width(dp(300));
    canvas.set_content_height(dp(50));
    canvas.set_hexpand(true);
    canvas.set_vexpand(true);
    frame.set_child(Some(&canvas));
    panel.append(&frame);

    let roots = gtk::Label::new(Some(strings.graph_roots_not_plotted()));
    roots.add_css_class("graph-status");
    roots.set_xalign(0.0);
    roots.set_wrap(true);
    roots.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    roots.set_lines(3);
    panel.append(&roots);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, dp(6));
    actions.set_halign(gtk::Align::End);
    let reset_button = gtk::Button::with_label(strings.graph_reset_view());
    reset_button.add_css_class("classic-button");
    reset_button.set_sensitive(false);
    reset_button.set_size_request(dp(92), dp(24));
    let export_button = gtk::Button::with_label(strings.graph_export());
    export_button.add_css_class("classic-button");
    export_button.set_sensitive(false);
    export_button.set_size_request(dp(92), dp(24));
    actions.append(&reset_button);
    actions.append(&export_button);
    panel.append(&actions);

    GraphPanel {
        panel,
        function_label,
        expression,
        plot_button,
        canvas,
        roots,
        reset_button,
        export_button,
        model: Rc::new(RefCell::new(GraphModel::default())),
        drag_initial: Rc::new(RefCell::new(None)),
        pointer: Rc::new(Cell::new((0.5, 0.5))),
    }
}

fn build_history_panel(strings: Strings) -> HistoryPanel {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, dp(6));
    panel.add_css_class("side-panel");
    panel.add_css_class("history-panel");
    panel.set_size_request(HISTORY_W, STD_H);

    let title = gtk::Label::new(Some(strings.history_title()));
    title.add_css_class("panel-title");
    title.set_xalign(0.0);
    panel.append(&title);

    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .build();
    scroller.add_css_class("classic-scroll");
    let list = gtk::ListBox::new();
    list.add_css_class("classic-list");
    list.set_selection_mode(gtk::SelectionMode::Single);
    scroller.set_child(Some(&list));
    panel.append(&scroller);

    let clear_button = gtk::Button::with_label(strings.clear_history());
    clear_button.add_css_class("classic-button");
    clear_button.set_halign(gtk::Align::End);
    clear_button.set_size_request(dp(94), dp(24));
    panel.append(&clear_button);

    HistoryPanel {
        panel,
        title,
        list,
        clear_button,
    }
}

fn bind_action_buttons(ui: &Rc<Ui>) {
    for (button, action) in &ui.action_buttons {
        let ui = Rc::clone(ui);
        let action = *action;
        button.connect_clicked(move |_| perform(&ui, action));
    }
}

fn bind_selectors(ui: &Rc<Ui>) {
    for (index, base) in [Base::Hex, Base::Dec, Base::Oct, Base::Bin]
        .into_iter()
        .enumerate()
    {
        let radio = ui.base_radios[index].clone();
        let ui = Rc::clone(ui);
        radio.connect_toggled(move |radio| {
            if radio.is_active() && ui.calc.borrow().base != base {
                mutate_calculator(&ui, |calc| calc.set_base(base));
                refresh(&ui);
                replot_existing_graph(&ui);
            }
        });
    }

    for (index, angle) in [AngleMode::Degrees, AngleMode::Radians, AngleMode::Grads]
        .into_iter()
        .enumerate()
    {
        let radio = ui.angle_radios[index].clone();
        let ui = Rc::clone(ui);
        radio.connect_toggled(move |radio| {
            if radio.is_active() && ui.calc.borrow().angle != angle {
                mutate_calculator(&ui, |calc| calc.angle = angle);
                refresh(&ui);
                replot_existing_graph(&ui);
            }
        });
    }

    {
        let check = ui.inv.clone();
        let ui = Rc::clone(ui);
        check.connect_toggled(move |check| {
            let active = check.is_active();
            if ui.calc.borrow().inv != active {
                mutate_calculator(&ui, |calc| calc.inv = active);
                refresh(&ui);
            }
        });
    }
    {
        let check = ui.hyp.clone();
        let ui = Rc::clone(ui);
        check.connect_toggled(move |check| {
            let active = check.is_active();
            if ui.calc.borrow().hyp != active {
                mutate_calculator(&ui, |calc| calc.hyp = active);
                refresh(&ui);
            }
        });
    }
}

fn bind_menu(ui: &Rc<Ui>) {
    connect_menu_item(&ui.menus.undo, &ui.menus.edit_popover, {
        let ui = Rc::clone(ui);
        move || undo(&ui)
    });
    connect_menu_item(&ui.menus.redo, &ui.menus.edit_popover, {
        let ui = Rc::clone(ui);
        move || redo(&ui)
    });
    connect_menu_item(&ui.menus.copy, &ui.menus.edit_popover, {
        let ui = Rc::clone(ui);
        move || perform(&ui, Action::Copy)
    });
    connect_menu_item(&ui.menus.paste, &ui.menus.edit_popover, {
        let ui = Rc::clone(ui);
        move || perform(&ui, Action::Paste)
    });

    connect_menu_item(&ui.menus.standard, &ui.menus.view_popover, {
        let ui = Rc::clone(ui);
        move || set_mode(&ui, Mode::Standard)
    });
    connect_menu_item(&ui.menus.scientific, &ui.menus.view_popover, {
        let ui = Rc::clone(ui);
        move || set_mode(&ui, Mode::Scientific)
    });
    connect_menu_item(&ui.menus.graph, &ui.menus.view_popover, {
        let ui = Rc::clone(ui);
        move || {
            let visible = !ui.settings.borrow().graph_visible;
            set_graph_visible(&ui, visible);
        }
    });
    connect_menu_item(&ui.menus.history, &ui.menus.view_popover, {
        let ui = Rc::clone(ui);
        move || {
            let visible = !ui.settings.borrow().history_visible;
            set_history_visible(&ui, visible);
        }
    });
    connect_menu_item(&ui.menus.period, &ui.menus.view_popover, {
        let ui = Rc::clone(ui);
        move || set_decimal_separator(&ui, DecimalSeparator::Period)
    });
    connect_menu_item(&ui.menus.comma, &ui.menus.view_popover, {
        let ui = Rc::clone(ui);
        move || set_decimal_separator(&ui, DecimalSeparator::Comma)
    });
    connect_menu_item(&ui.menus.english, &ui.menus.view_popover, {
        let ui = Rc::clone(ui);
        move || set_language(&ui, Language::English)
    });
    connect_menu_item(&ui.menus.portuguese, &ui.menus.view_popover, {
        let ui = Rc::clone(ui);
        move || set_language(&ui, Language::Portuguese)
    });
    connect_menu_item(&ui.menus.spanish, &ui.menus.view_popover, {
        let ui = Rc::clone(ui);
        move || set_language(&ui, Language::Spanish)
    });

    connect_menu_item(&ui.menus.help_topics, &ui.menus.help_popover, {
        let ui = Rc::clone(ui);
        move || perform(&ui, Action::Help)
    });
    connect_menu_item(&ui.menus.about, &ui.menus.help_popover, {
        let ui = Rc::clone(ui);
        move || perform(&ui, Action::About)
    });
}

fn connect_menu_item<F: Fn() + 'static>(
    item: &gtk::Button,
    popover: &gtk::Popover,
    callback: F,
) {
    let popover = popover.clone();
    item.connect_clicked(move |_| {
        popover.popdown();
        callback();
    });
}

fn bind_keyboard<W: IsA<gtk::Widget>>(ui: &Rc<Ui>, widget: &W) {
    let controller = gtk::EventControllerKey::new();
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    let ui = Rc::clone(ui);
    controller.connect_key_pressed(move |_, key, _, state| {
        if graph_entry_has_focus(&ui) {
            return glib::Propagation::Proceed;
        }
        if handle_key(&ui, key, state) {
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    widget.add_controller(controller);
}

fn graph_entry_has_focus(ui: &Ui) -> bool {
    let Some(focus) = gtk::prelude::RootExt::focus(&ui.window) else {
        return false;
    };
    let entry: gtk::Widget = ui.graph_panel.expression.clone().upcast();
    focus == entry || focus.is_ancestor(&entry)
}


fn handle_key(ui: &Rc<Ui>, key: gdk::Key, state: gdk::ModifierType) -> bool {
    if state.contains(gdk::ModifierType::ALT_MASK) {
        return false;
    }
    let control = state.contains(gdk::ModifierType::CONTROL_MASK);
    let shift = state.contains(gdk::ModifierType::SHIFT_MASK);
    let ch = key.to_unicode();

    if control {
        return match ch.map(|value| value.to_ascii_lowercase()) {
            Some('z') => {
                undo(ui);
                true
            }
            Some('y') => {
                redo(ui);
                true
            }
            Some('c') => {
                perform(ui, Action::Copy);
                true
            }
            Some('v') => {
                perform(ui, Action::Paste);
                true
            }
            Some('l') => {
                perform(ui, Action::MemC);
                true
            }
            Some('r') => {
                perform(ui, Action::MemR);
                true
            }
            Some('m') => {
                perform(ui, Action::MemS);
                true
            }
            Some('p') => {
                perform(ui, Action::MemAdd);
                true
            }
            Some('s') if ui.calc.borrow().mode == Mode::Scientific => {
                perform(ui, Action::StatsOpen);
                true
            }
            Some('a') if ui.calc.borrow().mode == Mode::Scientific => {
                perform(ui, Action::StatsAvg);
                true
            }
            Some('t') if ui.calc.borrow().mode == Mode::Scientific => {
                perform(ui, Action::StatsSum);
                true
            }
            Some('d') if ui.calc.borrow().mode == Mode::Scientific => {
                perform(ui, Action::StatsDev);
                true
            }
            _ => false,
        };
    }

    if shift && key == gdk::Key::Insert {
        perform(ui, Action::Paste);
        return true;
    }

    let direct = match key {
        gdk::Key::BackSpace => Some(Action::Back),
        gdk::Key::Escape => Some(Action::C),
        gdk::Key::Delete | gdk::Key::KP_Delete => Some(Action::CE),
        gdk::Key::Return | gdk::Key::KP_Enter => Some(Action::Eq),
        gdk::Key::KP_Add => Some(Action::Bin(BinaryOp::Add)),
        gdk::Key::KP_Subtract => Some(Action::Bin(BinaryOp::Sub)),
        gdk::Key::KP_Multiply => Some(Action::Bin(BinaryOp::Mul)),
        gdk::Key::KP_Divide => Some(Action::Bin(BinaryOp::Div)),
        gdk::Key::KP_Decimal => Some(Action::Dot),
        gdk::Key::F1 => Some(Action::Help),
        _ => None,
    };
    if let Some(action) = direct {
        perform(ui, action);
        return true;
    }

    let Some(ch) = ch else {
        return false;
    };
    let mode = ui.calc.borrow().mode;
    match ch {
        '0'..='9' => perform(ui, Action::Digit(ch)),
        'a'..='f' | 'A'..='F' => perform(ui, Action::Digit(ch)),
        '.' | ',' => perform(ui, Action::Dot),
        '+' => perform(ui, Action::Bin(BinaryOp::Add)),
        '-' | '−' => perform(ui, Action::Bin(BinaryOp::Sub)),
        '*' => perform(ui, Action::KeyboardStar),
        '×' => perform(ui, Action::Bin(BinaryOp::Mul)),
        '/' | '÷' => perform(ui, Action::Bin(BinaryOp::Div)),
        '%' if mode == Mode::Scientific => perform(ui, Action::Bin(BinaryOp::Mod)),
        '%' => perform(ui, Action::Percent),
        '=' => perform(ui, Action::Eq),
        '(' if mode == Mode::Scientific => perform(ui, Action::Open),
        ')' if mode == Mode::Scientific => perform(ui, Action::Close),
        '&' if mode == Mode::Scientific => perform(ui, Action::Bin(BinaryOp::And)),
        '|' if mode == Mode::Scientific => perform(ui, Action::Bin(BinaryOp::Or)),
        '^' if mode == Mode::Scientific => perform(ui, Action::Bin(BinaryOp::Xor)),
        '<' if mode == Mode::Scientific => perform(ui, Action::Bin(BinaryOp::Lsh)),
        '~' if mode == Mode::Scientific => perform(ui, Action::Unary("not")),
        ';' if mode == Mode::Scientific => perform(ui, Action::Unary("int")),
        _ => match ch.to_ascii_lowercase() {
            'r' => perform(ui, Action::Unary("recip")),
            's' if mode == Mode::Scientific => perform(ui, Action::Unary("sin")),
            'o' if mode == Mode::Scientific => perform(ui, Action::Unary("cos")),
            't' if mode == Mode::Scientific => perform(ui, Action::Unary("tan")),
            'n' if mode == Mode::Scientific => perform(ui, Action::Unary("ln")),
            'l' if mode == Mode::Scientific => perform(ui, Action::Unary("log")),
            'm' if mode == Mode::Scientific => perform(ui, Action::Unary("dms")),
            'x' if mode == Mode::Scientific => perform(ui, Action::Unary("exp")),
            'y' if mode == Mode::Scientific => perform(ui, Action::Bin(BinaryOp::Pow)),
            'p' if mode == Mode::Scientific => perform(ui, Action::Pi),
            'i' if mode == Mode::Scientific => {
                let active = !ui.calc.borrow().inv;
                mutate_calculator(ui, |calc| calc.inv = active);
                refresh(ui);
            }
            'h' if mode == Mode::Scientific => {
                let active = !ui.calc.borrow().hyp;
                mutate_calculator(ui, |calc| calc.hyp = active);
                refresh(ui);
            }
            'v' if mode == Mode::Scientific => perform(ui, Action::ToggleFE),
            _ => return false,
        },
    }
    true
}

fn bind_graph(ui: &Rc<Ui>) {
    {
        let button = ui.graph_panel.plot_button.clone();
        let ui = Rc::clone(ui);
        button.connect_clicked(move |_| plot_graph(&ui));
    }
    {
        let entry = ui.graph_panel.expression.clone();
        let ui = Rc::clone(ui);
        entry.connect_activate(move |_| plot_graph(&ui));
    }
    {
        let button = ui.graph_panel.reset_button.clone();
        let ui = Rc::clone(ui);
        button.connect_clicked(move |_| {
            ui.graph_panel.model.borrow_mut().reset_view();
            refresh_graph_root_label(&ui);
            ui.graph_panel.canvas.queue_draw();
        });
    }
    {
        let button = ui.graph_panel.export_button.clone();
        let ui = Rc::clone(ui);
        button.connect_clicked(move |_| export_graph(&ui));
    }

    {
        let canvas = ui.graph_panel.canvas.clone();
        let model = Rc::clone(&ui.graph_panel.model);
        let ui = Rc::clone(ui);
        canvas.set_draw_func(move |_, context, width, height| {
            let Ok(backend) = CairoBackend::new(context, (width.max(1) as u32, height.max(1) as u32)) else {
                return;
            };
            let area = backend.into_drawing_area();
            let separator = ui.settings.borrow().decimal_separator.as_char();
            let _ = model.borrow().draw(&area, separator);
        });
    }

    {
        let pointer = Rc::clone(&ui.graph_panel.pointer);
        let motion = gtk::EventControllerMotion::new();
        motion.connect_motion(move |_, x, y| pointer.set((x, y)));
        ui.graph_panel.canvas.add_controller(motion);
    }

    {
        let drag_initial = Rc::clone(&ui.graph_panel.drag_initial);
        let model = Rc::clone(&ui.graph_panel.model);
        let canvas = ui.graph_panel.canvas.clone();
        let ui_for_update = Rc::clone(ui);
        let drag = gtk::GestureDrag::new();
        drag.set_button(1);
        drag.connect_drag_begin(move |_, _, _| {
            *drag_initial.borrow_mut() = Some(model.borrow().viewport());
        });
        let initial = Rc::clone(&ui.graph_panel.drag_initial);
        drag.connect_drag_update(move |_, dx, dy| {
            let Some(viewport) = *initial.borrow() else {
                return;
            };
            ui_for_update.graph_panel.model.borrow_mut().pan_from(
                viewport,
                dx.round() as i32,
                dy.round() as i32,
                canvas.width(),
                canvas.height(),
            );
            refresh_graph_root_label(&ui_for_update);
            canvas.queue_draw();
        });
        let initial = Rc::clone(&ui.graph_panel.drag_initial);
        drag.connect_drag_end(move |_, _, _| {
            *initial.borrow_mut() = None;
        });
        ui.graph_panel.canvas.add_controller(drag);
    }

    {
        let ui_for_scroll = Rc::clone(ui);
        let pointer = Rc::clone(&ui_for_scroll.graph_panel.pointer);
        let canvas = ui_for_scroll.graph_panel.canvas.clone();
        let scroll = gtk::EventControllerScroll::new(
            gtk::EventControllerScrollFlags::VERTICAL
                | gtk::EventControllerScrollFlags::DISCRETE,
        );
        scroll.connect_scroll(move |_, _, dy| {
            let (x, y) = pointer.get();
            let width = canvas.width().max(1) as f64;
            let height = canvas.height().max(1) as f64;
            let rotation = if dy < 0.0 { 120 } else { -120 };
            ui_for_scroll.graph_panel.model.borrow_mut().zoom(
                rotation,
                (x / width).clamp(0.0, 1.0),
                (y / height).clamp(0.0, 1.0),
            );
            refresh_graph_root_label(&ui_for_scroll);
            canvas.queue_draw();
            glib::Propagation::Stop
        });
        ui.graph_panel.canvas.add_controller(scroll);
    }
}

fn bind_context_help(ui: &Rc<Ui>) {
    let context_menu = gtk::Popover::new();
    context_menu.add_css_class("classic-context-menu");
    context_menu.set_has_arrow(false);
    context_menu.set_autohide(true);
    context_menu.set_position(gtk::PositionType::Bottom);
    context_menu.set_parent(&ui.root);

    let context_item = gtk::Button::new();
    context_item.add_css_class("classic-context-item");
    context_item.set_halign(gtk::Align::Fill);
    context_menu.set_child(Some(&context_item));

    let help_popup = gtk::Popover::new();
    help_popup.add_css_class("classic-help-popup");
    help_popup.set_has_arrow(false);
    help_popup.set_autohide(true);
    help_popup.set_position(gtk::PositionType::Bottom);
    help_popup.set_parent(&ui.root);

    let help_label = gtk::Label::new(None);
    help_label.add_css_class("classic-help-label");
    help_label.set_xalign(0.0);
    help_label.set_wrap(true);
    help_label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    help_label.set_max_width_chars(60);
    help_popup.set_child(Some(&help_label));

    let selected_key = Rc::new(Cell::new(None::<&'static str>));
    {
        let ui = Rc::clone(ui);
        let context_menu = context_menu.clone();
        let help_popup = help_popup.clone();
        let help_label = help_label.clone();
        let selected_key = Rc::clone(&selected_key);
        context_item.connect_clicked(move |_| {
            context_menu.popdown();
            let Some(key) = selected_key.get() else {
                return;
            };
            let language = ui.settings.borrow().language;
            let Some(body) = ui.tooltips.get(language, key) else {
                return;
            };
            help_label.set_label(body);
            let help_popup = help_popup.clone();
            glib::idle_add_local_once(move || {
                help_popup.popup();
            });
        });
    }

    let targets = ui
        .tooltip_targets
        .iter()
        .map(|(widget, key)| (widget.clone(), *key))
        .collect::<Vec<_>>();
    let click = gtk::GestureClick::new();
    click.set_button(3);
    click.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let ui = Rc::clone(ui);
        let root = ui.root.clone();
        let context_menu = context_menu.clone();
        let context_item = context_item.clone();
        let help_popup = help_popup.clone();
        let selected_key = Rc::clone(&selected_key);
        click.connect_pressed(move |gesture, _, x, y| {
            let flags = gtk::PickFlags::INSENSITIVE | gtk::PickFlags::NON_TARGETABLE;
            let Some(picked) = root.pick(x, y, flags) else {
                return;
            };
            let Some(key) = targets.iter().find_map(|(widget, key)| {
                (picked == widget.clone() || picked.is_ancestor(widget)).then_some(*key)
            }) else {
                return;
            };

            gesture.set_state(gtk::EventSequenceState::Claimed);
            selected_key.set(Some(key));
            context_item.set_label(strings_for(&ui).whats_this());
            let point = gdk::Rectangle::new(
                x.round() as i32,
                y.round() as i32,
                1,
                1,
            );
            context_menu.set_pointing_to(Some(&point));
            help_popup.set_pointing_to(Some(&point));
            help_popup.popdown();
            context_menu.popup();
        });
    }
    ui.root.add_controller(click);
}

fn bind_outside_entry_focus(ui: &Rc<Ui>) {
    // Remove focus from the Function entry after a completed primary click
    // elsewhere in the client area without claiming the pointer sequence.
    let click = gtk::GestureClick::new();
    click.set_button(1);
    click.set_exclusive(false);
    click.set_propagation_phase(gtk::PropagationPhase::Bubble);
    let content = ui.content.clone();
    let entry: gtk::Widget = ui.graph_panel.expression.clone().upcast();
    let window = ui.window.clone();
    click.connect_released(move |_, _, x, y| {
        let target = content.pick(x, y, gtk::PickFlags::DEFAULT);
        let inside_entry = target
            .as_ref()
            .is_some_and(|target| target == &entry || target.is_ancestor(&entry));
        if !inside_entry {
            gtk::prelude::GtkWindowExt::set_focus(&window, None::<&gtk::Widget>);
        }
    });
    ui.content.add_controller(click);
}

fn bind_history(ui: &Rc<Ui>) {
    {
        let button = ui.history_panel.clear_button.clone();
        let ui = Rc::clone(ui);
        button.connect_clicked(move |_| {
            ui.calculation_log.borrow_mut().clear();
            refresh_calculation_history(&ui);
        });
    }
    {
        let list = ui.history_panel.list.clone();
        let ui = Rc::clone(ui);
        list.connect_row_activated(move |_, row| {
            let index = row.index().max(0) as usize;
            let value = ui
                .calculation_log
                .borrow()
                .newest(index)
                .and_then(|entry| entry.value);
            if let Some(value) = value {
                mutate_calculator(&ui, |calc| calc.recall_history_value(value));
                refresh(&ui);
            }
        });
    }
}

fn strings_for(ui: &Ui) -> Strings {
    Strings::new(ui.settings.borrow().language)
}

fn perform(ui: &Rc<Ui>, action: Action) {
    match action {
        Action::Copy => {
            let clipboard = ui.window.clipboard();
            let strings = strings_for(ui);
            let raw = ui.calc.borrow().display.clone();
            clipboard.set_text(strings.runtime_message(&raw).unwrap_or(raw.as_str()));
            return;
        }
        Action::Paste => {
            let clipboard = ui.window.clipboard();
            let ui = Rc::clone(ui);
            clipboard.read_text_async(None::<&gio::Cancellable>, move |result| {
                if let Ok(Some(text)) = result {
                    let text = text.to_string();
                    let expression = text.split_whitespace().collect::<Vec<_>>().join(" ");
                    mutate_calculator(&ui, |calc| calc.paste_expression(&text));
                    if !expression.is_empty() {
                        append_calculation_history(&ui, expression);
                    }
                    refresh(&ui);
                }
            });
            return;
        }
        Action::About => {
            let strings = strings_for(ui);
            show_message(&ui.window, strings.about_title(), strings.about_body());
            return;
        }
        Action::Help => {
            let language = ui.settings.borrow().language;
            if let Err(error) = platform::launch_help(language) {
                let strings = strings_for(ui);
                let localized = strings.runtime_message(&error).unwrap_or(error.as_str());
                show_message(&ui.window, strings.help_title(), localized);
            }
            return;
        }
        Action::StatsOpen => {
            toggle_stats_box(ui);
            return;
        }
        _ => {}
    }

    let history_expression = {
        let calc = ui.calc.borrow();
        history_expression_before_action(&calc, action)
    };

    mutate_calculator(ui, |calc| match action {
        Action::Digit(ch) => calc.digit(ch),
        Action::Dot => calc.decimal_point(),
        Action::Back => calc.backspace(),
        Action::CE => calc.clear_entry(),
        Action::C => calc.clear_all(),
        Action::Sign => calc.sign(),
        Action::Eq => calc.equals(),
        Action::Percent => calc.percent(),
        Action::Bin(op) => calc.binary(op),
        Action::KeyboardStar => calc.keyboard_star(),
        Action::Unary(name) => calc.unary(name),
        Action::MemC => calc.memory_clear(),
        Action::MemR => calc.memory_recall(),
        Action::MemS => calc.memory_store(),
        Action::MemAdd => calc.memory_add(),
        Action::Pi => calc.set_value(std::f64::consts::PI),
        Action::Open => calc.open_paren(),
        Action::Close => calc.close_paren(),
        Action::StatsDat => calc.stat_dat(),
        Action::StatsAvg => calc.stat_avg(),
        Action::StatsSum => calc.stat_sum(),
        Action::StatsDev => calc.stat_stddev(),
        Action::ToggleFE => calc.toggle_fe(),
        Action::Copy | Action::Paste | Action::About | Action::Help | Action::StatsOpen => {}
    });

    if let Some(expression) = history_expression {
        append_calculation_history(ui, expression);
    }
    refresh(ui);
}


fn show_message(parent: &gtk::ApplicationWindow, title: &str, body: &str) {
    let dialog = gtk::AlertDialog::builder().build();
    dialog.set_message(title);
    dialog.set_detail(body);
    dialog.set_modal(true);
    dialog.show(Some(parent));
}

fn mutate_calculator<F>(ui: &Ui, mutation: F) -> bool
where
    F: FnOnce(&mut Calculator),
{
    let before = ui.calc.borrow().clone();
    mutation(&mut ui.calc.borrow_mut());
    let changed = ui.calc.borrow().ne(&before);
    if changed {
        ui.history.borrow_mut().record(before);
    }
    refresh_history_menu(ui);
    changed
}

fn undo(ui: &Rc<Ui>) {
    let current = ui.calc.borrow().clone();
    let previous = ui.history.borrow_mut().undo(current);
    if let Some(mut state) = previous {
        state.set_decimal_separator(ui.settings.borrow().decimal_separator.as_char());
        let mode = state.mode;
        *ui.calc.borrow_mut() = state;
        sync_surface(ui, mode);
        refresh(ui);
    }
    refresh_history_menu(ui);
}

fn redo(ui: &Rc<Ui>) {
    let current = ui.calc.borrow().clone();
    let next = ui.history.borrow_mut().redo(current);
    if let Some(mut state) = next {
        state.set_decimal_separator(ui.settings.borrow().decimal_separator.as_char());
        let mode = state.mode;
        *ui.calc.borrow_mut() = state;
        sync_surface(ui, mode);
        refresh(ui);
    }
    refresh_history_menu(ui);
}

fn refresh_history_menu(ui: &Ui) {
    let history = ui.history.borrow();
    ui.menus.undo.set_sensitive(history.can_undo());
    ui.menus.redo.set_sensitive(history.can_redo());
}

fn set_mode(ui: &Rc<Ui>, mode: Mode) {
    if ui.calc.borrow().mode != mode {
        mutate_calculator(ui, |calc| calc.set_mode(mode));
    }
    if ui.settings.borrow().mode != mode {
        ui.settings.borrow_mut().mode = mode;
        persist_settings(ui);
    }
    sync_surface(ui, mode);
    refresh(ui);
    apply_menu_state(ui);
}

fn set_graph_visible(ui: &Rc<Ui>, visible: bool) {
    if ui.settings.borrow().graph_visible == visible {
        return;
    }
    ui.settings.borrow_mut().graph_visible = visible;
    sync_surface(ui, ui.calc.borrow().mode);
    apply_menu_state(ui);
    persist_settings(ui);
    if visible {
        ui.graph_panel.canvas.queue_draw();
    }
}

fn set_history_visible(ui: &Rc<Ui>, visible: bool) {
    if ui.settings.borrow().history_visible == visible {
        return;
    }
    ui.settings.borrow_mut().history_visible = visible;
    sync_surface(ui, ui.calc.borrow().mode);
    apply_menu_state(ui);
    persist_settings(ui);
}

fn set_decimal_separator(ui: &Rc<Ui>, separator: DecimalSeparator) {
    if ui.settings.borrow().decimal_separator == separator {
        return;
    }
    ui.settings.borrow_mut().decimal_separator = separator;
    ui.calc
        .borrow_mut()
        .set_decimal_separator(separator.as_char());
    for (button, action) in &ui.action_buttons {
        if matches!(action, Action::Dot) {
            button.set_label(&separator.as_char().to_string());
        }
    }
    refresh_calculation_history(ui);
    refresh_graph_root_label(ui);
    ui.graph_panel.canvas.queue_draw();
    apply_menu_state(ui);
    persist_settings(ui);
    refresh(ui);
}

fn set_language(ui: &Rc<Ui>, language: Language) {
    if ui.settings.borrow().language == language {
        return;
    }
    ui.settings.borrow_mut().language = language;
    apply_language(ui);
    persist_settings(ui);
    refresh(ui);
}

fn persist_settings(ui: &Ui) {
    let strings = strings_for(ui);
    if let Err(error) = ui.settings.borrow_mut().save() {
        show_message(
            &ui.window,
            strings.calculator_title(),
            &format!("{}: {error}", strings.settings_error_prefix()),
        );
    }
}

fn sync_surface(ui: &Ui, mode: Mode) {
    let scientific = mode == Mode::Scientific;
    ui.standard_panel.set_visible(!scientific);
    ui.scientific_panel.set_visible(scientific);
    let (calc_width, height) = if scientific {
        (SCI_W, SCI_H)
    } else {
        (STD_W, STD_H)
    };
    ui.calculator_host.set_size_request(calc_width, height);
    ui.graph_panel.panel.set_size_request(GRAPH_W, height);
    ui.history_panel.panel.set_size_request(HISTORY_W, height);
    let settings = ui.settings.borrow();
    ui.graph_panel.panel.set_visible(settings.graph_visible);
    ui.graph_separator.set_visible(settings.graph_visible);
    ui.history_separator.set_visible(settings.history_visible);
    ui.history_panel.panel.set_visible(settings.history_visible);
    let width = calc_width
        + if settings.graph_visible {
            GRAPH_W + PANEL_SEPARATOR_W
        } else {
            0
        }
        + if settings.history_visible {
            HISTORY_W + PANEL_SEPARATOR_W
        } else {
            0
        };
    drop(settings);
    let window_height = height + dp(22);
    ui.content.set_size_request(width, height);
    ui.window.set_default_size(width, window_height);
    ui.window.set_resizable(false);
    ui.window.queue_resize();
}

fn apply_language(ui: &Ui) {
    let strings = strings_for(ui);
    ui.window.set_title(Some(strings.calculator_title()));
    ui.menus.edit_label.set_label(&menu_text(strings.edit_menu()));
    ui.menus.view_label.set_label(&menu_text(strings.view_menu()));
    ui.menus.help_label.set_label(&menu_text(strings.help_menu()));

    ui.menus.undo.set_label(&menu_text(strings.undo()));
    ui.menus.redo.set_label(&menu_text(strings.redo()));
    ui.menus.copy.set_label(&menu_text(strings.copy()));
    ui.menus.paste.set_label(&menu_text(strings.paste()));
    ui.menus.help_topics.set_label(&menu_text(strings.help_topics()));
    ui.menus.about.set_label(&menu_text(strings.about_opencalc()));

    ui.history_panel.title.set_label(strings.history_title());
    ui.history_panel
        .clear_button
        .set_label(strings.clear_history());
    ui.graph_panel
        .function_label
        .set_label(strings.graph_function());
    ui.graph_panel.plot_button.set_label(strings.graph_plot());
    ui.graph_panel
        .reset_button
        .set_label(strings.graph_reset_view());
    ui.graph_panel
        .export_button
        .set_label(strings.graph_export());

    if let Some(stats) = ui.stats_box.borrow().as_ref() {
        stats.window.set_title(Some(strings.statistics_box_title()));
    }

    apply_menu_state(ui);
    refresh_graph_root_label(ui);
    refresh_calculation_history(ui);
}

fn apply_menu_state(ui: &Ui) {
    let strings = strings_for(ui);
    let calc = ui.calc.borrow();
    let settings = ui.settings.borrow();
    ui.menus.standard.set_label(&radio_menu_label(
        calc.mode == Mode::Standard,
        strings.standard(),
    ));
    ui.menus.scientific.set_label(&radio_menu_label(
        calc.mode == Mode::Scientific,
        strings.scientific(),
    ));
    ui.menus.graph.set_label(&check_menu_label(
        settings.graph_visible,
        strings.graph(),
    ));
    ui.menus.history.set_label(&check_menu_label(
        settings.history_visible,
        strings.history(),
    ));
    ui.menus.period.set_label(&radio_menu_label(
        settings.decimal_separator == DecimalSeparator::Period,
        strings.period_separator(),
    ));
    ui.menus.comma.set_label(&radio_menu_label(
        settings.decimal_separator == DecimalSeparator::Comma,
        strings.comma_separator(),
    ));
    ui.menus.english.set_label(&radio_menu_label(
        settings.language == Language::English,
        Language::English.autonym(),
    ));
    ui.menus.portuguese.set_label(&radio_menu_label(
        settings.language == Language::Portuguese,
        Language::Portuguese.autonym(),
    ));
    ui.menus.spanish.set_label(&radio_menu_label(
        settings.language == Language::Spanish,
        Language::Spanish.autonym(),
    ));
}

fn radio_menu_label(active: bool, text: &str) -> String {
    format!("{} {}", if active { "●" } else { " " }, menu_text(text))
}

fn check_menu_label(active: bool, text: &str) -> String {
    format!("{} {}", if active { "✓" } else { " " }, menu_text(text))
}

fn refresh(ui: &Ui) {
    let calc = ui.calc.borrow();
    let strings = strings_for(ui);
    let display = strings
        .runtime_message(&calc.display)
        .unwrap_or(calc.display.as_str());
    ui.standard_display.set_label(display);
    ui.scientific_display.set_label(display);
    let memory = if calc.memory_set { "M" } else { "" };
    ui.standard_memory.set_label(memory);
    ui.scientific_memory.set_label(memory);
    let parens = if calc.paren_depth() == 0 {
        String::new()
    } else {
        format!("(={}", calc.paren_depth())
    };
    ui.scientific_parens.set_label(&parens);

    ui.inv.set_active(calc.inv);
    ui.hyp.set_active(calc.hyp);
    let base_index = match calc.base {
        Base::Hex => 0,
        Base::Dec => 1,
        Base::Oct => 2,
        Base::Bin => 3,
    };
    for (index, radio) in ui.base_radios.iter().enumerate() {
        radio.set_active(index == base_index);
    }
    let angle_index = match calc.angle {
        AngleMode::Degrees => 0,
        AngleMode::Radians => 1,
        AngleMode::Grads => 2,
    };
    for (index, radio) in ui.angle_radios.iter().enumerate() {
        radio.set_active(index == angle_index);
    }
    drop(calc);
    refresh_stats(ui);
    refresh_history_menu(ui);
    apply_menu_state(ui);
}

fn clean_history_number(calc: &Calculator, text: &str) -> String {
    let mut value = text.trim().to_string();
    if calc.error.is_none() && calc.base == Base::Dec {
        let separator = calc.decimal_separator();
        if value.ends_with(separator) {
            value.pop();
        }
    }
    if value.is_empty() {
        "0".to_string()
    } else {
        value
    }
}

fn binary_history_symbol(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "mod",
        BinaryOp::Pow => "^",
        BinaryOp::Root => "root",
        BinaryOp::And => "and",
        BinaryOp::Or => "or",
        BinaryOp::Xor => "xor",
        BinaryOp::Lsh => "lsh",
    }
}

fn unary_history_expression(calc: &Calculator, name: &'static str) -> Option<String> {
    if calc.error.is_some() {
        return None;
    }
    let operand = clean_history_number(calc, &calc.display);
    let actual = match (name, calc.inv, calc.hyp) {
        ("sin", true, true) => "asinh",
        ("cos", true, true) => "acosh",
        ("tan", true, true) => "atanh",
        ("sin", true, false) => "asin",
        ("cos", true, false) => "acos",
        ("tan", true, false) => "atan",
        ("sin", false, true) => "sinh",
        ("cos", false, true) => "cosh",
        ("tan", false, true) => "tanh",
        ("ln", true, _) => "exp",
        ("log", true, _) => "pow10",
        ("dms", true, _) => "dms_inv",
        _ => name,
    };
    Some(match actual {
        "factorial" => format!("{operand}!"),
        "square" => format!("sqr({operand})"),
        "cube" => format!("cube({operand})"),
        "recip" => format!("1/({operand})"),
        "pow10" => format!("10^({operand})"),
        "dms_inv" => format!("dms^-1({operand})"),
        "int" => format!("Int({operand})"),
        "not" => format!("Not({operand})"),
        other => format!("{other}({operand})"),
    })
}

fn history_expression_before_action(calc: &Calculator, action: Action) -> Option<String> {
    match action {
        Action::Eq => match calc.mode {
            Mode::Standard => calc.pending_standard_history_parts().map(|(op, lhs, rhs)| {
                format!(
                    "{} {} {}",
                    clean_history_number(calc, &lhs),
                    binary_history_symbol(op),
                    clean_history_number(calc, &rhs)
                )
            }),
            Mode::Scientific => calc.pending_scientific_history_expression().map(|expression| {
                if calc.decimal_separator() == ',' {
                    expression.replace('.', ",")
                } else {
                    expression
                }
            }),
        },
        Action::Bin(_) | Action::KeyboardStar
            if calc.mode == Mode::Standard && calc.is_entering_value() =>
        {
            calc.pending_standard_history_parts().map(|(op, lhs, rhs)| {
                format!(
                    "{} {} {}",
                    clean_history_number(calc, &lhs),
                    binary_history_symbol(op),
                    clean_history_number(calc, &rhs)
                )
            })
        }
        Action::Unary(name) => unary_history_expression(calc, name),
        Action::Percent if calc.error.is_none() => {
            Some(format!("{}%", clean_history_number(calc, &calc.display)))
        }
        Action::StatsAvg => Some(format!("Ave(n={})", calc.stats.len())),
        Action::StatsSum => Some(format!("Sum(n={})", calc.stats.len())),
        Action::StatsDev => Some(format!("s(n={})", calc.stats.len())),
        _ => None,
    }
}

fn append_calculation_history(ui: &Ui, expression: String) {
    let (result, value, separator) = {
        let calc = ui.calc.borrow();
        (
            clean_history_number(&calc, &calc.display),
            calc.value().ok(),
            calc.decimal_separator(),
        )
    };
    ui.calculation_log
        .borrow_mut()
        .push_localized(expression, result, value, separator);
    refresh_calculation_history(ui);
}

fn refresh_calculation_history(ui: &Ui) {
    while let Some(child) = ui.history_panel.list.first_child() {
        ui.history_panel.list.remove(&child);
    }

    let strings = strings_for(ui);
    let separator = ui.settings.borrow().decimal_separator.as_char();
    for entry in ui.calculation_log.borrow().newest_first() {
        let expression = entry.localized_expression(separator);
        let result = strings
            .runtime_message(&entry.result)
            .map(str::to_owned)
            .unwrap_or_else(|| entry.localized_result(separator));
        let label = gtk::Label::new(Some(&format!("{expression} =\n{result}")));
        label.set_xalign(0.0);
        label.set_wrap(true);
        label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        let row = gtk::ListBoxRow::new();
        row.set_child(Some(&label));
        row.set_activatable(entry.value.is_some());
        ui.history_panel.list.append(&row);
    }
}

fn plot_graph(ui: &Ui) {
    let expression = ui.graph_panel.expression.text().to_string();
    let context = ui.calc.borrow().eval_context();
    let succeeded = ui
        .graph_panel
        .model
        .borrow_mut()
        .plot(&expression, context)
        .is_ok();
    ui.graph_panel.reset_button.set_sensitive(succeeded);
    ui.graph_panel.export_button.set_sensitive(succeeded);
    refresh_graph_root_label(ui);
    ui.graph_panel.canvas.queue_draw();
}

fn replot_existing_graph(ui: &Ui) {
    if !ui.graph_panel.model.borrow().has_plot() {
        return;
    }
    plot_graph(ui);
}

fn graph_root_label(ui: &Ui) -> String {
    let strings = strings_for(ui);
    let separator = ui.settings.borrow().decimal_separator.as_char();
    let model = ui.graph_panel.model.borrow();
    if let Some(error) = model.error() {
        return format!("{}: {error}", strings.graph_plot_error());
    }
    match model.root_result() {
        RootResult::NotPlotted => strings.graph_roots_not_plotted().to_string(),
        RootResult::Roots(roots) => format!(
            "{}{}",
            strings.graph_roots_visible(),
            format_root_values(roots, separator)
        ),
        RootResult::None => strings.graph_no_roots().to_string(),
        RootResult::Infinite => strings.graph_infinite_roots().to_string(),
        RootResult::Unreliable => strings.graph_roots_unreliable().to_string(),
    }
}

fn refresh_graph_root_label(ui: &Ui) {
    ui.graph_panel.roots.set_label(&graph_root_label(ui));
}

fn export_graph(ui: &Rc<Ui>) {
    if !ui.graph_panel.model.borrow().has_plot() {
        refresh_graph_root_label(ui);
        return;
    }

    let strings = strings_for(ui);
    let title = strings.graph_export_title().to_string();
    let accept_label = strings.save().to_string();
    let filter_name = strings.graph_export_filter().to_string();
    let ui = Rc::clone(ui);

    glib::MainContext::default().spawn_local(async move {
        let Some(identifier) = WindowIdentifier::from_native(&ui.window).await else {
            let strings = strings_for(&ui);
            show_graph_export_error(&ui, strings.graph_export_window_identifier_error());
            return;
        };

        let filter = PortalFileFilter::new(filter_name.as_str())
            .glob("*.png")
            .glob("*.jpg")
            .glob("*.jpeg")
            .glob("*.svg");

        let request = match SelectedFiles::save_file()
            .identifier(identifier)
            .title(title.as_str())
            .accept_label(accept_label.as_str())
            .current_name("graph.png")
            .modal(true)
            .filter(filter)
            .send()
            .await
        {
            Ok(request) => request,
            Err(error) if portal_request_was_cancelled(&error) => return,
            Err(error) => {
                show_graph_export_error(&ui, error);
                return;
            }
        };

        let selected = match request.response() {
            Ok(selected) => selected,
            Err(error) if portal_request_was_cancelled(&error) => return,
            Err(error) => {
                show_graph_export_error(&ui, error);
                return;
            }
        };

        let Some(uri) = selected.uris().first() else {
            let strings = strings_for(&ui);
            show_graph_export_error(&ui, strings.graph_export_no_destination());
            return;
        };
        let uri = uri.to_string();
        let file = gio::File::for_uri(&uri);
        let Some(path) = file.path() else {
            let strings = strings_for(&ui);
            show_graph_export_error(&ui, strings.graph_export_local_file_required());
            return;
        };

        let (path, format) = graph_export_format(&path);
        let separator = ui.settings.borrow().decimal_separator.as_char();
        let root_summary = graph_root_label(&ui);
        if let Err(error) = ui.graph_panel.model.borrow().export(
            &path,
            format,
            (
                ui.graph_panel.canvas.width().max(1) as u32,
                ui.graph_panel.canvas.height().max(1) as u32,
            ),
            separator,
            &root_summary,
        ) {
            show_graph_export_error(&ui, error);
        }
    });
}

fn portal_request_was_cancelled(error: &PortalError) -> bool {
    matches!(
        error,
        PortalError::Response(PortalResponseError::Cancelled)
            | PortalError::Portal(PortalServiceError::Cancelled(_))
    )
}

fn show_graph_export_error(ui: &Ui, detail: impl std::fmt::Display) {
    let strings = strings_for(ui);
    show_message(
        &ui.window,
        strings.calculator_title(),
        &format!("{}: {detail}", strings.graph_export_error()),
    );
}

fn graph_export_format(path: &Path) -> (PathBuf, ExportFormat) {
    let mut path = path.to_path_buf();
    let format = match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("jpg") | Some("jpeg") => ExportFormat::Jpeg,
        Some("svg") => ExportFormat::Svg,
        Some("png") => ExportFormat::Png,
        _ => {
            path.set_extension("png");
            ExportFormat::Png
        }
    };
    (path, format)
}

fn toggle_stats_box(ui: &Rc<Ui>) {
    let existing = ui.stats_box.borrow_mut().take();
    if let Some(stats) = existing {
        stats.window.close();
        ui.window.present();
        return;
    }

    let stats = build_stats_box(ui);
    stats.window.present();
    *ui.stats_box.borrow_mut() = Some(stats);
    refresh_stats(ui);
}

fn build_stats_box(ui: &Rc<Ui>) -> StatsBox {
    let strings = strings_for(ui);
    let window = gtk::Window::builder()
        .transient_for(&ui.window)
        .title(strings.statistics_box_title())
        .resizable(false)
        .default_width(dp(240))
        .default_height(dp(165))
        .build();
    window.add_css_class("opencalc-window");

    let title_bar = gtk::HeaderBar::new();
    title_bar.add_css_class("opencalc-titlebar");
    title_bar.set_show_title_buttons(true);
    title_bar.set_decoration_layout(Some("icon:minimize,close"));
    window.set_titlebar(Some(&title_bar));

    window.set_destroy_with_parent(true);
    if let Some(application) = ui.window.application() {
        window.set_application(Some(&application));
    }

    let root = gtk::Box::new(gtk::Orientation::Vertical, dp(6));
    root.add_css_class("side-panel");
    window.set_child(Some(&root));

    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .build();
    let list = gtk::ListBox::new();
    list.add_css_class("classic-list");
    list.set_selection_mode(gtk::SelectionMode::Single);
    scroller.set_child(Some(&list));
    root.append(&scroller);

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, dp(5));
    let ret = stats_button("RET");
    let load = stats_button("LOAD");
    let clear_datum = stats_button("CD");
    let clear_all = stats_button("CAD");
    buttons.append(&ret);
    buttons.append(&load);
    buttons.append(&clear_datum);
    buttons.append(&clear_all);
    root.append(&buttons);

    let count_row = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    count_row.set_halign(gtk::Align::Center);
    count_row.append(&gtk::Label::new(Some("n=")));
    let count = gtk::Label::new(Some("0"));
    count_row.append(&count);
    root.append(&count_row);

    bind_keyboard(ui, &window);

    {
        let main = ui.window.clone();
        ret.connect_clicked(move |_| main.present());
    }
    {
        let ui = Rc::clone(ui);
        load.connect_clicked(move |_| stats_load(&ui));
    }
    {
        let ui = Rc::clone(ui);
        clear_datum.connect_clicked(move |_| stats_clear_datum(&ui));
    }
    {
        let ui = Rc::clone(ui);
        clear_all.connect_clicked(move |_| stats_clear_all(&ui));
    }
    {
        let ui = Rc::clone(ui);
        window.connect_close_request(move |_| {
            *ui.stats_box.borrow_mut() = None;
            glib::Propagation::Proceed
        });
    }

    StatsBox {
        window,
        list,
        count,
    }
}

fn stats_button(label: &str) -> gtk::Button {
    let button = gtk::Button::with_label(label);
    button.add_css_class("classic-button");
    button.set_hexpand(true);
    button.set_size_request(-1, dp(26));
    button
}

fn refresh_stats(ui: &Ui) {
    let Ok(stats_guard) = ui.stats_box.try_borrow() else {
        return;
    };
    let Some(stats) = stats_guard.as_ref() else {
        return;
    };
    while let Some(child) = stats.list.first_child() {
        stats.list.remove(&child);
    }
    let calc = ui.calc.borrow();
    for value in &calc.stats {
        let row = gtk::ListBoxRow::new();
        let label = gtk::Label::new(Some(&calc.format_decimal_value(*value)));
        label.set_xalign(0.0);
        row.set_child(Some(&label));
        stats.list.append(&row);
    }
    stats.count.set_label(&calc.stats.len().to_string());
}

fn stats_selection(ui: &Ui) -> Option<usize> {
    ui.stats_box
        .borrow()
        .as_ref()?
        .list
        .selected_row()
        .map(|row| row.index().max(0) as usize)
}

fn stats_load(ui: &Rc<Ui>) {
    let Some(index) = stats_selection(ui) else {
        return;
    };
    let value = ui.calc.borrow().stats.get(index).copied();
    if let Some(value) = value {
        mutate_calculator(ui, |calc| calc.set_value(value));
        refresh(ui);
    }
}

fn stats_clear_datum(ui: &Rc<Ui>) {
    let Some(index) = stats_selection(ui) else {
        return;
    };
    if index < ui.calc.borrow().stats.len() {
        mutate_calculator(ui, |calc| {
            calc.stats.remove(index);
        });
        refresh_stats(ui);
    }
}

fn stats_clear_all(ui: &Rc<Ui>) {
    mutate_calculator(ui, |calc| calc.stats.clear());
    refresh_stats(ui);
}

fn action_help_key(action: Action) -> &'static str {
    match action {
        Action::Digit(ch) if ch.is_ascii_alphabetic() => "hex_digit",
        Action::Digit(_) => "digit",
        Action::Dot => "decimal_point",
        Action::Back => "back",
        Action::CE => "ce",
        Action::C => "clear",
        Action::Sign => "sign",
        Action::Eq => "equals",
        Action::Percent => "percent",
        Action::Bin(BinaryOp::Add) => "add",
        Action::Bin(BinaryOp::Sub) => "subtract",
        Action::Bin(BinaryOp::Mul) | Action::KeyboardStar => "multiply",
        Action::Bin(BinaryOp::Div) => "divide",
        Action::Bin(BinaryOp::Mod) => "mod",
        Action::Bin(BinaryOp::Pow) | Action::Bin(BinaryOp::Root) => "pow",
        Action::Bin(BinaryOp::And) => "and",
        Action::Bin(BinaryOp::Or) => "or",
        Action::Bin(BinaryOp::Xor) => "xor",
        Action::Bin(BinaryOp::Lsh) => "lsh",
        Action::Unary("sqrt") => "sqrt",
        Action::Unary("recip") => "reciprocal",
        Action::Unary("dms") => "dms",
        Action::Unary("sin") => "sin",
        Action::Unary("cos") => "cos",
        Action::Unary("tan") => "tan",
        Action::Unary("exp") => "exp",
        Action::Unary("cube") => "cube",
        Action::Unary("square") => "square",
        Action::Unary("ln") => "ln",
        Action::Unary("log") => "log",
        Action::Unary("factorial") => "factorial",
        Action::Unary("not") => "not",
        Action::Unary("int") => "int",
        Action::Unary(_) => "display",
        Action::MemC => "mc",
        Action::MemR => "mr",
        Action::MemS => "ms",
        Action::MemAdd => "mplus",
        Action::Pi => "pi",
        Action::Open => "open_paren",
        Action::Close => "close_paren",
        Action::StatsOpen => "sta",
        Action::StatsDat => "dat",
        Action::StatsAvg => "ave",
        Action::StatsSum => "sum",
        Action::StatsDev => "stddev",
        Action::ToggleFE => "fe",
        Action::Copy | Action::Paste | Action::About | Action::Help => "display",
    }
}

fn standard_button_defs() -> Vec<ButtonDef> {
    let mut defs = vec![
        def(64, 39, 60, 27, "Back", Action::Back, Tone::Maroon),
        def(129, 39, 60, 27, "CE", Action::CE, Tone::Maroon),
        def(195, 39, 60, 27, "C", Action::C, Tone::Maroon),
    ];
    let rows = [('7', '8', '9'), ('4', '5', '6'), ('1', '2', '3')];
    let mem = [
        ("MC", Action::MemC),
        ("MR", Action::MemR),
        ("MS", Action::MemS),
        ("M+", Action::MemAdd),
    ];
    let ops = [
        ("/", Action::Bin(BinaryOp::Div)),
        ("*", Action::Bin(BinaryOp::Mul)),
        ("-", Action::Bin(BinaryOp::Sub)),
        ("+", Action::Bin(BinaryOp::Add)),
    ];
    let funcs = [
        ("sqrt", Action::Unary("sqrt")),
        ("%", Action::Percent),
        ("1/x", Action::Unary("recip")),
        ("=", Action::Eq),
    ];
    for row in 0..4usize {
        let y = 71 + row as i32 * 32;
        defs.push(def(10, y, 38, 27, mem[row].0, mem[row].1, Tone::Red));
        if row < 3 {
            let (a, b, c) = rows[row];
            defs.push(def(64, y, 38, 27, digit_label(a), Action::Digit(a), Tone::Blue));
            defs.push(def(107, y, 38, 27, digit_label(b), Action::Digit(b), Tone::Blue));
            defs.push(def(150, y, 38, 27, digit_label(c), Action::Digit(c), Tone::Blue));
        } else {
            defs.push(def(64, y, 38, 27, "0", Action::Digit('0'), Tone::Blue));
            defs.push(def(107, y, 38, 27, "+/-", Action::Sign, Tone::Blue));
            defs.push(def(150, y, 38, 27, ".", Action::Dot, Tone::Blue));
        }
        defs.push(def(193, y, 29, 27, ops[row].0, ops[row].1, Tone::Red));
        defs.push(def(227, y, 28, 27, funcs[row].0, funcs[row].1, Tone::Red));
    }
    defs
}

fn scientific_button_defs() -> Vec<ButtonDef> {
    let mut defs = vec![
        def(330, SCI_COMMAND_Y, 48, 27, "Back", Action::Back, Tone::Maroon),
        def(383, SCI_COMMAND_Y, 48, 27, "CE", Action::CE, Tone::Maroon),
        def(436, SCI_COMMAND_Y, 48, 27, "C", Action::C, Tone::Maroon),
    ];
    let rows: [[(&str, Action, Tone); 11]; 5] = [
        [
            ("Sta", Action::StatsOpen, Tone::Navy),
            ("F-E", Action::ToggleFE, Tone::Magenta),
            ("(", Action::Open, Tone::Magenta),
            (")", Action::Close, Tone::Magenta),
            ("MC", Action::MemC, Tone::Red),
            ("7", Action::Digit('7'), Tone::Blue),
            ("8", Action::Digit('8'), Tone::Blue),
            ("9", Action::Digit('9'), Tone::Blue),
            ("/", Action::Bin(BinaryOp::Div), Tone::Red),
            ("Mod", Action::Bin(BinaryOp::Mod), Tone::Red),
            ("And", Action::Bin(BinaryOp::And), Tone::Red),
        ],
        [
            ("Ave", Action::StatsAvg, Tone::Navy),
            ("dms", Action::Unary("dms"), Tone::Magenta),
            ("Exp", Action::Unary("exp"), Tone::Magenta),
            ("ln", Action::Unary("ln"), Tone::Magenta),
            ("MR", Action::MemR, Tone::Red),
            ("4", Action::Digit('4'), Tone::Blue),
            ("5", Action::Digit('5'), Tone::Blue),
            ("6", Action::Digit('6'), Tone::Blue),
            ("*", Action::Bin(BinaryOp::Mul), Tone::Red),
            ("Or", Action::Bin(BinaryOp::Or), Tone::Red),
            ("Xor", Action::Bin(BinaryOp::Xor), Tone::Red),
        ],
        [
            ("Sum", Action::StatsSum, Tone::Navy),
            ("sin", Action::Unary("sin"), Tone::Magenta),
            ("x^y", Action::Bin(BinaryOp::Pow), Tone::Magenta),
            ("log", Action::Unary("log"), Tone::Magenta),
            ("MS", Action::MemS, Tone::Red),
            ("1", Action::Digit('1'), Tone::Blue),
            ("2", Action::Digit('2'), Tone::Blue),
            ("3", Action::Digit('3'), Tone::Blue),
            ("-", Action::Bin(BinaryOp::Sub), Tone::Red),
            ("Lsh", Action::Bin(BinaryOp::Lsh), Tone::Red),
            ("Not", Action::Unary("not"), Tone::Red),
        ],
        [
            ("s", Action::StatsDev, Tone::Navy),
            ("cos", Action::Unary("cos"), Tone::Magenta),
            ("x^3", Action::Unary("cube"), Tone::Magenta),
            ("n!", Action::Unary("factorial"), Tone::Magenta),
            ("M+", Action::MemAdd, Tone::Red),
            ("0", Action::Digit('0'), Tone::Blue),
            ("+/-", Action::Sign, Tone::Blue),
            (".", Action::Dot, Tone::Blue),
            ("+", Action::Bin(BinaryOp::Add), Tone::Red),
            ("=", Action::Eq, Tone::Red),
            ("Int", Action::Unary("int"), Tone::Red),
        ],
        [
            ("Dat", Action::StatsDat, Tone::Navy),
            ("tan", Action::Unary("tan"), Tone::Magenta),
            ("x^2", Action::Unary("square"), Tone::Magenta),
            ("1/x", Action::Unary("recip"), Tone::Magenta),
            ("PI", Action::Pi, Tone::Navy),
            ("A", Action::Digit('A'), Tone::Navy),
            ("B", Action::Digit('B'), Tone::Navy),
            ("C", Action::Digit('C'), Tone::Navy),
            ("D", Action::Digit('D'), Tone::Navy),
            ("E", Action::Digit('E'), Tone::Navy),
            ("F", Action::Digit('F'), Tone::Navy),
        ],
    ];
    let xs = [13, 65, 105, 145, 197, 249, 289, 330, 370, 409, 449];
    let ys = [
        SCI_KEYPAD_Y,
        SCI_KEYPAD_Y + SCI_KEYPAD_STEP,
        SCI_KEYPAD_Y + SCI_KEYPAD_STEP * 2,
        SCI_KEYPAD_Y + SCI_KEYPAD_STEP * 3,
        SCI_KEYPAD_Y + SCI_KEYPAD_STEP * 4,
    ];
    for (row_index, row) in rows.iter().enumerate() {
        for (column, (label, action, tone)) in row.iter().copied().enumerate() {
            defs.push(def(xs[column], ys[row_index], 35, 27, label, action, tone));
        }
    }
    defs
}

const fn def(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    label: &'static str,
    action: Action,
    tone: Tone,
) -> ButtonDef {
    ButtonDef {
        x,
        y,
        w,
        h,
        label,
        action,
        tone,
    }
}

fn digit_label(ch: char) -> &'static str {
    match ch {
        '0' => "0",
        '1' => "1",
        '2' => "2",
        '3' => "3",
        '4' => "4",
        '5' => "5",
        '6' => "6",
        '7' => "7",
        '8' => "8",
        '9' => "9",
        'A' => "A",
        'B' => "B",
        'C' => "C",
        'D' => "D",
        'E' => "E",
        'F' => "F",
        _ => "",
    }
}
