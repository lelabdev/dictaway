use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, DrawingArea};
use gtk4_layer_shell::LayerShell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const BAR_COUNT: usize = 9;
const BAR_WIDTH: f64 = 8.0;
const BAR_GAP: f64 = 5.0;
const MIN_BAR_HEIGHT: f64 = 6.0;
const MAX_BAR_HEIGHT: f64 = 48.0;
const WIN_WIDTH: i32 = ((BAR_WIDTH + BAR_GAP) * BAR_COUNT as f64) as i32 + 20;
const WIN_HEIGHT: i32 = 64;
const BAR_RADIUS: f64 = 4.0;

pub struct Overlay {
    volumes: Arc<Mutex<[f64; BAR_COUNT]>>,
    smooth: Arc<Mutex<[f64; BAR_COUNT]>>,
}

impl Overlay {
    pub fn new() -> Self {
        Self {
            volumes: Arc::new(Mutex::new([0.0; BAR_COUNT])),
            smooth: Arc::new(Mutex::new([0.0; BAR_COUNT])),
        }
    }

    pub fn update_volume(&self, vol: f32) {
        let mut v = self.volumes.lock().unwrap();
        for i in 0..BAR_COUNT - 1 {
            v[i] = v[i + 1];
        }
        v[BAR_COUNT - 1] = vol.clamp(0.0, 1.0) as f64;
    }

    pub fn show(&self, stop: &Arc<AtomicBool>) {
        let volumes = self.volumes.clone();
        let smooth = self.smooth.clone();
        let stop_flag = stop.clone();

        let app = Application::builder()
            .application_id("com.dictate.overlay")
            .build();

        app.connect_activate(move |app| {
            let window = ApplicationWindow::builder()
                .application(app)
                .default_width(WIN_WIDTH)
                .default_height(WIN_HEIGHT)
                .decorated(false)
                .build();

            window.add_css_class("dictate-overlay");

            window.init_layer_shell();
            window.set_layer(gtk4_layer_shell::Layer::Overlay);
            window.set_anchor(gtk4_layer_shell::Edge::Bottom, true);
            window.set_margin(gtk4_layer_shell::Edge::Bottom, 32);

            let drawing = DrawingArea::builder()
                .content_width(WIN_WIDTH)
                .content_height(WIN_HEIGHT)
                .hexpand(true)
                .vexpand(true)
                .build();

            let vols = volumes.clone();
            let sm = smooth.clone();
            let tick = Arc::new(Mutex::new(0u64));
            let tick_clone = tick.clone();
            drawing.set_draw_func(move |_area, cr, _w, h| {
                // Smooth interpolation
                {
                    let v = vols.lock().unwrap();
                    let mut s = sm.lock().unwrap();
                    for i in 0..BAR_COUNT {
                        let target = v[i];
                        s[i] += (target - s[i]) * 0.35;
                        // Decay faster when silent
                        if target < 0.05 {
                            s[i] *= 0.85;
                        }
                    }
                }

                let mut t = tick_clone.lock().unwrap();
                *t += 1;
                let now = *t;
                drop(t);

                let s = sm.lock().unwrap();
                let total_w = (BAR_WIDTH + BAR_GAP) * BAR_COUNT as f64;
                let start_x = (WIN_WIDTH as f64 - total_w) / 2.0;

                // Background pill
                let bg_x = start_x - 8.0;
                let bg_y = 4.0;
                let bg_w = total_w + 16.0;
                let bg_h = h as f64 - 8.0;
                cr.set_source_rgba(0.0, 0.0, 0.0, 0.55);
                rounded_rect(cr, bg_x, bg_y, bg_w, bg_h, 10.0);
                let _ = cr.fill();

                for i in 0..BAR_COUNT {
                    let vol = s[i].clamp(0.0, 1.0);
                    // Use power curve for more dynamic range — small sounds still visible
                    let visual = vol.powf(0.6);
                    let bar_h = MIN_BAR_HEIGHT + visual * (MAX_BAR_HEIGHT - MIN_BAR_HEIGHT);
                    let x = start_x + (i as f64) * (BAR_WIDTH + BAR_GAP);
                    let y = h as f64 - bar_h - 8.0;

                    // Color gradient: idle=warm amber, mid=gold, active=bright teal/cyan
                    let r = (1.0 - visual * 0.7).max(0.2);
                    let g = 0.55 + visual * 0.45;
                    let b = 0.15 + visual * 0.65;
                    let alpha = 0.65 + visual * 0.35;

                    // Glow behind bar
                    if visual > 0.08 {
                        cr.set_source_rgba(r, g, b, visual * 0.18);
                        rounded_rect(cr, x - 2.5, y - 2.5, BAR_WIDTH + 5.0, bar_h + 5.0, BAR_RADIUS + 2.5);
                        let _ = cr.fill();
                    }

                    // Main bar
                    cr.set_source_rgba(r, g, b, alpha);
                    rounded_rect(cr, x, y, BAR_WIDTH, bar_h, BAR_RADIUS);
                    let _ = cr.fill();
                }

                // REC dot indicator — pulsing
                let pulse = 0.5 + 0.5 * ((now as f64 * 0.07).sin());
                let dot_r = 3.0;
                let dot_x = start_x + total_w / 2.0;
                let dot_y = 10.0;
                cr.set_source_rgba(1.0, 0.55, 0.15, 0.45 + pulse * 0.55);
                cr.arc(dot_x, dot_y, dot_r, 0.0, std::f64::consts::PI * 2.0);
                let _ = cr.fill();
            });

            let drawing_clone = drawing.clone();
            let stop_c = stop_flag.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(30), move || {
                drawing_clone.queue_draw();
                if stop_c.load(Ordering::SeqCst) {
                    if let Some(native) = drawing_clone.native() {
                        if let Ok(win) = native.downcast::<ApplicationWindow>() {
                            win.close();
                        }
                    }
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });

            window.set_child(Some(&drawing));
            window.show();
        });

        app.run_with_args::<String>(&[]);
    }
}

fn rounded_rect(cr: &gtk4::cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let r = r.min(w / 2.0).min(h / 2.0);
    cr.move_to(x + r, y);
    cr.line_to(x + w - r, y);
    cr.arc(x + w - r, y + r, r, -std::f64::consts::FRAC_PI_2, 0.0);
    cr.line_to(x + w, y + h - r);
    cr.arc(x + w - r, y + h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
    cr.line_to(x + r, y + h);
    cr.arc(x + r, y + h - r, r, std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
    cr.line_to(x, y + r);
    cr.arc(x + r, y + r, r, std::f64::consts::PI, std::f64::consts::PI + std::f64::consts::FRAC_PI_2);
    cr.close_path();
}
