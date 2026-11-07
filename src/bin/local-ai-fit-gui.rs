//! GUI: 내 PC에서 돌릴 수 있는 로컬 AI 모델을 창으로 보여준다. 판정 로직은 lib.rs(CLI와 같음).

#![windows_subsystem = "windows"] // 더블클릭 시 콘솔 창을 띄우지 않음

use eframe::egui::{self, Color32, RichText};
use local_ai_fit::*;
use std::sync::mpsc::{channel, Receiver};

struct App {
    models: Vec<Model>,
    source: String,
    detected: Sys,
    ram: f64,
    vram: f64,
    cat: &'static str,
    show_all: bool,
    updating: Option<Receiver<Option<Vec<Model>>>>,
    status: String,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        add_korean_font(&cc.egui_ctx);
        let (models, source) = load_models();
        let detected = detect();
        Self {
            ram: detected.ram,
            vram: detected.gpu.as_ref().map_or(0.0, |g| g.vram),
            models,
            source,
            detected,
            cat: CATEGORIES[0],
            show_all: false,
            updating: None,
            status: String::new(),
        }
    }

    /// 화면에서 고친 RAM·VRAM을 반영한 사양
    fn sys(&self) -> Sys {
        let same = self.detected.gpu.as_ref().filter(|g| (g.vram - self.vram).abs() < 0.05);
        Sys {
            cpu: self.detected.cpu.clone(),
            ram: self.ram,
            gpu: same.cloned().or_else(|| (self.vram > 0.0).then(|| Gpu { name: "직접 입력".into(), vram: self.vram, unified: false })),
        }
    }

    fn start_update(&mut self, ctx: &egui::Context) {
        let (tx, rx) = channel();
        let (models, ctx) = (self.models.clone(), ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(update::update(&models));
            ctx.request_repaint();
        });
        self.updating = Some(rx);
        self.status = "Ollama에서 최신 목록을 받는 중이에요… (15초쯤 걸려요)".into();
    }

    fn check_update(&mut self) {
        let Some(Ok(result)) = self.updating.as_ref().map(|rx| rx.try_recv()) else { return };
        self.updating = None;
        self.status = match result {
            None => "인터넷에 연결하지 못했어요. 목록은 그대로예요.".into(),
            Some(new) => {
                let added = new.len().saturating_sub(self.models.len());
                let saved = match saved_list_path().map(|p| save_models(&new, &p).map(|_| p)) {
                    Some(Ok(p)) => {
                        self.source = format!("{} (--update로 받은 목록)", p.display());
                        String::new()
                    }
                    Some(Err(e)) => format!(" 다만 파일 저장은 실패했어요({e})."),
                    None => " 다만 저장 위치를 찾지 못했어요.".into(),
                };
                self.models = new;
                format!("목록을 갱신했어요. 새 모델 {added}개, 모두 {}개예요.{saved}", self.models.len())
            }
        };
    }
}

/// egui 기본 글꼴엔 한글이 없어서 시스템 글꼴을 덧붙인다.
fn add_korean_font(ctx: &egui::Context) {
    let candidates = [
        r"C:\Windows\Fonts\malgun.ttf",
        "/System/Library/Fonts/AppleSDGothicNeo.ttc",
        "/usr/share/fonts/truetype/nanum/NanumGothic.ttf",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    ];
    let Some(data) = candidates.iter().find_map(|p| std::fs::read(p).ok()) else { return };
    ctx.add_font(egui::epaint::text::FontInsert {
        name: "korean".into(),
        data: egui::FontData::from_owned(data),
        families: [egui::FontFamily::Proportional, egui::FontFamily::Monospace]
            .into_iter()
            .map(|family| egui::epaint::text::InsertFontFamily { family, priority: egui::epaint::text::FontPriority::Lowest })
            .collect(),
    });
}

