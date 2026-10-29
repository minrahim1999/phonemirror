//! Shared vector icons, independent of application state.
use super::super::theme::PILL;
use egui::{vec2, Color32, Rect, Stroke};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app::ui) enum Icon {
    Phone,
    Wifi,
    Plus,
    Filter,
    Sidebar,
    Search,
    Keyboard,
    Split,
    ZoomOut,
    ZoomFit,
    ZoomActual,
    ZoomIn,
    Expand,
    Collapse,
    More,
    Sliders,
    Document,
    Info,
}

pub(in crate::app::ui) fn paint_icon(p: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    let c = rect.center();
    let s = Stroke::new(1.5, color);
    let line = |a: egui::Vec2, b: egui::Vec2| {
        p.line_segment([c + a, c + b], s);
    };
    let outline = |size: egui::Vec2, radius: f32| {
        p.rect_stroke(
            Rect::from_center_size(c, size),
            radius,
            s,
            egui::StrokeKind::Inside,
        );
    };
    let magnifier = || {
        p.circle_stroke(c + vec2(-1.5, -1.5), 6.0, s);
        p.line_segment([c + vec2(2.9, 2.9), c + vec2(7.0, 7.0)], s);
    };
    match icon {
        Icon::Phone => {
            outline(vec2(11.0, 18.0), 2.5);
            line(vec2(-1.5, -6.0), vec2(1.5, -6.0));
        }
        Icon::Wifi => {
            for radius in [6.0_f32, 11.0] {
                let points: Vec<_> = (0..17)
                    .map(|i| {
                        let angle = std::f32::consts::PI * (1.25 + i as f32 / 32.0);
                        c + vec2(angle.cos(), angle.sin()) * radius
                    })
                    .collect();
                p.add(egui::Shape::line(points, s));
            }
            p.circle_filled(c + vec2(0.0, 3.0), 1.8, color);
        }
        Icon::Plus => {
            line(vec2(-6.5, 0.0), vec2(6.5, 0.0));
            line(vec2(0.0, -6.5), vec2(0.0, 6.5));
        }
        Icon::Filter => {
            line(vec2(-7.0, -4.5), vec2(7.0, -4.5));
            line(vec2(-4.5, 0.0), vec2(4.5, 0.0));
            line(vec2(-2.0, 4.5), vec2(2.0, 4.5));
        }
        Icon::Sidebar => {
            outline(vec2(18.0, 14.0), 3.0);
            line(vec2(-3.0, -6.5), vec2(-3.0, 6.5));
        }
        Icon::Search => {
            p.circle_stroke(c + vec2(-1.0, -1.0), 4.5, s);
            line(vec2(2.3, 2.3), vec2(5.5, 5.5));
        }
        Icon::Keyboard => {
            outline(vec2(20.0, 13.0), 2.5);
            for y in [-2.8, 0.2] {
                for x in [-6.0, -2.0, 2.0, 6.0] {
                    p.circle_filled(c + vec2(x, y), 0.95, color);
                }
            }
            line(vec2(-4.0, 3.6), vec2(4.0, 3.6));
        }
        Icon::Split => {
            let frame = Rect::from_center_size(c, vec2(19.0, 14.0));
            p.rect_stroke(frame, 2.5, s, egui::StrokeKind::Inside);
            p.rect_filled(
                Rect::from_min_size(frame.right_top() + vec2(-8.5, 3.0), vec2(5.5, 4.5)),
                1.0,
                color,
            );
        }
        Icon::ZoomOut => {
            magnifier();
            line(vec2(-4.5, -1.5), vec2(1.5, -1.5));
        }
        Icon::ZoomIn => {
            magnifier();
            line(vec2(-4.5, -1.5), vec2(1.5, -1.5));
            line(vec2(-1.5, -4.5), vec2(-1.5, 1.5));
        }
        Icon::ZoomFit => {
            magnifier();
            p.rect_stroke(
                Rect::from_center_size(c + vec2(-1.5, -1.5), vec2(5.0, 5.0)),
                1.0,
                Stroke::new(1.2, color),
                egui::StrokeKind::Inside,
            );
        }
        Icon::ZoomActual => {
            magnifier();
            p.text(
                c + vec2(-1.5, -1.3),
                egui::Align2::CENTER_CENTER,
                "1",
                egui::FontId::proportional(8.5),
                color,
            );
        }
        Icon::Expand | Icon::Collapse => {
            for (sx, sy) in [(1.0, -1.0), (-1.0, 1.0)] {
                let outer = vec2(7.0 * sx, 7.0 * sy);
                let inner = vec2(1.5 * sx, 1.5 * sy);
                line(inner, outer);
                let (tip, dir) = if icon == Icon::Expand {
                    (outer, -1.0)
                } else {
                    (inner, 1.0)
                };
                line(tip, tip + vec2(4.0 * sx * dir, 0.0));
                line(tip, tip + vec2(0.0, 4.0 * sy * dir));
            }
        }
        Icon::More => {
            for x in [-6.0, 0.0, 6.0] {
                p.circle_filled(c + vec2(x, 0.0), 1.7, color);
            }
        }
        Icon::Sliders => {
            for (y, knob) in [(-5.0, -3.5), (0.0, 3.5), (5.0, -1.0)] {
                line(vec2(-8.0, y), vec2(8.0, y));
                p.circle_filled(c + vec2(knob, y), 2.6, PILL);
                p.circle_stroke(c + vec2(knob, y), 2.6, s);
            }
        }
        Icon::Document => {
            outline(vec2(14.0, 18.0), 2.5);
            for (y, half) in [(-3.5, 3.5), (0.0, 3.5), (3.5, 2.0)] {
                line(vec2(-3.5, y), vec2(-3.5 + half * 2.0, y));
            }
        }
        Icon::Info => {
            p.circle_filled(c + vec2(0.0, -6.0), 1.3, color);
            line(vec2(0.0, -2.0), vec2(0.0, 6.5));
            line(vec2(-1.8, -2.0), vec2(0.0, -2.0));
        }
    }
}
