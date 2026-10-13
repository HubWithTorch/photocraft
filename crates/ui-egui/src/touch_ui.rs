//! Finger-accessible shortcuts to existing editor commands and latched modifier keys.
use egui::{Button, RawInput, Ui, vec2};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{PhotocraftApp, canvas::ToolEvent, state::Tool};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TouchUi {
    pub enabled: bool,
    #[serde(skip_deserializing)]
    pub sample_armed: Option<Tool>,
    #[serde(skip_deserializing)]
    pub sample_active: Option<Tool>,
    #[serde(skip)]
    last_tool: Option<Tool>,
}

pub(crate) fn can_sample(tool: Tool) -> bool {
    matches!(tool, Tool::CloneStamp | Tool::Healing | Tool::Brush | Tool::Pencil | Tool::Gradient | Tool::PaintBucket)
}

pub(crate) fn clear(app: &mut PhotocraftApp) {
    app.ui.touch.sample_armed = None;
    app.ui.touch.sample_active = None;
    app.ui.shell.sticky_shift = false;
    app.ui.shell.sticky_alt = false;
    app.ui.shell.sticky_command = false;
}

fn sync_tool(app: &mut PhotocraftApp) {
    let tool = app.ui.tool;
    if app.ui.touch.last_tool.is_some_and(|previous| previous != tool) {
        app.ui.touch.sample_armed = None;
        app.ui.touch.sample_active = None;
        // A constraint armed for a crop must not become a straight brush stroke later.
        if app.ui.touch.enabled {
            app.ui.shell.sticky_shift = false;
            app.ui.shell.sticky_alt = false;
            app.ui.shell.sticky_command = false;
        }
    }
    app.ui.touch.last_tool = Some(tool);
    if app.ui.transform.is_some() {
        app.ui.touch.sample_armed = None;
        app.ui.touch.sample_active = None;
    }
}

pub(crate) fn raw_input(app: &mut PhotocraftApp, input: &RawInput) {
    if app.ui.touch.enabled && (!input.focused || input.events.iter().any(|e| matches!(e, egui::Event::WindowFocused(false)))) {
        clear(app);
    }
    // A fresh physical press recovers from a lost release. The canvas can issue another Down
    // for the *same* click on release; that must remain swallowed by sample_event below.
    if input.events.iter().any(|e| matches!(e, egui::Event::PointerButton { button: egui::PointerButton::Primary, pressed: true, .. })) {
        app.ui.touch.sample_active = None;
    }
}

/// Consume an entire sampling gesture. A missing Up cannot poison the next fresh press.
pub(crate) fn sample_event(app: &mut PhotocraftApp, event: ToolEvent) -> bool {
    if !app.ui.touch.enabled {
        app.ui.touch.sample_armed = None;
        app.ui.touch.sample_active = None;
        return false;
    }
    sync_tool(app);
    if app.ui.touch.sample_active.is_some() {
        if matches!(event, ToolEvent::Up { .. }) {
            app.ui.touch.sample_active = None;
        }
        return true;
    }
    if let ToolEvent::Down { x, y, .. } = event {
        if app.ui.touch.sample_armed != Some(app.ui.tool) {
            return false;
        }
        app.ui.touch.sample_active = Some(app.ui.tool);
        let inside = app
            .session
            .active()
            .is_some_and(|st| x.is_finite() && y.is_finite() && x >= 0.0 && y >= 0.0 && x < f64::from(st.doc.size.width) && y < f64::from(st.doc.size.height));
        if !inside {
            app.ui.status = tl!("Tap inside the image to sample").into();
            app.ui.status_error = false;
            return true;
        }
        app.ui.touch.sample_armed = None;
        app.drag = None;
        app.live_stroke = None;
        if matches!(app.ui.tool, Tool::CloneStamp | Tool::Healing) {
            crate::retouch_ui::set_source(app, x, y);
        } else if let Some([r, g, b]) = crate::canvas::eyedropper_color(app, x, y) {
            let _ = app.run("tools.setColors", json!({"foreground": [r, g, b, 1.0]}));
        }
        return true;
    }
    false
}

fn action(app: &mut PhotocraftApp, ui: &mut Ui, label: &str, id: &str) {
    if ui.add_enabled(crate::menus::is_enabled(app, id), Button::new(label).min_size(vec2(64.0, 44.0))).clicked()
        && let Err(e) = crate::menus::invoke(app, ui.ctx(), id, json!({}))
    {
        app.ui.status = e;
        app.ui.status_error = true;
    }
}

