//! Function plotting, viewport navigation, numerical roots, and image export.
//!
//! The graph feature deliberately reuses the calculator expression language.
//! `CompiledExpression` keeps the lexed token stream while this module samples
//! it at different x values, splits discontinuities, and locates real roots in
//! the currently visible x range.

use crate::expr::{CompiledExpression, EvalContext};
use plotters::coord::Shift;
use plotters::prelude::*;
use std::cmp::Ordering;
use std::path::Path;

const DEFAULT_X_MIN: f64 = -10.0;
const DEFAULT_X_MAX: f64 = 10.0;
const DEFAULT_Y_MIN: f64 = -10.0;
const DEFAULT_Y_MAX: f64 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            x_min: DEFAULT_X_MIN,
            x_max: DEFAULT_X_MAX,
            y_min: DEFAULT_Y_MIN,
            y_max: DEFAULT_Y_MAX,
        }
    }
}

impl Viewport {
    pub fn x_span(self) -> f64 {
        self.x_max - self.x_min
    }

    pub fn y_span(self) -> f64 {
        self.y_max - self.y_min
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Png,
    Jpeg,
    Svg,
}

#[derive(Clone, Debug)]
pub enum RootResult {
    NotPlotted,
    Roots(Vec<f64>),
    None,
    Infinite,
    Unreliable,
}

#[derive(Clone, Debug)]
pub struct GraphModel {
    input: String,
    compiled: Option<CompiledExpression>,
    viewport: Viewport,
    root_result: RootResult,
    error: Option<String>,
}

impl Default for GraphModel {
    fn default() -> Self {
        Self {
            input: String::new(),
            compiled: None,
            viewport: Viewport::default(),
            root_result: RootResult::NotPlotted,
            error: None,
        }
    }
}

impl GraphModel {
    pub fn viewport(&self) -> Viewport {
        self.viewport
    }