fn fit_color(f: Fit) -> Color32 {
    match f {
        Fit::Gpu => Color32::from_rgb(80, 190, 110),
        Fit::Split(_) => Color32::from_rgb(225, 170, 50),
        Fit::Cpu => Color32::from_rgb(100, 155, 230),
        Fit::No => Color32::from_rgb(200, 90, 90),
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.check_update();
        let sys = self.sys();

        egui::Panel::top("spec").show(ui, |ui| {
            ui.add_space(6.0);
            ui.heading("내 PC에서 돌릴 수 있는 로컬 AI 모델");
            ui.add_space(4.0);
            egui::Grid::new("spec_grid").num_columns(3).spacing([12.0, 6.0]).show(ui, |ui| {
                ui.label("CPU");
                ui.label(if sys.cpu.is_empty() { "(알 수 없음)" } else { &sys.cpu });
                ui.label("");
                ui.end_row();

                ui.label("RAM");
                ui.add(egui::DragValue::new(&mut self.ram).range(0.0..=2048.0).speed(0.5).fixed_decimals(1).suffix(" GB"));
                ui.weak(format!("모델에 쓸 수 있는 양은 약 {:.1} GB예요", usable_ram(&sys)));
                ui.end_row();

                ui.label("GPU VRAM");
                ui.add(egui::DragValue::new(&mut self.vram).range(0.0..=512.0).speed(0.5).fixed_decimals(1).suffix(" GB"));
                ui.weak(match &sys.gpu {
                    Some(g) => format!("{}, 모델에 쓸 수 있는 양은 약 {:.1} GB예요", g.name, usable_vram(g)),
                    None => "GPU가 없어서 CPU로만 실행해요".into(),
                });
                ui.end_row();
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("내 PC 사양으로 되돌리기").clicked() {
                    self.ram = self.detected.ram;
                    self.vram = self.detected.gpu.as_ref().map_or(0.0, |g| g.vram);
                }
                ui.checkbox(&mut self.show_all, "돌릴 수 없는 모델도 보기");
                ui.separator();
                let busy = self.updating.is_some();
                if ui.add_enabled(!busy, egui::Button::new("최신 목록 받기")).on_hover_text("Ollama에서 모델 크기를 새로 받고, 인기 있는 새 모델을 추가해요").clicked() {
                    self.start_update(ui.ctx());
                }
                if busy {
                    ui.spinner();
                }
                ui.label(&self.status);
            });
            ui.weak(format!("모델 목록: {}", self.source));
            ui.add_space(6.0);
        });

        egui::Panel::left("cats").exact_size(150.0).show(ui, |ui| {
            ui.add_space(6.0);
            for cat in CATEGORIES {
                let n = rows(&self.models, &sys, cat, false).len();
                let total = self.models.iter().filter(|m| m.cat == cat).count();
                ui.selectable_value(&mut self.cat, cat, format!("{cat}  {n}/{total}"));
            }
            ui.add_space(10.0);
            ui.weak("숫자는 돌릴 수 있는 모델 수 / 전체 모델 수예요.");
        });

        egui::CentralPanel::default_margins().show(ui, |ui| {
            let rows = rows(&self.models, &sys, self.cat, self.show_all);
            match best(&rows) {
                Some(m) => ui.label(RichText::new(format!("추천: {}  ({})", m.tag, m.note)).strong()),
                None => ui.label("이 사양에서 돌릴 수 있는 모델이 없어요."),
            };
            ui.weak("기준: 문맥 약 8K 토큰, Ollama 기본 양자화(대부분 Q4_K_M). 속도는 대략적인 체감 등급이에요.");
            ui.add_space(6.0);

            let best_tag = best(&rows).map(|m| m.tag.clone());
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                egui::Grid::new("models").striped(true).num_columns(7).spacing([16.0, 6.0]).show(ui, |ui| {
                    for h in ["모델", "실행 도구", "크기 GB", "필요 GB", "실행 방식", "속도", "메모"] {
                        ui.strong(h);
                    }
                    ui.end_row();
                    for (m, f) in &rows {
                        let tag = if best_tag.as_deref() == Some(m.tag.as_str()) { format!("★ {}", m.tag) } else { m.tag.clone() };
                        ui.label(RichText::new(tag).monospace());
                        ui.label(m.runtime());
                        ui.label(format!("{:.1}", m.size));
                        ui.label(format!("{:.1}", m.need()));
                        ui.label(RichText::new(f.label()).color(fit_color(*f)));
                        ui.label(speed(m, *f));
                        let mut note = m.note.clone();
                        if q8_fits(m, *f, &sys) {
                            note += " · Q8도 GPU에 들어가요";
                        }
                        ui.label(note);
                        ui.end_row();
                    }
                });
            });
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1150.0, 760.0]).with_title("local-ai-fit"),
        ..Default::default()
    };
    eframe::run_native("local-ai-fit", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