pub(crate) fn bar(app: &mut PhotocraftApp, ui: &mut Ui) {
    if !app.ui.touch.enabled {
        return;
    }
    sync_tool(app);
    let t = crate::theme::Tokens::get(ui.ctx());
    egui::Panel::bottom("touch-controls").exact_size(56.0).frame(egui::Frame::NONE.fill(t.chrome).inner_margin(4)).show(ui, |ui| {
        ui.spacing_mut().interact_size.y = 44.0;
        egui::ScrollArea::horizontal().id_salt("touch-controls-scroll").show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                let tool = app.ui.tool;
                if app.ui.view.hides_chrome() {
                    action(app, ui, tl!("Show Panels"), "view.screenMode.standard");
                }
                if can_sample(tool) && app.ui.transform.is_none() {
                    let armed = app.ui.touch.sample_armed == Some(tool);
                    let label = if armed {
                        tl!("Cancel Sampling")
                    } else if matches!(tool, Tool::CloneStamp | Tool::Healing) {
                        tl!("Set Sample Point")
                    } else {
                        tl!("Pick Color Once")
                    };
                    if ui
                        .add_enabled(app.session.active().is_some(), Button::new(label).selected(armed).min_size(vec2(112.0, 44.0)))
                        .on_hover_text(tl!("Tap this, then tap the image. Drawing resumes automatically."))
                        .clicked()
                    {
                        app.ui.touch.sample_armed = if armed { None } else { Some(tool) };
                        app.ui.touch.sample_active = None;
                    }
                }
                action(app, ui, tl!("Undo"), "edit.undo");
                action(app, ui, tl!("Redo"), "edit.redo");
                ui.menu_button(tl!("Touch Tools"), |ui| {
                    ui.spacing_mut().interact_size.y = 44.0;
                    egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                        for tool in Tool::ALL {
                            if ui.selectable_label(app.ui.tool == tool, tl!(tool.label())).clicked() {
                                app.ui.tool = tool;
                                app.ui.remember_group_tool();
                                sync_tool(app);
                                ui.close();
                            }
                        }
                    });
                });
                if tool.is_brushlike() {
                    ui.menu_button(tl!("Brush Size"), |ui| {
                        ui.set_min_width(260.0);
                        ui.spacing_mut().interact_size.y = 44.0;
                        let before = app.session.tools.brush.clone();
                        let mut brush = before.clone();
                        ui.add(egui::Slider::new(&mut brush.size, 1.0..=300.0).suffix(" px"));
                        ui.horizontal(|ui| {
                            for size in [5.0, 20.0, 50.0, 100.0] {
                                if ui.add(Button::new(format!("{size:.0}")).min_size(vec2(54.0, 44.0))).clicked() {
                                    brush.size = size;
                                }
                            }
                        });
                        crate::brush_panel::commit_gesture(app, ui.ctx(), &before, &brush);
                    });
                }
                let command_label = crate::shortcuts::pretty("Cmd");
                let shell = &mut app.ui.shell;
                for (label, on) in [("Shift", &mut shell.sticky_shift), ("Alt", &mut shell.sticky_alt), (&*command_label, &mut shell.sticky_command)] {
                    if ui.add(Button::new(label).selected(*on).min_size(vec2(64.0, 44.0))).clicked() {
                        *on = !*on;
                    }
                }
                if ui.add(Button::new(tl!("Clear Modifiers")).min_size(vec2(80.0, 44.0))).clicked() {
                    clear(app);
                }
                ui.menu_button(tl!("Selection"), |ui| {
                    ui.spacing_mut().interact_size.y = 44.0;
                    for (mode, label) in
                        [tl!("New selection"), tl!("Add to selection"), tl!("Subtract from selection"), tl!("Intersect with selection")].iter().enumerate()
                    {
                        if ui.selectable_label(usize::from(app.ui.selection_mode) == mode, *label).clicked() {
                            clear(app);
                            app.ui.selection_mode = mode as u8;
                            ui.close();
                        }
                    }
                    ui.separator();
                    action(app, ui, tl!("Deselect"), "select.deselect");
                });
                action(app, ui, tl!("Fit on Screen"), "view.fitOnScreen");
                if app.ui.transform.is_some() {
                    if ui.button(tl!("Commit transform")).clicked() {
                        crate::transform_tool::commit(app);
                    }
                    if ui.button(tl!("Cancel")).clicked() {
                        crate::transform_tool::cancel(app);
                    }
                } else if app.ui.tool == Tool::Crop && app.ui.crop_rect.is_some() {
                    if ui.button(tl!("Commit crop")).clicked() {
                        crate::canvas::commit_crop(app);
                    }
                    if ui.button(tl!("Cancel")).clicked() {
                        crate::crop_ui::cancel(app);
                    }
                } else if app.ui.tool == Tool::PolygonLasso && app.ui.polygon.len() >= 3 {
                    if ui.button(tl!("Close selection")).clicked() {
                        crate::canvas::commit_polygon(app);
                    }
                } else {
                    action(app, ui, tl!("Free Transform"), "edit.freeTransform");
                }
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::tool_event;

    fn image(tool: Tool) -> PhotocraftApp {
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), Default::default());
        app.run("file.new", json!({"width": 200, "height": 100, "background": "white"})).unwrap();
        for (x, colour) in [(0, "#ff0000"), (100, "#00ff00")] {
            app.run("select.rect", json!({"x": x, "y": 0, "width": 100, "height": 100})).unwrap();
            app.run("edit.fill", json!({"color": colour})).unwrap();
        }
        app.run("select.deselect", json!({})).unwrap();
        app.ui.tool = tool;
        app.ui.touch.enabled = true;
        app.run("tools.setBrush", json!({"brush": {"size": 10, "hardness": 1.0}})).unwrap();
        app
    }
    fn down(app: &mut PhotocraftApp, x: f64, y: f64) {
        tool_event(app, ToolEvent::Down { x, y, pressure: 1.0 }, egui::Modifiers::NONE);
    }
    fn up(app: &mut PhotocraftApp, x: f64, y: f64) {
        tool_event(app, ToolEvent::Up { x, y }, egui::Modifiers::NONE);
    }

    #[test]
    fn touch_source_is_one_gesture_then_clone_paints_without_alt() {
        for tool in [Tool::CloneStamp, Tool::Healing] {
            let mut app = image(tool);
            let revision = app.session.active().unwrap().revision;
            app.ui.touch.sample_armed = Some(tool);
            down(&mut app, 150.0, 50.0);
            tool_event(&mut app, ToolEvent::Move { x: 160.0, y: 50.0, pressure: 1.0 }, egui::Modifiers::NONE);
            up(&mut app, 160.0, 50.0);
            assert_eq!(app.ui.clone_source, Some([150.0, 50.0]));
            assert_eq!(app.session.active().unwrap().revision, revision);
            assert!(app.ui.touch.sample_armed.is_none() && app.ui.touch.sample_active.is_none());
            if tool == Tool::CloneStamp {
                down(&mut app, 50.0, 50.0);
                up(&mut app, 50.0, 50.0);
                let pixel = app.session.active().unwrap().doc.layers[0].surface().unwrap().rgba(50, 50);
                assert!(pixel[1] > 0.9 && pixel[0] < 0.1, "green cloned over red without Alt: {pixel:?}");
            }
        }
    }

    #[test]
    fn touch_colour_sampling_never_paints_and_next_press_draws_normally() {
        let mut app = image(Tool::Brush);
        let revision = app.session.active().unwrap().revision;
        app.ui.touch.sample_armed = Some(Tool::Brush);
        down(&mut app, 150.0, 50.0);
        tool_event(&mut app, ToolEvent::Move { x: 50.0, y: 50.0, pressure: 1.0 }, egui::Modifiers::NONE);
        up(&mut app, 50.0, 50.0);
        assert_eq!(app.session.tools.foreground, [0.0, 1.0, 0.0, 1.0]);
        assert_eq!(app.session.active().unwrap().revision, revision);
        down(&mut app, 50.0, 50.0);
        up(&mut app, 50.0, 50.0);
        let pixel = app.session.active().unwrap().doc.layers[0].surface().unwrap().rgba(50, 50);
        assert!(pixel[1] > 0.9 && pixel[0] < 0.1);
    }

    #[test]
    fn invalid_sample_keeps_the_picker_armed_and_focus_loss_clears_keys() {
        let mut app = image(Tool::CloneStamp);
        app.ui.touch.sample_armed = Some(Tool::CloneStamp);
        down(&mut app, f64::NAN, 50.0);
        up(&mut app, 0.0, 50.0);
        assert_eq!(app.ui.touch.sample_armed, Some(Tool::CloneStamp));
        assert!(app.ui.clone_source.is_none());
        app.ui.shell.sticky_alt = true;
        app.ui.shell.sticky_shift = true;
        raw_input(&mut app, &RawInput { focused: false, ..Default::default() });
        assert!(app.ui.touch.sample_armed.is_none());
        assert!(!app.ui.shell.sticky_alt && !app.ui.shell.sticky_shift);
    }

    #[test]
    fn a_fresh_physical_press_recovers_from_a_missing_sample_release() {
        let mut app = image(Tool::CloneStamp);
        app.ui.touch.sample_armed = Some(Tool::CloneStamp);
        down(&mut app, 150.0, 50.0);
        assert!(app.ui.touch.sample_active.is_some());
        raw_input(
            &mut app,
            &RawInput {
                events: vec![egui::Event::PointerButton {
                    pos: egui::Pos2::ZERO,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
        );
        down(&mut app, 50.0, 50.0);
        up(&mut app, 50.0, 50.0);
        let pixel = app.session.active().unwrap().doc.layers[0].surface().unwrap().rgba(50, 50);
        assert!(pixel[1] > 0.9 && pixel[0] < 0.1);
    }

    #[test]
    fn tool_change_and_restored_settings_cannot_leave_sampling_held() {
        let mut app = image(Tool::CloneStamp);
        sync_tool(&mut app);
        app.ui.touch.sample_armed = Some(Tool::CloneStamp);
        app.ui.shell.sticky_alt = true;
        app.ui.tool = Tool::Brush;
        sync_tool(&mut app);
        assert!(app.ui.touch.sample_armed.is_none() && !app.ui.shell.sticky_alt);
        app.ui.touch.sample_armed = Some(Tool::Brush);
        app.ui.touch.sample_active = Some(Tool::Brush);
        let restored: TouchUi = serde_json::from_value(serde_json::to_value(&app.ui.touch).unwrap()).unwrap();
        assert!(restored.sample_armed.is_none() && restored.sample_active.is_none());
    }

    #[test]
    fn real_canvas_click_after_touch_button_sets_source_without_a_keyboard() {
        use egui_kittest::{Harness, kittest::Queryable};
        let mut app = image(Tool::CloneStamp);
        app.sync_views();
        let mut h = Harness::builder().with_size(vec2(1100.0, 700.0)).build_ui_state(
            |ui, app: &mut PhotocraftApp| {
                if !ui.ctx().fonts(|f| f.families().contains(&egui::FontFamily::Name("medium".into()))) {
                    return;
                }
                bar(app, ui);
                egui::CentralPanel::default().show(ui, |ui| crate::canvas::document_area(app, ui));
            },
            app,
        );
        PhotocraftApp::setup_context(&h.ctx, crate::theme::ThemeKind::ALL[0]);
        h.run_steps(4);
        h.get_by_label(tl!("Set Sample Point")).click();
        h.run_steps(2);
        assert_eq!(h.state().ui.touch.sample_armed, Some(Tool::CloneStamp));
        let revision = h.state().session.active().unwrap().revision;
        let view = &h.state().ui.views[0];
        let xf = crate::canvas::ViewXform {
            rect: crate::rulers::content_rect(h.state(), h.state().last_canvas_rect),
            zoom: view.zoom,
            center: view.center,
            flip: false,
            rotation: 0.0,
            aspect: 1.0,
        };
        let p = xf.to_screen(150.0, 50.0);
        h.event(egui::Event::PointerMoved(p));
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE });
            h.run_steps(1);
        }
        assert_eq!(h.state().ui.clone_source, Some([150.0, 50.0]));
        assert!(h.state().drag.is_none());
        assert_eq!(h.state().session.active().unwrap().revision, revision, "a source tap must not paint a zero-offset clone stroke");
        assert!(h.state().ui.clone_offset.is_none());
    }

    #[test]
    fn touch_ctrl_button_selects_multiple_layers_without_a_keyboard() {
        use egui_kittest::{Harness, kittest::Queryable};
        let mut app = image(Tool::Brush);
        let first = app.session.active().unwrap().doc.layers[0].id;
        app.run("layer.new.layer", json!({"name": "Retouch"})).unwrap();
        app.sync_views();
        let mut h = Harness::builder().with_size(vec2(1300.0, 850.0)).build_ui_state(
            |ui, app: &mut PhotocraftApp| {
                if !ui.ctx().fonts(|f| f.families().contains(&egui::FontFamily::Name("medium".into()))) {
                    return;
                }
                bar(app, ui);
                crate::panels::right_dock(app, ui);
                egui::CentralPanel::default().show(ui, |ui| crate::canvas::document_area(app, ui));
            },
            app,
        );
        PhotocraftApp::setup_context(&h.ctx, crate::theme::ThemeKind::ALL[0]);
        h.run_steps(4);
        h.get_by_label(&crate::shortcuts::pretty("Cmd")).click();
        h.run_steps(2);
        let row = crate::layer_row_ui::recorded(&h.ctx).into_iter().find(|r| r.layer == first.0).unwrap();
        let p = row.name.unwrap().center();
        h.event(egui::Event::PointerMoved(p));
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE });
            h.run_steps(1);
        }
        assert_eq!(h.state().session.active().unwrap().selected_layers().len(), 2);
    }
}