    pub fn root_result(&self) -> &RootResult {
        &self.root_result
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn has_plot(&self) -> bool {
        self.compiled.is_some() && self.error.is_none()
    }

    pub fn plot(&mut self, input: &str, ctx: EvalContext) -> Result<(), String> {
        // A failed plot attempt must replace the previous graph rather than
        // leaving an old curve/caption visible beside a new error message.
        self.input = input.trim().to_string();
        self.compiled = None;
        self.viewport = Viewport::default();
        self.root_result = RootResult::NotPlotted;
        self.error = None;

        let normalized = match normalize_graph_expression(input) {
            Ok(expression) => expression,
            Err(error) => {
                self.error = Some(error.clone());
                return Err(error);
            }
        };
        let compiled = match CompiledExpression::parse(&normalized, ctx) {
            Ok(expression) => expression,
            Err(error) => {
                self.error = Some(error.clone());
                return Err(error);
            }
        };
        self.compiled = Some(compiled);

        if let Err(error) = self.auto_range_y() {
            self.error = Some(error.clone());
            self.root_result = RootResult::Unreliable;
            return Err(error);
        }
        self.refresh_roots();
        Ok(())
    }

    pub fn reset_view(&mut self) {
        self.viewport.x_min = DEFAULT_X_MIN;
        self.viewport.x_max = DEFAULT_X_MAX;
        self.viewport.y_min = DEFAULT_Y_MIN;
        self.viewport.y_max = DEFAULT_Y_MAX;
        if self.compiled.is_some() {
            let _ = self.auto_range_y();
            self.refresh_roots();
        }
    }

    pub fn zoom(&mut self, wheel_rotation: i32, focus_x: f64, focus_y: f64) {
        if self.compiled.is_none() || wheel_rotation == 0 {
            return;
        }
        let steps = (wheel_rotation as f64 / 120.0).clamp(-8.0, 8.0);
        let factor = 0.85_f64.powf(steps);
        let fx = focus_x.clamp(0.0, 1.0);
        let fy = focus_y.clamp(0.0, 1.0);
        let anchor_x = self.viewport.x_min + fx * self.viewport.x_span();
        // Screen y grows downward.
        let anchor_y = self.viewport.y_max - fy * self.viewport.y_span();
        let new_x_span = (self.viewport.x_span() * factor).clamp(1.0e-9, 1.0e12);
        let new_y_span = (self.viewport.y_span() * factor).clamp(1.0e-9, 1.0e12);
        self.viewport.x_min = anchor_x - fx * new_x_span;
        self.viewport.x_max = self.viewport.x_min + new_x_span;
        self.viewport.y_max = anchor_y + fy * new_y_span;
        self.viewport.y_min = self.viewport.y_max - new_y_span;
        self.refresh_roots();
    }

    pub fn pan_from(&mut self, initial: Viewport, dx: i32, dy: i32, width: i32, height: i32) {
        if width <= 0 || height <= 0 {
            return;
        }
        let x_delta = -(dx as f64 / width as f64) * initial.x_span();
        let y_delta = (dy as f64 / height as f64) * initial.y_span();
        self.viewport = Viewport {
            x_min: initial.x_min + x_delta,
            x_max: initial.x_max + x_delta,
            y_min: initial.y_min + y_delta,
            y_max: initial.y_max + y_delta,
        };
        self.refresh_roots();
    }

    pub fn draw<B: DrawingBackend>(
        &self,
        area: &DrawingArea<B, Shift>,
        decimal_separator: char,
    ) -> Result<(), String>
    where
        B::ErrorType: std::fmt::Debug,
    {
        area.fill(&WHITE).map_err(draw_error)?;
        let caption = if self.input.trim().is_empty() {
            "f(x)".to_string()
        } else {
            self.input.clone()
        };
        self.draw_chart(area, decimal_separator, Some(&caption), true)?;
        area.present().map_err(draw_error)
    }

    /// Exports a presentation-oriented page rather than merely dumping the
    /// on-screen canvas.  The function receives its own typographic header,
    /// the chart keeps only unobtrusive root markers, and the localized root
    /// summary is placed in a dedicated footer where it cannot collide with
    /// the curve or axis labels.
    pub fn export(
        &self,
        path: &Path,
        format: ExportFormat,
        size: (u32, u32),
        decimal_separator: char,
        root_summary: &str,
    ) -> Result<(), String> {
        if !self.has_plot() {
            return Err("Plot a valid function before exporting.".to_string());
        }

        // A very wide but shallow graph canvas made the former export look
        // cramped.  Keep the current aspect as a hint, but reserve enough
        // height for a readable chart plus the new header and root footer.
        let export_size = (size.0.max(960), size.1.max(620));
        match format {
            ExportFormat::Png | ExportFormat::Jpeg => {
                let area = BitMapBackend::new(path, export_size).into_drawing_area();
                self.draw_export(&area, decimal_separator, root_summary)
            }
            ExportFormat::Svg => {
                let area = SVGBackend::new(path, export_size).into_drawing_area();
                self.draw_export(&area, decimal_separator, root_summary)
            }
        }
    }

    fn draw_export<B: DrawingBackend>(
        &self,
        area: &DrawingArea<B, Shift>,
        decimal_separator: char,
        root_summary: &str,
    ) -> Result<(), String>
    where
        B::ErrorType: std::fmt::Debug,
    {
        area.fill(&WHITE).map_err(draw_error)?;
        let (width, height) = area.dim_in_pixel();
        let root_lines = wrap_export_summary(root_summary, 84, 3);
        let header_height = 94_u32.min(height.saturating_sub(2));
        let requested_footer_height = 76_u32 + root_lines.len() as u32 * 24;
        let footer_height = requested_footer_height
            .min(height.saturating_sub(header_height + 1));
        let chart_height = height
            .saturating_sub(header_height)
            .saturating_sub(footer_height)
            .max(1);
        let (header, remainder) = area.split_vertically(header_height);
        let (chart_area, footer) = remainder.split_vertically(chart_height);

        let equation = format_function_for_export(&self.input, decimal_separator);
        let title_size = export_title_font_size(&equation);
        header
            .draw(&Text::new(
                "OpenCalc",
                (28, 25),
                ("sans-serif", 14)
                    .into_font()
                    .color(&RGBColor(96, 96, 96)),
            ))
            .map_err(draw_error)?;
        header
            .draw(&Text::new(
                equation,
                (28, 63),
                ("sans-serif", title_size).into_font().color(&BLACK),
            ))
            .map_err(draw_error)?;
        header
            .draw(&PathElement::new(
                [(24, 88), (width.saturating_sub(24) as i32, 88)],
                RGBColor(208, 208, 208).stroke_width(1),
            ))
            .map_err(draw_error)?;

        self.draw_chart(&chart_area, decimal_separator, None, false)?;

        let footer_width = footer.dim_in_pixel().0;
        footer
            .draw(&PathElement::new(
                [(24, 8), (footer_width.saturating_sub(24) as i32, 8)],
                RGBColor(208, 208, 208).stroke_width(1),
            ))
            .map_err(draw_error)?;
        for (index, line) in root_lines.iter().enumerate() {
            footer
                .draw(&Text::new(
                    line.clone(),
                    (28, 35 + index as i32 * 24),
                    ("sans-serif", 16).into_font().color(&BLACK),
                ))
                .map_err(draw_error)?;
        }
        let range = format!(
            "x: {} … {}    y: {} … {}",
            format_number(self.viewport.x_min, decimal_separator, 6),
            format_number(self.viewport.x_max, decimal_separator, 6),
            format_number(self.viewport.y_min, decimal_separator, 6),
            format_number(self.viewport.y_max, decimal_separator, 6),
        );
        footer
            .draw(&Text::new(
                range,
                (28, 52 + root_lines.len() as i32 * 24),
                ("sans-serif", 13)
                    .into_font()
                    .color(&RGBColor(96, 96, 96)),
            ))
            .map_err(draw_error)?;

        area.present().map_err(draw_error)
    }

    fn draw_chart<B: DrawingBackend>(
        &self,
        area: &DrawingArea<B, Shift>,
        decimal_separator: char,
        caption: Option<&str>,
        label_roots: bool,
    ) -> Result<(), String>
    where
        B::ErrorType: std::fmt::Debug,
    {
        area.fill(&WHITE).map_err(draw_error)?;
        let mut builder = ChartBuilder::on(area);
        builder
            .margin(if caption.is_some() { 8 } else { 18 })
            .x_label_area_size(if caption.is_some() { 28 } else { 42 })
            .y_label_area_size(if caption.is_some() { 42 } else { 58 });
        if let Some(caption) = caption {
            builder.caption(caption, ("sans-serif", 13).into_font());
        }
        let mut chart = builder
            .build_cartesian_2d(
                self.viewport.x_min..self.viewport.x_max,
                self.viewport.y_min..self.viewport.y_max,
            )
            .map_err(draw_error)?;

        let x_formatter = |value: &f64| format_axis_number(*value, decimal_separator);
        let y_formatter = |value: &f64| format_axis_number(*value, decimal_separator);
        chart
            .configure_mesh()
            .x_labels(7)
            .y_labels(7)
            .x_label_formatter(&x_formatter)
            .y_label_formatter(&y_formatter)
            .light_line_style(RGBColor(232, 232, 232))
            .bold_line_style(RGBColor(196, 196, 196))
            .axis_style(BLACK)
            .label_style(
                ("sans-serif", if caption.is_some() { 11 } else { 14 }).into_font(),
            )
            .draw()
            .map_err(draw_error)?;

        // Draw the mathematical axes distinctly from the lighter grid whenever
        // zero is inside the current viewport.
        if self.viewport.y_min <= 0.0 && self.viewport.y_max >= 0.0 {
            chart
                .draw_series(LineSeries::new(
                    [(self.viewport.x_min, 0.0), (self.viewport.x_max, 0.0)],
                    BLACK.mix(0.65).stroke_width(1),
                ))
                .map_err(draw_error)?;
        }
        if self.viewport.x_min <= 0.0 && self.viewport.x_max >= 0.0 {
            chart
                .draw_series(LineSeries::new(
                    [(0.0, self.viewport.y_min), (0.0, self.viewport.y_max)],
                    BLACK.mix(0.65).stroke_width(1),
                ))
                .map_err(draw_error)?;
        }

        if self.compiled.is_some() && self.error.is_none() {
            let width = area.dim_in_pixel().0.max(200) as usize;
            for segment in self.sample_segments((width * 2).clamp(400, 4000)) {
                if segment.len() > 1 {
                    chart
                        .draw_series(LineSeries::new(segment, BLUE.stroke_width(2)))
                        .map_err(draw_error)?;
                }
            }
            if let RootResult::Roots(roots) = &self.root_result {
                if label_roots {
                    chart
                        .draw_series(roots.iter().copied().map(|x| {
                            let label = format_number(x, decimal_separator, 6);
                            EmptyElement::at((x, 0.0))
                                + Circle::new((0, 0), 4, RED.filled())
                                + Text::new(
                                    label,
                                    (7, -8),
                                    ("sans-serif", 10).into_font().color(&RED),
                                )
                        }))
                        .map_err(draw_error)?;
                } else {
                    chart
                        .draw_series(
                            roots
                                .iter()
                                .copied()
                                .map(|x| Circle::new((x, 0.0), 5, RED.filled())),
                        )
                        .map_err(draw_error)?;
                }
            }
        }
        Ok(())
    }

    fn evaluate(&self, x: f64) -> Option<f64> {
        let value = self.compiled.as_ref()?.evaluate_at(x).ok()?;
        value.is_finite().then_some(value)
    }

    fn auto_range_y(&mut self) -> Result<(), String> {
        let mut values = Vec::new();
        for index in 0..1200 {
            let ratio = index as f64 / 1199.0;
            let x = self.viewport.x_min + ratio * self.viewport.x_span();
            if let Some(y) = self.evaluate(x) {
                if y.abs() < 1.0e100 {
                    values.push(y);
                }
            }
        }
        if values.is_empty() {
            return Err("The function has no finite values in the default graph range.".to_string());
        }
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        let low_index = ((values.len() - 1) as f64 * 0.02).round() as usize;
        let high_index = ((values.len() - 1) as f64 * 0.98).round() as usize;
        let mut low = values[low_index.min(values.len() - 1)].min(0.0);
        let mut high = values[high_index.min(values.len() - 1)].max(0.0);
        if (high - low).abs() < 1.0e-12 {
            let center = (high + low) / 2.0;
            low = center - 1.0;
            high = center + 1.0;
        } else {
            let padding = (high - low) * 0.10;
            low -= padding;
            high += padding;
        }
        self.viewport.y_min = low;
        self.viewport.y_max = high;
        Ok(())
    }

    fn sample_segments(&self, samples: usize) -> Vec<Vec<(f64, f64)>> {
        let mut segments = Vec::new();
        let mut current = Vec::new();
        let jump_limit = self.viewport.y_span().abs().max(1.0) * 3.0;
        let mut previous_y: Option<f64> = None;
        for index in 0..samples.max(2) {
            let ratio = index as f64 / (samples.max(2) - 1) as f64;
            let x = self.viewport.x_min + ratio * self.viewport.x_span();
            match self.evaluate(x) {
                Some(y)
                    if y >= self.viewport.y_min - self.viewport.y_span()
                        && y <= self.viewport.y_max + self.viewport.y_span() =>
                {
                    if previous_y.is_some_and(|previous| (y - previous).abs() > jump_limit) {
                        if current.len() > 1 {
                            segments.push(std::mem::take(&mut current));
                        } else {
                            current.clear();
                        }
                    }
                    current.push((x, y));
                    previous_y = Some(y);
                }
                _ => {
                    if current.len() > 1 {
                        segments.push(std::mem::take(&mut current));
                    } else {
                        current.clear();
                    }
                    previous_y = None;
                }
            }
        }
        if current.len() > 1 {
            segments.push(current);
        }
        segments
    }

    fn refresh_roots(&mut self) {
        let Some(compiled) = self.compiled.as_ref() else {
            self.root_result = RootResult::NotPlotted;
            return;
        };
        let span = self.viewport.x_span();
        if !span.is_finite() || span <= 0.0 {
            self.root_result = RootResult::Unreliable;
            return;
        }

        let samples = 2000usize;
        let step = span / samples as f64;
        let y_tolerance = (self.viewport.y_span().abs() * 1.0e-7).max(1.0e-10);
        let probe_tolerance = (self.viewport.y_span().abs() * 2.0e-4).max(1.0e-7);
        let mut points = Vec::with_capacity(samples + 1);
        let mut finite_count = 0usize;
        let mut near_zero_count = 0usize;
        for index in 0..=samples {
            let x = self.viewport.x_min + index as f64 * step;
            let y = compiled.evaluate_at(x).ok().filter(|value| value.is_finite());
            if let Some(value) = y {
                finite_count += 1;
                if value.abs() <= y_tolerance {
                    near_zero_count += 1;
                }
            }
            points.push((x, y));
        }
        if finite_count == 0 {
            self.root_result = RootResult::Unreliable;
            return;
        }
        if near_zero_count > finite_count * 9 / 10 {
            self.root_result = RootResult::Infinite;
            return;
        }

        let mut roots = Vec::new();
        for pair in points.windows(2) {
            let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
            match (y0, y1) {
                (Some(a), Some(_)) if a == 0.0 => roots.push(x0),
                (Some(a), Some(b)) if a.signum() != b.signum() => {
                    if let Some(root) = bisect_root(compiled, x0, x1, y_tolerance) {
                        roots.push(root);
                    }
                }
                _ => {}
            }
        }

        // Sign-change bracketing misses tangent roots such as (x-2)^2. Probe
        // local minima of |f| and refine them with a finite-difference Newton step.
        for triple in points.windows(3) {
            let ((x0, y0), (xm, ym), (x1, y1)) = (triple[0], triple[1], triple[2]);
            let (Some(a), Some(m), Some(b)) = (y0, ym, y1) else { continue };
            if m.abs() <= a.abs() && m.abs() <= b.abs() && m.abs() <= probe_tolerance {
                if let Some(root) = refine_tangent_root(compiled, xm, x0, x1, y_tolerance) {
                    roots.push(root);
                }
            }
        }

        roots.retain(|root| {
            *root >= self.viewport.x_min - step
                && *root <= self.viewport.x_max + step
                && compiled
                    .evaluate_at(*root)
                    .ok()
                    .is_some_and(|value| value.is_finite() && value.abs() <= probe_tolerance)
        });
        roots.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        let merge_tolerance = (span * 1.0e-6).max(1.0e-9);
        roots.dedup_by(|a, b| (*a - *b).abs() <= merge_tolerance);
        self.root_result = if roots.is_empty() {
            RootResult::None
        } else {
            RootResult::Roots(roots)
        };
    }
}

/// Formats the user-entered function for the exported page without changing
/// the expression that is evaluated.  Common calculator notation is converted
/// to readable mathematical typography: powers become Unicode superscripts,
/// multiplication becomes a centered dot, `pi` becomes π, and a bare
/// expression receives an explicit `f(x) =` prefix.
pub fn format_function_for_export(input: &str, decimal_separator: char) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "f(x)".to_string();
    }

