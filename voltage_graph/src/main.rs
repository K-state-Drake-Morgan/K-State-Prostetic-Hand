use eframe::egui::{self, CentralPanel, Id, SidePanel, Visuals};
use egui_plotter::EguiBackend;
use plotters::prelude::*;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    env_logger::init();

    let native_options = eframe::NativeOptions::default();

    eframe::run_native(
        "VisualGraph Example",
        native_options,
        Box::new(|cc| Ok(Box::new(VisualGraph::new(cc)))),
    )
    .expect("Unable to use eframe");

    Ok(())
}

enum Theme {
    Light,
    Dark,
}

struct VisualGraph {
    theme: Theme,
    voltage_data: Vec<(f32, f32)>,
    frequency: f32,
    amplitude: f32,
    phase: f32,
}

impl VisualGraph {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let context = &cc.egui_ctx;
        context.set_visuals(Visuals::dark());

        let frequency = 1.0;
        let amplitude = 1.0;
        let phase = 0.0;

        let voltage_data = Self::generate_waveform(frequency, amplitude, phase);

        Self {
            theme: Theme::Dark,
            voltage_data,
            frequency,
            amplitude,
            phase,
        }
    }

    fn generate_waveform(freq: f32, amp: f32, phase: f32) -> Vec<(f32, f32)> {
        let samples = 200;
        let mut data = Vec::with_capacity(samples);
        for i in 0..samples {
            let x = i as f32 / samples as f32 * std::f32::consts::TAU;
            let y = amp * (freq * x + phase).sin();
            data.push((x, y));
        }
        data
    }
}

impl eframe::App for VisualGraph {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        SidePanel::new(egui::panel::Side::Left, Id::new("Graph sources"))
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("EMG Waveform Controls");

                ui.add(egui::Slider::new(&mut self.frequency, 0.5..=10.0).text("Frequency"));
                ui.add(egui::Slider::new(&mut self.amplitude, 0.0..=5.0).text("Amplitude"));
                ui.add(
                    egui::Slider::new(&mut self.phase, 0.0..=std::f32::consts::TAU).text("Phase"),
                );

                if ui.button("Regenerate Waveform").clicked() {
                    self.voltage_data =
                        Self::generate_waveform(self.frequency, self.amplitude, self.phase);
                }
            });
        CentralPanel::default().show(ctx, |ui| {
            let root = EguiBackend::new(ui).into_drawing_area();
            root.fill(&WHITE).unwrap();
            let mut chart = ChartBuilder::on(&root)
                .caption("y=x^2", ("sans-serif", 50).into_font())
                .margin(5)
                .x_label_area_size(30)
                .y_label_area_size(30)
                .build_cartesian_2d(-1f32..1f32, -0.1f32..1f32)
                .unwrap();

            chart.configure_mesh().draw().unwrap();

            chart
                .draw_series(LineSeries::new(self.voltage_data.clone(), &RED))
                .unwrap()
                .label("Simulated EMG Voltage")
                .legend(|(x, y)| PathElement::new(vec![(x, y), (x, y)], &RED));

            chart
                .configure_series_labels()
                .background_style(&WHITE.mix(0.8))
                .border_style(&BLACK)
                .draw()
                .unwrap();

            root.present().unwrap();
        });
    }
}