    let equals_count = trimmed.chars().filter(|ch| *ch == '=').count();
    if equals_count == 1 {
        let mut sides = trimmed.splitn(2, '=');
        let left = sides.next().unwrap_or_default().trim();
        let right = sides.next().unwrap_or_default().trim();
        let compact_left: String = left
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .flat_map(char::to_lowercase)
            .collect();
        if compact_left == "y" || compact_left == "f(x)" {
            return format!(
                "f(x) = {}",
                prettify_math_expression(right, decimal_separator)
            );
        }
        return format!(
            "{} = {}",
            prettify_math_expression(left, decimal_separator),
            prettify_math_expression(right, decimal_separator)
        );
    }

    format!(
        "f(x) = {}",
        prettify_math_expression(trimmed, decimal_separator)
    )
}

fn wrap_export_summary(text: &str, max_chars: usize, max_lines: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let additional = word.chars().count() + usize::from(!current.is_empty());
        if !current.is_empty() && current.chars().count() + additional > max_chars {
            lines.push(std::mem::take(&mut current));
            if lines.len() == max_lines {
                let last = lines.last_mut().expect("a line was just added");
                if !last.ends_with('…') {
                    last.push_str(" …");
                }
                return lines;
            }
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn export_title_font_size(equation: &str) -> u32 {
    match equation.chars().count() {
        0..=38 => 27,
        39..=58 => 23,
        59..=82 => 20,
        _ => 17,
    }
}

fn prettify_math_expression(expression: &str, decimal_separator: char) -> String {
    let chars: Vec<char> = expression.chars().collect();
    let mut compact = String::with_capacity(expression.len() + 8);
    let mut index = 0usize;
    while index < chars.len() {
        let ch = chars[index];

        let exponent_start = if ch == '^' {
            Some(index + 1)
        } else if ch == '*' && chars.get(index + 1) == Some(&'*') {
            Some(index + 2)
        } else {
            None
        };
        if let Some(mut cursor) = exponent_start {
            let mut exponent = String::new();
            if chars.get(cursor) == Some(&'-') {
                exponent.push('⁻');
                cursor += 1;
            } else if chars.get(cursor) == Some(&'+') {
                exponent.push('⁺');
                cursor += 1;
            }
            let digit_start = cursor;
            while let Some(digit) = chars.get(cursor).and_then(|value| superscript_digit(*value)) {
                exponent.push(digit);
                cursor += 1;
            }
            // Unicode has no dependable superscript decimal separator. Keep
            // fractional and expression exponents in caret notation rather
            // than producing misleading mixed-height text such as ⁰.5.
            let exponent_is_integer = cursor > digit_start
                && !matches!(chars.get(cursor), Some(&'.') | Some(&','));
            if exponent_is_integer {
                compact.push_str(&exponent);
                index = cursor;
                continue;
            }
            compact.push('^');
            index = exponent_start.unwrap_or(index + 1);
            continue;
        }
        if ch == '*' {
            compact.push('·');
            index += 1;
            continue;
        }
        if ch == '-' {
            compact.push('−');
            index += 1;
            continue;
        }
        if ch == '.' && decimal_separator == ',' {
            compact.push(',');
            index += 1;
            continue;
        }
        if is_pi_token(&chars, index) {
            compact.push('π');
            index += 2;
            continue;
        }
        if starts_ascii_case_insensitive(&chars, index, "sqrt(") {
            compact.push('√');
            compact.push('(');
            index += 5;
            continue;
        }
        compact.push(ch);
        index += 1;
    }

    space_math_operators(&compact)
}

fn superscript_digit(ch: char) -> Option<char> {
    Some(match ch {
        '0' => '⁰',
        '1' => '¹',
        '2' => '²',
        '3' => '³',
        '4' => '⁴',
        '5' => '⁵',
        '6' => '⁶',
        '7' => '⁷',
        '8' => '⁸',
        '9' => '⁹',
        _ => return None,
    })
}

fn is_pi_token(chars: &[char], index: usize) -> bool {
    let Some(first) = chars.get(index) else { return false };
    let Some(second) = chars.get(index + 1) else { return false };
    if !first.eq_ignore_ascii_case(&'p') || !second.eq_ignore_ascii_case(&'i') {
        return false;
    }
    let before_is_word = index
        .checked_sub(1)
        .and_then(|position| chars.get(position))
        .is_some_and(|ch| ch.is_ascii_alphanumeric() || *ch == '_');
    let after_is_word = chars
        .get(index + 2)
        .is_some_and(|ch| ch.is_ascii_alphanumeric() || *ch == '_');
    !before_is_word && !after_is_word
}

fn starts_ascii_case_insensitive(chars: &[char], index: usize, needle: &str) -> bool {
    let needle_chars: Vec<char> = needle.chars().collect();
    let Some(slice) = chars.get(index..index + needle_chars.len()) else {
        return false;
    };
    slice
        .iter()
        .zip(needle_chars.iter())
        .all(|(left, right)| left.eq_ignore_ascii_case(right))
}

fn space_math_operators(expression: &str) -> String {
    let chars: Vec<char> = expression.chars().collect();
    let mut out = String::with_capacity(expression.len() + 12);
    for (index, ch) in chars.iter().copied().enumerate() {
        let is_unary_minus = ch == '−'
            && chars[..index]
                .iter()
                .rev()
                .find(|candidate| !candidate.is_whitespace())
                .is_none_or(|previous| matches!(previous, '(' | '+' | '−' | '·' | '/' | '=' | ','));
        let spaced = matches!(ch, '+' | '=' | '·' | '/') || (ch == '−' && !is_unary_minus);
        if spaced {
            while out.ends_with(' ') {
                out.pop();
            }
            if !out.is_empty() {
                out.push(' ');
            }
            out.push(ch);
            out.push(' ');
        } else if ch.is_whitespace() {
            if !out.is_empty() && !out.ends_with(' ') {
                out.push(' ');
            }
        } else {
            out.push(ch);
        }
    }
    out.trim().to_string()
}

pub fn normalize_graph_expression(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Enter a function or equation to plot.".to_string());
    }
    let parts: Vec<&str> = trimmed.split('=').collect();
    let expression = match parts.as_slice() {
        [expression] => expression.trim().to_string(),
        [left, right] => {
            let left_trimmed = left.trim();
            let right_trimmed = right.trim();
            if left_trimmed.is_empty() || right_trimmed.is_empty() {
                return Err("Both sides of an equation must contain an expression.".to_string());
            }
            let compact_left: String = left_trimmed
                .chars()
                .filter(|ch| !ch.is_whitespace())
                .flat_map(char::to_lowercase)
                .collect();
            if compact_left == "y" || compact_left == "f(x)" {
                right_trimmed.to_string()
            } else {
                format!("({left_trimmed})-({right_trimmed})")
            }
        }
        _ => return Err("Enter one function or one equation only.".to_string()),
    };
    normalize_graph_notation(&expression)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum GraphLexemeKind {
    Number,
    ValueIdentifier,
    Function,
    WordOperator,
    UnknownIdentifier,
    Operator,
    LeftParen,
    RightParen,
    Postfix,
}

#[derive(Clone, Debug)]
struct GraphLexeme {
    text: String,
    kind: GraphLexemeKind,
}

/// Accept notation commonly used in graphing calculators without changing the
/// ordinary clipboard-expression grammar. In graph mode only, Unicode
/// superscripts are expanded and juxtaposition becomes multiplication.
fn normalize_graph_notation(input: &str) -> Result<String, String> {
    let expanded = expand_superscript_powers(input)?;
    let lexemes = lex_graph_notation(&expanded)?;
    let mut output = String::with_capacity(expanded.len() + 8);
    for (index, lexeme) in lexemes.iter().enumerate() {
        if index > 0 && graph_needs_implicit_multiply(&lexemes[index - 1], lexeme) {
            output.push('*');
        }
        output.push_str(&lexeme.text);
    }
    Ok(output)
}

fn expand_superscript_powers(input: &str) -> Result<String, String> {
    let mut output = String::with_capacity(input.len() + 4);
    let mut in_superscript = false;
    for ch in input.chars() {
        let ordinary = match ch {
            '⁰' => Some('0'),
            '¹' => Some('1'),
            '²' => Some('2'),
            '³' => Some('3'),
            '⁴' => Some('4'),
            '⁵' => Some('5'),
            '⁶' => Some('6'),
            '⁷' => Some('7'),
            '⁸' => Some('8'),
            '⁹' => Some('9'),
            '⁺' => Some('+'),
            '⁻' => Some('-'),
            _ => None,
        };
        if let Some(ordinary) = ordinary {
            if !in_superscript {
                if output.trim_end().is_empty() {
                    return Err(
                        "A superscript must follow a number, x, constant, or parenthesized expression."
                            .to_string(),
                    );
                }
                if !output.trim_end().ends_with('^') {
                    output.push('^');
                }
                in_superscript = true;
            }
            output.push(ordinary);
        } else {
            in_superscript = false;
            output.push(ch);
        }
    }
    Ok(output)
}

fn lex_graph_notation(input: &str) -> Result<Vec<GraphLexeme>, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut lexemes = Vec::new();
    let mut index = 0usize;
    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() {
            index += 1;
            continue;
        }
        if ch.is_ascii_digit() || ch == '.' || ch == ',' {
            let start = index;
            while index < chars.len()
                && (chars[index].is_ascii_digit()
                    || matches!(chars[index], '.' | ',' | '\u{00a0}' | '\u{202f}'))
            {
                index += 1;
            }
            if index < chars.len() && matches!(chars[index], 'e' | 'E') {
                let mut cursor = index + 1;
                if cursor < chars.len() && matches!(chars[cursor], '+' | '-') {
                    cursor += 1;
                }
                let digit_start = cursor;
                while cursor < chars.len() && chars[cursor].is_ascii_digit() {
                    cursor += 1;
                }
                if cursor > digit_start {
                    index = cursor;
                }
            }
            lexemes.push(GraphLexeme {
                text: chars[start..index].iter().collect(),
                kind: GraphLexemeKind::Number,
            });
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '_' || matches!(ch, 'π' | 'Π') {
            let start = index;
            if matches!(ch, 'π' | 'Π') {
                index += 1;
            } else {
                index += 1;
                while index < chars.len()
                    && (chars[index].is_ascii_alphanumeric() || chars[index] == '_')
                {
                    index += 1;
                }
            }
            let text: String = chars[start..index].iter().collect();
            let lower = if matches!(text.as_str(), "π" | "Π") {
                "pi".to_string()
            } else {
                text.to_ascii_lowercase()
            };
            let kind = if matches!(lower.as_str(), "x" | "pi" | "e") {
                GraphLexemeKind::ValueIdentifier
            } else if matches!(
                lower.as_str(),
                "mod" | "and" | "or" | "xor" | "lsh" | "root"
            ) {
                GraphLexemeKind::WordOperator
            } else if matches!(
                lower.as_str(),
                "sqrt"
                    | "sin"
                    | "cos"
                    | "tan"
                    | "asin"
                    | "acos"
                    | "atan"
                    | "sinh"
                    | "cosh"
                    | "tanh"
                    | "asinh"
                    | "acosh"
                    | "atanh"
                    | "ln"
                    | "log"
                    | "log10"
                    | "exp"
                    | "abs"
                    | "int"
                    | "floor"
                    | "ceil"
                    | "fact"
                    | "factorial"
            ) {
                GraphLexemeKind::Function
            } else {
                GraphLexemeKind::UnknownIdentifier
            };
            lexemes.push(GraphLexeme { text, kind });
            continue;
        }
        let (text, kind, advance) = match ch {
            '*' if chars.get(index + 1) == Some(&'*') => {
                ("**".to_string(), GraphLexemeKind::Operator, 2)
            }
            '+' | '-' | '−' | '*' | '×' | '/' | '÷' | '^' | '=' => {
                (ch.to_string(), GraphLexemeKind::Operator, 1)
            }
            '(' => ("(".to_string(), GraphLexemeKind::LeftParen, 1),
            ')' => (")".to_string(), GraphLexemeKind::RightParen, 1),
            '!' | '%' => (ch.to_string(), GraphLexemeKind::Postfix, 1),
            _ => return Err(format!("Invalid character '{ch}' in graph expression.")),
        };
        lexemes.push(GraphLexeme { text, kind });
        index += advance;
    }
    Ok(lexemes)
}

fn graph_needs_implicit_multiply(previous: &GraphLexeme, current: &GraphLexeme) -> bool {
    // Preserve function-call syntax. Unknown identifiers followed by `(` are
    // left intact so the shared parser can report its proper unknown-function
    // error, and pi() remains the supported zero-argument constant spelling.
    if current.kind == GraphLexemeKind::LeftParen
        && (previous.kind == GraphLexemeKind::UnknownIdentifier
            || previous.text.eq_ignore_ascii_case("pi")
            || matches!(previous.text.as_str(), "π" | "Π"))
    {
        return false;
    }
    let previous_ends_value = matches!(
        previous.kind,
        GraphLexemeKind::Number
            | GraphLexemeKind::ValueIdentifier
            | GraphLexemeKind::UnknownIdentifier
            | GraphLexemeKind::RightParen
            | GraphLexemeKind::Postfix
    );
    let current_starts_value = matches!(
        current.kind,
        GraphLexemeKind::Number
            | GraphLexemeKind::ValueIdentifier
            | GraphLexemeKind::Function
            | GraphLexemeKind::UnknownIdentifier
            | GraphLexemeKind::LeftParen
    );
    previous_ends_value && current_starts_value
}

pub fn format_root_values(roots: &[f64], decimal_separator: char) -> String {
    roots
        .iter()
        .map(|value| format_number(*value, decimal_separator, 8))
        .collect::<Vec<_>>()
        .join("; ")
}

fn bisect_root(
    expression: &CompiledExpression,
    mut left: f64,
    mut right: f64,
    tolerance: f64,
) -> Option<f64> {
    let mut f_left = expression.evaluate_at(left).ok()?;
    let f_right = expression.evaluate_at(right).ok()?;
    if !f_left.is_finite() || !f_right.is_finite() || f_left.signum() == f_right.signum() {
        return None;
    }
    for _ in 0..80 {
        let middle = (left + right) / 2.0;
        let value = expression.evaluate_at(middle).ok()?;
        if !value.is_finite() {
            return None;
        }
        if value.abs() <= tolerance || (right - left).abs() <= 1.0e-12 {
            return Some(middle);
        }
        if f_left.signum() != value.signum() {
            right = middle;
        } else {
            left = middle;
            f_left = value;
        }
    }
    let root = (left + right) / 2.0;
    expression
        .evaluate_at(root)
        .ok()
        .filter(|value| value.is_finite() && value.abs() <= tolerance * 100.0)
        .map(|_| root)
}

fn refine_tangent_root(
    expression: &CompiledExpression,
    mut x: f64,
    left: f64,
    right: f64,
    tolerance: f64,
) -> Option<f64> {
    for _ in 0..24 {
        let value = expression.evaluate_at(x).ok()?;
        if value.abs() <= tolerance {
            return Some(x);
        }
        let h = ((right - left).abs() * 1.0e-3).max(1.0e-9);
        let low = expression.evaluate_at((x - h).max(left)).ok()?;
        let high = expression.evaluate_at((x + h).min(right)).ok()?;
        let derivative = (high - low) / (2.0 * h);
        if !derivative.is_finite() || derivative.abs() < 1.0e-14 {
            break;
        }
        let next = (x - value / derivative).clamp(left, right);
        if (next - x).abs() < 1.0e-13 {
            x = next;
            break;
        }
        x = next;
    }
    expression
        .evaluate_at(x)
        .ok()
        .filter(|value| value.is_finite() && value.abs() <= tolerance * 100.0)
        .map(|_| x)
}

fn format_axis_number(value: f64, decimal_separator: char) -> String {
    format_number(value, decimal_separator, 5)
}

fn format_number(value: f64, decimal_separator: char, precision: usize) -> String {
    let normalized = if value.abs() < 5.0e-13 { 0.0 } else { value };
    let mut text = if normalized.abs() >= 1.0e7
        || (normalized != 0.0 && normalized.abs() < 1.0e-5)
    {
        format!("{normalized:.4e}")
    } else {
        let mut plain = format!("{normalized:.precision$}");
        while plain.contains('.') && plain.ends_with('0') {
            plain.pop();
        }
        if plain.ends_with('.') {
            plain.pop();
        }
        plain
    };
    if decimal_separator != '.' {
        text = text.replace('.', &decimal_separator.to_string());
    }
    text
}

fn draw_error<E: std::fmt::Debug>(error: E) -> String {
    format!("Graph rendering failed: {error:?}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::AngleMode;

    fn context() -> EvalContext {
        EvalContext {
            angle: AngleMode::Radians,
            decimal_separator: '.',
            thousands_separator: Some(','),
        }
    }

    #[test]
    fn graph_starts_blank_and_unplotted() {
        let model = GraphModel::default();
        assert!(model.input.is_empty());
        assert!(!model.has_plot());
        assert!(matches!(model.root_result(), RootResult::NotPlotted));
    }

    #[test]
    fn normalizes_function_and_equation_spellings() {
        assert_eq!(normalize_graph_expression("x^2-4").unwrap(), "x^2-4");
        assert_eq!(normalize_graph_expression("y = x^2-4").unwrap(), "x^2-4");
        assert_eq!(normalize_graph_expression("f(x)=sin(x)").unwrap(), "sin(x)");
        assert_eq!(normalize_graph_expression("x^2 = 4").unwrap(), "(x^2)-(4)");
    }

    #[test]
    fn accepts_graphing_calculator_power_and_implicit_multiplication_notation() {
        assert_eq!(normalize_graph_expression("2x² + 2x + 2").unwrap(), "2*x^2+2*x+2");
        assert_eq!(normalize_graph_expression("2x^2 + 2x + 2").unwrap(), "2*x^2+2*x+2");
        assert_eq!(normalize_graph_expression("2x**2 + 2x + 2").unwrap(), "2*x**2+2*x+2");
        assert_eq!(normalize_graph_expression("2x**2 + 3x + 10").unwrap(), "2*x**2+3*x+10");
        assert_eq!(normalize_graph_expression("2(x+1)").unwrap(), "2*(x+1)");
        assert_eq!(normalize_graph_expression("(x+1)(x-1)").unwrap(), "(x+1)*(x-1)");
        assert_eq!(normalize_graph_expression("2sqrt(x)").unwrap(), "2*sqrt(x)");
        assert_eq!(normalize_graph_expression("2floor(x)").unwrap(), "2*floor(x)");
        assert_eq!(normalize_graph_expression("pi()").unwrap(), "pi()");
    }

    #[test]
    fn normalizes_implicit_products_with_powers_parentheses_and_functions() {
        for (expression, normalized) in [
            ("2x**3 + 4", "2*x**3+4"),
            ("3(x + 1)(x - 1)", "3*(x+1)*(x-1)"),
            ("2sin(x) + pi x", "2*sin(x)+pi*x"),
        ] {
            assert_eq!(normalize_graph_expression(expression).unwrap(), normalized);
            let mut model = GraphModel::default();
            model.plot(expression, context()).unwrap();
            assert!(model.has_plot(), "{expression}");
        }
    }

    #[test]
    fn plots_quadratics_with_caret_double_star_or_superscript_notation() {
        for expression in [
            "2x² + 2x + 2",
            "2x^2 + 2x + 2",
            "2x**2 + 2x + 2",
            "2x**2 + 3x + 10",
        ] {
            let mut model = GraphModel::default();
            model.plot(expression, context()).unwrap();
            assert!(model.has_plot(), "{expression}");
            assert!(matches!(model.root_result(), RootResult::None));
        }
    }

    #[test]
    fn failed_plot_replaces_the_previous_curve_and_records_the_new_error() {
        let mut model = GraphModel::default();
        model.plot("x^2", context()).unwrap();
        assert!(model.plot("unknown(x)", context()).is_err());
        assert!(!model.has_plot());
        assert!(model.error().is_some());
        assert_eq!(model.input, "unknown(x)");
    }

    #[test]
    fn formats_export_equations_typographically() {
        assert_eq!(format_function_for_export("x^2 - 4", '.'), "f(x) = x² − 4");
        assert_eq!(format_function_for_export("f(x)=sin(x)", '.'), "f(x) = sin(x)");
        assert_eq!(format_function_for_export("x^2=4", '.'), "x² = 4");
        assert_eq!(format_function_for_export("2*x^10", '.'), "f(x) = 2 · x¹⁰");
        assert_eq!(format_function_for_export("9**0.5", ','), "f(x) = 9^0,5");
        assert_eq!(format_function_for_export("sqrt(x)-pi", '.'), "f(x) = √(x) − π");
    }

    #[test]
    fn wraps_long_export_root_summaries_without_losing_the_heading() {
        let lines = wrap_export_summary(
            "Roots in visible range: x = -9; -8; -7; -6; -5; -4; -3; -2; -1; 0; 1; 2; 3; 4; 5; 6; 7; 8; 9",
            42,
            3,
        );
        assert!(lines.len() <= 3);
        assert!(lines[0].starts_with("Roots in visible range:"));
    }

    #[test]
    fn finds_crossing_and_tangent_roots() {
        let mut model = GraphModel::default();
        model.plot("x^2-4", context()).unwrap();
        let RootResult::Roots(roots) = model.root_result() else { panic!("roots expected") };
        assert!(roots.iter().any(|root| (*root + 2.0).abs() < 1.0e-5));
        assert!(roots.iter().any(|root| (*root - 2.0).abs() < 1.0e-5));

        model.plot("(x-2)^2", context()).unwrap();
        let RootResult::Roots(roots) = model.root_result() else { panic!("tangent root expected") };
        assert!(roots.iter().any(|root| (*root - 2.0).abs() < 1.0e-4));
    }

    #[test]
    fn asymptote_is_not_reported_as_a_root() {
        let mut model = GraphModel::default();
        model.plot("1/x", context()).unwrap();
        assert!(matches!(model.root_result(), RootResult::None));
    }
}
