// ============================================================================
//  Patcher de Traduction FR Undertale Yellow & Red and Yellow
//  Licence: MIT
// ============================================================================

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use serde::{Deserialize, Serialize};

// Chargement des binaires xdelta3 à la compilation
const XDELTA_LINUX: &[u8] = include_bytes!("../bin/linux/xdelta3");
const XDELTA_WIN: &[u8] = include_bytes!("../bin/win/xdelta3.exe");

// Chargement des images à la compilation
const JACKET_UTY: &[u8] = include_bytes!("../assets/Undertale_Yellow.webp");
const JACKET_RY: &[u8] = include_bytes!("../assets/undertale-red-yellow.webp");
const BG_IMAGE_BYTES: &[u8] = include_bytes!("../assets/banniere_UTY.webp");
const DISCORD_ICON_BYTES: &[u8] = include_bytes!("../assets/discord.webp");
const APP_ICON_BYTES: &[u8] = include_bytes!("../assets/coeur.webp");

// URL du versions.json sur GitHub (source de vérité pour les mises à jour)
const VERSIONS_URL: &str = "https://raw.githubusercontent.com/redyellowpatchfr-a11y/patcher/main/versions.json";

// Liens de support
const DISCORD_URL: &str = "https://discord.gg/mAwZBxhSSf";

#[derive(Serialize, Deserialize, Clone, Debug)]
struct PatchMetadata {
    filename: String,
    sha256: String,
    size: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ProjectVersion {
    version: String,
    patch_url: String,
    repack_url: Option<String>,
    date: String,
    patch: PatchMetadata,
    changelog: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct VersionResponse {
    projects: std::collections::HashMap<String, ProjectVersion>,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum GameProject {
    UndertaleYellow,
    RedAndYellow,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum Step {
    MainSelection,
    ChooseInstallMethod, 
    DetectGame,          
    InstallRepack,       
    Patching,
    Success,
    Error,
}

struct AppState {
    current_step: Step,
    selected_project: Option<GameProject>,
    detected_path: Option<PathBuf>,
    manual_path: Option<PathBuf>,
    
    // Repack local sélectionné manuellement
    manual_repack_path: Option<PathBuf>,
    
    // Status variables
    status_message: String,
    error_message: String,
    progress: f32,
    download_speed: String,
    
    // Auto install Yellow + Patch choice
    auto_install_uty: bool,
    is_update_mode: bool,
    
    // Threads communication
    is_patching: bool,
    update_data: Option<VersionResponse>,

    // Options d'installation
    install_dir: PathBuf,
    create_shortcut: bool,

    // Infos pour le lancement du jeu
    final_game_dir: Option<PathBuf>,
    final_is_unx: bool,
}

impl Default for AppState {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let default_install = PathBuf::from(home).join("Games").join("UndertaleYellowFR");
        Self {
            current_step: Step::MainSelection,
            selected_project: None,
            detected_path: None,
            manual_path: None,
            manual_repack_path: None,
            status_message: String::new(),
            error_message: String::new(),
            progress: 0.0,
            download_speed: String::new(),
            auto_install_uty: false,
            is_update_mode: false,
            is_patching: false,
            update_data: None,
            install_dir: default_install,
            create_shortcut: true,
            final_game_dir: None,
            final_is_unx: false,
        }
    }
}

struct PatcherApp {
    state: Arc<Mutex<AppState>>,
    tex_uty: Option<egui::TextureHandle>,
    tex_ry: Option<egui::TextureHandle>,
    tex_bg: Option<egui::TextureHandle>,
    tex_discord: Option<egui::TextureHandle>,
    tex_heart: Option<egui::TextureHandle>,
}

impl PatcherApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_retro_style(&cc.egui_ctx);
        
        let state = Arc::new(Mutex::new(AppState::default()));
        
        // Charger les textures
        let tex_uty = load_image_bytes(&cc.egui_ctx, "jacket_uty", JACKET_UTY);
        let tex_ry = load_image_bytes(&cc.egui_ctx, "jacket_ry", JACKET_RY);
        let tex_bg = load_image_bytes(&cc.egui_ctx, "bg_image", BG_IMAGE_BYTES);
        let tex_discord = load_image_bytes(&cc.egui_ctx, "discord_icon", DISCORD_ICON_BYTES);
        let tex_heart = load_image_bytes(&cc.egui_ctx, "app_icon", APP_ICON_BYTES);

        // Vérification des mises à jour en arrière-plan
        let state_clone = Arc::clone(&state);
        let ctx_clone = cc.egui_ctx.clone();
        thread::spawn(move || {
            if let Ok(response) = minreq::get(VERSIONS_URL).send() {
                if response.status_code == 200 {
                    if let Ok(parsed) = serde_json::from_str::<VersionResponse>(response.as_str().unwrap_or_default()) {
                        let mut s = state_clone.lock().unwrap();
                        s.update_data = Some(parsed);
                        ctx_clone.request_repaint();
                    }
                }
            }
        });

        Self { state, tex_uty, tex_ry, tex_bg, tex_discord, tex_heart }
    }
}

fn setup_retro_style(ctx: &egui::Context) {
    ctx.set_pixels_per_point(1.0);

    let mut style = (*ctx.style()).clone();
    
    style.visuals.dark_mode = true;
    style.visuals.window_fill = egui::Color32::BLACK;
    style.visuals.panel_fill = egui::Color32::BLACK;
    
    style.visuals.window_rounding = 4.0.into();
    style.visuals.menu_rounding = 4.0.into();
    style.visuals.widgets.noninteractive.rounding = 4.0.into();
    style.visuals.widgets.inactive.rounding = 4.0.into();
    style.visuals.widgets.hovered.rounding = 4.0.into();
    style.visuals.widgets.active.rounding = 4.0.into();
    style.visuals.widgets.open.rounding = 4.0.into();
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    
    style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_black_alpha(200);
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(2.0_f32, egui::Color32::WHITE);
    
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_black_alpha(220);
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(2.0_f32, egui::Color32::WHITE);
    
    // Sélection Jaune sur survol
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_black_alpha(240);
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(255, 204, 0));
    
    // Sélection Rouge sur clic
    style.visuals.widgets.active.bg_fill = egui::Color32::from_black_alpha(255);
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(255, 51, 51));
    
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(255, 204, 0);
    style.visuals.selection.stroke = egui::Stroke::new(1.0_f32, egui::Color32::BLACK);

    ctx.set_style(style);

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "DeterminationMono".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/DeterminationMono.ttf")),
    );
    
    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap()
        .insert(0, "DeterminationMono".to_owned());
    fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap()
        .insert(0, "DeterminationMono".to_owned());
        
    ctx.set_fonts(fonts);
}

fn load_image_bytes(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Option<egui::TextureHandle> {
    if let Ok(img) = image::load_from_memory(bytes) {
        let size = [img.width() as _, img.height() as _];
        let image_buffer = img.to_rgba8();
        let pixels = image_buffer.as_flat_samples();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
        Some(ctx.load_texture(name, color_image, Default::default()))
    } else {
        None
    }
}

// Redimensionner l'icône de l'application Pop!_OS pour qu'elle soit carrée (64x64)
fn load_app_icon(bytes: &[u8]) -> Option<egui::IconData> {
    if let Ok(img) = image::load_from_memory(bytes) {
        let img = img.resize_exact(64, 64, image::imageops::FilterType::Nearest);
        let img = img.to_rgba8();
        Some(egui::IconData {
            rgba: img.into_raw(),
            width: 64,
            height: 64,
        })
    } else {
        None
    }
}

// Bouton personnalisé Undertale avec gestion de hover, couleur d'accent et curseur
fn undertale_btn(ui: &mut egui::Ui, text: &str, is_primary: bool, size: egui::Vec2) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    
    let is_hovered = response.hovered();
    let is_clicked = response.is_pointer_button_down_on();
    
    let bg_color = if is_clicked {
        egui::Color32::from_rgb(60, 20, 30)
    } else if is_hovered {
        if is_primary {
            egui::Color32::from_rgb(45, 34, 10)
        } else {
            egui::Color32::from_rgb(34, 25, 48)
        }
    } else {
        if is_primary {
            egui::Color32::from_rgb(26, 18, 8)
        } else {
            egui::Color32::from_rgb(20, 14, 28)
        }
    };
    
    let stroke_color = if is_clicked {
        egui::Color32::from_rgb(255, 60, 60)
    } else if is_hovered {
        if is_primary {
            egui::Color32::from_rgb(255, 204, 0)
        } else {
            egui::Color32::from_rgb(210, 190, 255)
        }
    } else {
        if is_primary {
            egui::Color32::from_rgb(180, 140, 0)
        } else {
            egui::Color32::from_rgb(65, 48, 90)
        }
    };
    
    let text_color = if is_hovered || is_primary {
        egui::Color32::from_rgb(255, 204, 0)
    } else {
        egui::Color32::WHITE
    };
    
    ui.painter().rect_filled(rect, 4.0, bg_color);
    ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(if is_hovered { 1.5_f32 } else { 1.0_f32 }, stroke_color));
    
    let font_id = egui::FontId::proportional(12.0);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        font_id,
        text_color,
    );
    
    response
}

// Carte d'option avec titre, description et feedback au survol
fn custom_option_card(
    ui: &mut egui::Ui,
    title: &str,
    desc: &str,
    is_primary: bool,
    width: f32,
    height: f32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
    
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    
    let is_hovered = response.hovered();
    let is_clicked = response.is_pointer_button_down_on();
    
    let bg_color = if is_clicked {
        egui::Color32::from_rgb(55, 20, 32)
    } else if is_hovered {
        if is_primary {
            egui::Color32::from_rgb(38, 28, 10)
        } else {
            egui::Color32::from_rgb(30, 22, 42)
        }
    } else {
        if is_primary {
            egui::Color32::from_rgb(22, 16, 8)
        } else {
            egui::Color32::from_rgb(18, 12, 26)
        }
    };
    
    let stroke_color = if is_clicked {
        egui::Color32::from_rgb(255, 60, 60)
    } else if is_hovered {
        if is_primary {
            egui::Color32::from_rgb(255, 204, 0)
        } else {
            egui::Color32::from_rgb(200, 180, 240)
        }
    } else {
        if is_primary {
            egui::Color32::from_rgb(160, 120, 0)
        } else {
            egui::Color32::from_rgb(55, 40, 75)
        }
    };
    
    let title_color = if is_hovered || is_primary {
        egui::Color32::from_rgb(255, 204, 0)
    } else {
        egui::Color32::WHITE
    };
    
    ui.painter().rect_filled(rect, 6.0, bg_color);
    ui.painter().rect_stroke(rect, 6.0, egui::Stroke::new(if is_hovered { 1.5_f32 } else { 1.0_f32 }, stroke_color));
    
    // Titre
    let title_font = egui::FontId::proportional(12.5);
    ui.painter().text(
        egui::pos2(rect.min.x + 18.0, rect.min.y + 15.0),
        egui::Align2::LEFT_CENTER,
        title,
        title_font,
        title_color,
    );
    
    // Description sous le titre
    let desc_font = egui::FontId::proportional(9.5);
    ui.painter().text(
        egui::pos2(rect.min.x + 18.0, rect.min.y + 35.0),
        egui::Align2::LEFT_CENTER,
        desc,
        desc_font,
        egui::Color32::from_rgb(170, 165, 185),
    );
    
    response
}

impl eframe::App for PatcherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut state = self.state.lock().unwrap();
        
        if state.is_patching {
            ctx.request_repaint_after(Duration::from_millis(50));
        }

        // --- 1. Barre de navigation inférieure fixe (Footer) ---
        let bottom_bar_frame = egui::Frame::none()
            .fill(egui::Color32::from_rgb(12, 8, 18))
            .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(32, 22, 46)))
            .inner_margin(egui::Margin::symmetric(24.0, 10.0));

        egui::TopBottomPanel::bottom("bottom_bar")
            .frame(bottom_bar_frame)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Zénith Patcher v1.0.0").size(11.0).color(egui::Color32::from_rgb(140, 135, 155)));
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (rect, response) = ui.allocate_exact_size(egui::vec2(125.0, 20.0), egui::Sense::click());
                        if response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        if response.clicked() {
                            let _ = webbrowser::open(DISCORD_URL);
                        }
                        
                        let is_hovered = response.hovered();
                        let link_color = if is_hovered {
                            egui::Color32::from_rgb(114, 137, 218)
                        } else {
                            egui::Color32::from_rgb(180, 175, 200)
                        };
                        
                        let mut icon_rect = rect;
                        icon_rect.set_width(14.0);
                        icon_rect.set_height(14.0);
                        let icon_pos = egui::pos2(rect.min.x, rect.center().y - 7.0);
                        
                        if let Some(discord_tex) = &self.tex_discord {
                            ui.painter().image(
                                discord_tex.id(),
                                egui::Rect::from_min_size(icon_pos, egui::vec2(14.0, 14.0)),
                                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                if is_hovered { egui::Color32::WHITE } else { egui::Color32::from_rgb(180, 175, 200) }
                            );
                        }
                        
                        let font_id = egui::FontId::proportional(11.0);
                        let text_pos = egui::pos2(rect.min.x + 20.0, rect.center().y);
                        ui.painter().text(
                            text_pos,
                            egui::Align2::LEFT_CENTER,
                            "Aide & Discord",
                            font_id,
                            link_color,
                        );
                        if is_hovered {
                            ui.painter().line_segment(
                                [egui::pos2(text_pos.x, text_pos.y + 6.0), egui::pos2(text_pos.x + 85.0, text_pos.y + 6.0)],
                                egui::Stroke::new(1.0_f32, link_color)
                            );
                        }
                    });
                });
            });

        // --- 2. Panel Central ---
        let panel_frame = egui::Frame::none().fill(egui::Color32::from_rgb(14, 9, 21));
        egui::CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
            if let Some(bg_tex) = &self.tex_bg {
                let rect = ui.max_rect();
                ui.painter().image(
                    bg_tex.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::from_white_alpha(20)
                );
            }

            egui::Frame::none()
                .inner_margin(egui::Margin::symmetric(28.0, 16.0))
                .show(ui, |ui| {
                
                // En-tête et Stepper de navigation (Ordre 1 -> 2 -> 3 -> 4)
                ui.horizontal(|ui| {
                    ui.horizontal(|ui| {
                        if let Some(heart_tex) = &self.tex_heart {
                            ui.image((heart_tex.id(), egui::vec2(18.0, 18.0)));
                            ui.add_space(2.0);
                        }
                        ui.vertical(|ui| {
                            ui.heading(egui::RichText::new("ZÉNITH PATCHER").size(19.0).strong().color(egui::Color32::WHITE));
                            ui.label(egui::RichText::new("Patch de traduction française • Undertale Yellow & Red and Yellow").size(10.0).color(egui::Color32::from_rgb(160, 150, 180)));
                        });
                    });
                    
                    // Aligner le stepper complètement à droite de la fenêtre
                    let available_space = ui.available_width();
                    let stepper_width = 290.0;
                    if available_space > stepper_width {
                        ui.add_space(available_space - stepper_width);
                    }
                    
                    draw_step_header(ui, state.current_step);
                });
                
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(14.0);

                match state.current_step {
                    Step::MainSelection => {
                        let avail_w = ui.available_width();
                        let avail_h = ui.available_height();
                        
                        let card_w = 175.0;
                        let card_h = 245.0;
                        let total_w = card_w * 2.0 + 30.0;
                        let margin_x = ((avail_w - total_w) / 2.0).max(0.0);
                        let top_margin = ((avail_h - (card_h + 60.0)) / 2.0).clamp(4.0, 30.0);

                        ui.add_space(top_margin);
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("Sélectionnez votre jeu :").size(15.0).strong().color(egui::Color32::from_rgb(255, 204, 0)));
                            ui.add_space(14.0);
                            
                            ui.horizontal(|ui| {
                                ui.add_space(margin_x);
                                
                                // Card 1 : Undertale Yellow
                                let (card_rect, card_resp) = ui.allocate_exact_size(egui::vec2(card_w, card_h), egui::Sense::click());
                                if card_resp.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                                if card_resp.clicked() {
                                    state.selected_project = Some(GameProject::UndertaleYellow);
                                    state.current_step = Step::ChooseInstallMethod;
                                }
                                
                                let is_h = card_resp.hovered();
                                let card_bg = if is_h { egui::Color32::from_rgb(34, 24, 46) } else { egui::Color32::from_rgb(20, 14, 28) };
                                let card_stroke = if is_h { egui::Color32::from_rgb(255, 204, 0) } else { egui::Color32::from_rgb(50, 36, 68) };
                                
                                ui.painter().rect_filled(card_rect, 6.0, card_bg);
                                ui.painter().rect_stroke(card_rect, 6.0, egui::Stroke::new(if is_h { 1.5_f32 } else { 1.0_f32 }, card_stroke));
                                
                                if let Some(tex) = &self.tex_uty {
                                    let img_rect = egui::Rect::from_min_size(
                                        egui::pos2(card_rect.min.x + 12.0, card_rect.min.y + 12.0),
                                        egui::vec2(151.0, 178.0)
                                    );
                                    ui.painter().image(
                                        tex.id(),
                                        img_rect,
                                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                        egui::Color32::WHITE
                                    );
                                }
                                
                                let font_id = egui::FontId::proportional(12.5);
                                let text_color = if is_h { egui::Color32::from_rgb(255, 204, 0) } else { egui::Color32::WHITE };
                                ui.painter().text(
                                    egui::pos2(card_rect.center().x, card_rect.min.y + 204.0),
                                    egui::Align2::CENTER_CENTER,
                                    "Undertale Yellow",
                                    font_id,
                                    text_color,
                                );
                                
                                let sub_font = egui::FontId::proportional(9.5);
                                ui.painter().text(
                                    egui::pos2(card_rect.center().x, card_rect.min.y + 224.0),
                                    egui::Align2::CENTER_CENTER,
                                    "Version FR v0.5.0",
                                    sub_font,
                                    egui::Color32::from_rgb(160, 155, 175),
                                );

                                ui.add_space(30.0);

                                // Card 2 : Red & Yellow
                                let (card_rect, card_resp) = ui.allocate_exact_size(egui::vec2(card_w, card_h), egui::Sense::click());
                                if card_resp.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                                if card_resp.clicked() {
                                    state.selected_project = Some(GameProject::RedAndYellow);
                                    state.current_step = Step::ChooseInstallMethod;
                                }
                                
                                let is_h = card_resp.hovered();
                                let card_bg = if is_h { egui::Color32::from_rgb(38, 20, 30) } else { egui::Color32::from_rgb(20, 14, 28) };
                                let card_stroke = if is_h { egui::Color32::from_rgb(255, 70, 70) } else { egui::Color32::from_rgb(50, 36, 68) };
                                
                                ui.painter().rect_filled(card_rect, 6.0, card_bg);
                                ui.painter().rect_stroke(card_rect, 6.0, egui::Stroke::new(if is_h { 1.5_f32 } else { 1.0_f32 }, card_stroke));
                                
                                if let Some(tex) = &self.tex_ry {
                                    let img_rect = egui::Rect::from_min_size(
                                        egui::pos2(card_rect.min.x + 12.0, card_rect.min.y + 12.0),
                                        egui::vec2(151.0, 178.0)
                                    );
                                    ui.painter().image(
                                        tex.id(),
                                        img_rect,
                                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                        egui::Color32::WHITE
                                    );
                                }
                                
                                let font_id = egui::FontId::proportional(12.5);
                                let text_color = if is_h { egui::Color32::from_rgb(255, 70, 70) } else { egui::Color32::WHITE };
                                ui.painter().text(
                                    egui::pos2(card_rect.center().x, card_rect.min.y + 204.0),
                                    egui::Align2::CENTER_CENTER,
                                    "Red & Yellow",
                                    font_id,
                                    text_color,
                                );
                                
                                let sub_font = egui::FontId::proportional(9.5);
                                ui.painter().text(
                                    egui::pos2(card_rect.center().x, card_rect.min.y + 224.0),
                                    egui::Align2::CENTER_CENTER,
                                    "Version FR v2.2.0",
                                    sub_font,
                                    egui::Color32::from_rgb(160, 155, 175),
                                );
                            });
                        });
                    }

                    Step::ChooseInstallMethod => {
                        let (project_name, version_str, game_tex) = match state.selected_project {
                            Some(GameProject::UndertaleYellow) => ("Undertale Yellow", "Traduction FR v0.5.0", self.tex_uty.as_ref()),
                            Some(GameProject::RedAndYellow) => ("Undertale Red & Yellow", "Traduction FR v2.2.0", self.tex_ry.as_ref()),
                            None => ("", "", None),
                        };

                        let avail_w = ui.available_width();
                        let avail_h = ui.available_height();
                        
                        let left_w = 145.0;
                        let right_w = 420.0;
                        let total_w = left_w + right_w + 24.0;
                        let margin_x = ((avail_w - total_w) / 2.0).max(0.0);
                        let top_margin = ((avail_h - 290.0) / 2.0).clamp(4.0, 24.0);

                        ui.add_space(top_margin);
                        ui.horizontal(|ui| {
                            ui.add_space(margin_x);
                            
                            // --- Colonne Gauche : Jaquette & Infos ---
                            let left_frame = egui::Frame::none()
                                .fill(egui::Color32::from_rgb(20, 14, 28))
                                .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(50, 36, 68)))
                                .rounding(6.0)
                                .inner_margin(egui::Margin::symmetric(10.0, 12.0));
                            
                            left_frame.show(ui, |ui| {
                                ui.set_width(left_w);
                                ui.vertical_centered(|ui| {
                                    if let Some(tex) = game_tex {
                                        ui.image((tex.id(), egui::vec2(125.0, 160.0)));
                                        ui.add_space(8.0);
                                    }
                                    ui.label(egui::RichText::new(project_name).size(13.0).strong().color(egui::Color32::WHITE));
                                    ui.label(egui::RichText::new(version_str).size(10.0).color(egui::Color32::from_rgb(255, 204, 0)));
                                });
                            });

                            ui.add_space(20.0);

                            // --- Colonne Droite : 3 Options d'installation & Navigation ---
                            ui.vertical(|ui| {
                                ui.set_width(right_w);
                                ui.label(egui::RichText::new("CHOIX DU MODE D'INSTALLATION").size(14.0).strong().color(egui::Color32::from_rgb(255, 204, 0)));
                                ui.add_space(2.0);
                                ui.label(egui::RichText::new("Sélectionnez l'option correspondant à votre situation :").size(10.5).color(egui::Color32::from_rgb(170, 165, 185)));
                                ui.add_space(10.0);

                                // Option 1 : Traduire un jeu existant
                                let opt1_resp = custom_option_card(
                                    ui,
                                    "1. Traduire un jeu déjà installé",
                                    "Détecte votre installation Steam ou locale et applique le patch français.",
                                    false,
                                    right_w,
                                    50.0,
                                );
                                if opt1_resp.clicked() {
                                    state.auto_install_uty = false;
                                    state.is_update_mode = false;
                                    state.current_step = Step::DetectGame;
                                    start_game_detection(&mut state);
                                }

                                ui.add_space(8.0);

                                // Option 2 : Mettre à jour la traduction
                                let opt2_resp = custom_option_card(
                                    ui,
                                    "2. Mettre à jour la traduction",
                                    "Vérifie votre version installée et applique le dernier correctif disponible.",
                                    false,
                                    right_w,
                                    50.0,
                                );
                                if opt2_resp.clicked() {
                                    state.auto_install_uty = false;
                                    state.is_update_mode = true;
                                    state.current_step = Step::DetectGame;
                                    start_game_detection(&mut state);
                                }

                                ui.add_space(8.0);

                                // Option 3 : Installation complète autonome
                                let opt3_resp = custom_option_card(
                                    ui,
                                    "3. Télécharger & installer le jeu complet",
                                    "Installe la version complète autonome déjà traduite en français (prêt à jouer).",
                                    true,
                                    right_w,
                                    50.0,
                                );
                                if opt3_resp.clicked() {
                                    state.auto_install_uty = true;
                                    state.is_update_mode = false;
                                    state.current_step = Step::InstallRepack;
                                }

                                ui.add_space(14.0);
                                if undertale_btn(ui, "< Choisir un autre jeu", false, egui::vec2(170.0, 32.0)).clicked() {
                                    state.current_step = Step::MainSelection;
                                    state.selected_project = None;
                                }
                            });
                        });
                    }
                    
                    Step::DetectGame => {
                        let is_update = state.is_update_mode;
                        let title = if is_update { "MISE À JOUR DU PATCH" } else { "APPLICATION DE LA TRADUCTION" };
                        
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new(title).size(15.0).strong().color(egui::Color32::from_rgb(255, 204, 0)));
                            ui.add_space(14.0);

                            if let Some(path) = state.manual_path.clone().or_else(|| state.detected_path.clone()) {
                                let label_text = if is_update { "Emplacement de votre jeu à mettre à jour :" } else { "Emplacement du jeu détecté :" };
                                ui.label(egui::RichText::new(label_text).size(11.5).color(egui::Color32::from_rgb(200, 200, 210)));
                                ui.add_space(6.0);
                                
                                let max_width = 540.0;
                                let available = ui.available_width();
                                let margin = ((available - max_width) / 2.0).max(0.0);
                                
                                ui.horizontal(|ui| {
                                    ui.add_space(margin);
                                    
                                    let path_box = egui::Frame::none()
                                        .fill(egui::Color32::from_rgb(20, 14, 28))
                                        .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(50, 38, 70)))
                                        .rounding(4.0)
                                        .inner_margin(egui::Margin::symmetric(12.0, 8.0));
                                    
                                    path_box.show(ui, |ui| {
                                        ui.add_sized([380.0, 20.0], egui::Label::new(
                                            egui::RichText::new(path.to_string_lossy().to_string()).color(egui::Color32::WHITE).size(11.0)
                                        ));
                                    });
                                    
                                    ui.add_space(6.0);
                                    if undertale_btn(ui, "Parcourir...", false, egui::vec2(130.0, 36.0)).clicked() {
                                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                            state.manual_path = Some(folder);
                                        }
                                    }
                                });
                                
                                ui.add_space(10.0);
                                ui.checkbox(&mut state.create_shortcut, "Créer un raccourci de jeu sur le Bureau");
                                ui.add_space(18.0);

                                ui.horizontal(|ui| {
                                    ui.add_space(margin);
                                    if undertale_btn(ui, "< Retour", false, egui::vec2(130.0, 38.0)).clicked() {
                                        state.current_step = Step::ChooseInstallMethod;
                                        state.manual_path = None;
                                        state.detected_path = None;
                                    }
                                    ui.add_space(30.0);
                                    let btn_label = if is_update { "Mettre à jour le patch >" } else { "Lancer la traduction >" };
                                    if undertale_btn(ui, btn_label, true, egui::vec2(360.0, 38.0)).clicked() {
                                        state.current_step = Step::Patching;
                                        start_patching_process(Arc::clone(&self.state));
                                    }
                                });
                            } else {
                                let (game_name, can_repack) = match state.selected_project {
                                    Some(GameProject::UndertaleYellow) => ("Undertale Yellow", true),
                                    Some(GameProject::RedAndYellow) => ("Undertale", true),
                                    None => ("Jeu", false),
                                };

                                ui.label(egui::RichText::new(format!("{} n'a pas été détecté automatiquement.", game_name)).size(13.0).color(egui::Color32::from_rgb(255, 180, 0)));
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new("Sélectionnez le dossier de votre jeu ou optez pour l'installation complète autonome.").size(10.5).color(egui::Color32::from_rgb(170, 165, 185)));
                                ui.add_space(18.0);
                                
                                let total_btn_w = if can_repack { 480.0 } else { 320.0 };
                                let avail = ui.available_width();
                                let margin = ((avail - total_btn_w) / 2.0).max(0.0);

                                ui.horizontal(|ui| {
                                    ui.add_space(margin);
                                    if undertale_btn(ui, "< Retour", false, egui::vec2(110.0, 36.0)).clicked() {
                                        state.current_step = Step::ChooseInstallMethod;
                                    }
                                    ui.add_space(12.0);
                                    if undertale_btn(ui, "Parcourir un dossier...", false, egui::vec2(170.0, 36.0)).clicked() {
                                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                            state.manual_path = Some(folder);
                                        }
                                    }
                                    if can_repack {
                                        ui.add_space(12.0);
                                        if undertale_btn(ui, "Installer le pack complet", true, egui::vec2(170.0, 36.0)).clicked() {
                                            state.auto_install_uty = true;
                                            state.is_update_mode = false;
                                            state.current_step = Step::InstallRepack;
                                        }
                                    }
                                });
                            }
                        });
                    }

                    Step::InstallRepack => {
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("INSTALLATION DU JEU COMPLET").size(15.0).strong().color(egui::Color32::from_rgb(255, 204, 0)));
                            ui.add_space(10.0);

                            ui.label("Le jeu configuré avec la traduction française sera installé sur votre machine.");
                            ui.add_space(14.0);

                            let max_width = 540.0;
                            let available = ui.available_width();
                            let margin = ((available - max_width) / 2.0).max(0.0);

                            ui.label(egui::RichText::new("Dossier d'installation :").size(12.0).color(egui::Color32::from_rgb(200, 200, 210)));
                            ui.add_space(4.0);
                            
                            ui.horizontal(|ui| {
                                ui.add_space(margin);
                                
                                let path_box = egui::Frame::none()
                                    .fill(egui::Color32::from_rgb(20, 14, 28))
                                    .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(50, 38, 70)))
                                    .rounding(4.0)
                                    .inner_margin(egui::Margin::symmetric(12.0, 8.0));
                                
                                path_box.show(ui, |ui| {
                                    ui.add_sized([380.0, 20.0], egui::Label::new(
                                        egui::RichText::new(state.install_dir.to_string_lossy().to_string()).color(egui::Color32::WHITE).size(11.0)
                                    ));
                                });
                                
                                ui.add_space(6.0);
                                if undertale_btn(ui, "Modifier...", false, egui::vec2(130.0, 36.0)).clicked() {
                                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                        state.install_dir = folder.join("UndertaleFR");
                                    }
                                }
                            });

                            ui.add_space(10.0);
                            ui.checkbox(&mut state.create_shortcut, "Créer un raccourci sur le Bureau");
                            ui.add_space(20.0);

                            ui.horizontal(|ui| {
                                ui.add_space(margin);
                                if undertale_btn(ui, "< Retour", false, egui::vec2(120.0, 38.0)).clicked() {
                                    state.current_step = Step::ChooseInstallMethod;
                                }
                                ui.add_space(10.0);
                                if undertale_btn(ui, "Archive ZIP locale", false, egui::vec2(170.0, 38.0)).clicked() {
                                    if let Some(file) = rfd::FileDialog::new()
                                        .add_filter("Archive ZIP", &["zip"])
                                        .pick_file() 
                                    {
                                        state.manual_repack_path = Some(file);
                                        state.current_step = Step::Patching;
                                        start_patching_process(Arc::clone(&self.state));
                                    }
                                }
                                ui.add_space(10.0);
                                if undertale_btn(ui, "Lancer l'installation >", true, egui::vec2(210.0, 38.0)).clicked() {
                                    state.current_step = Step::Patching;
                                    start_patching_process(Arc::clone(&self.state));
                                }
                            });
                        });
                    }
                    
                    Step::Patching => {
                        let title = if state.is_update_mode {
                            "MISE À JOUR DE LA TRADUCTION EN COURS..."
                        } else {
                            "INSTALLATION DE LA TRADUCTION EN COURS..."
                        };
                        
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new(title).size(15.0).strong().color(egui::Color32::WHITE));
                            ui.add_space(18.0);
                            
                            ui.label(egui::RichText::new(&state.status_message).size(13.0).strong().color(egui::Color32::from_rgb(255, 204, 0)));
                            ui.add_space(14.0);
                            
                            draw_custom_progress_bar(ui, state.progress);
                            ui.add_space(8.0);
                            
                            // Alignement parfaitement centré sous la barre
                            let percent_str = format!("{:.0}%", state.progress * 100.0);
                            let detail_str = if !state.download_speed.is_empty() {
                                format!(" • {}", state.download_speed)
                            } else {
                                String::new()
                            };
                            ui.label(egui::RichText::new(format!("{}{}", percent_str, detail_str)).size(13.0).strong().color(egui::Color32::WHITE));
                        });
                    }
                    
                    Step::Success => {
                        let title = if state.is_update_mode {
                            "TRADUCTION MISE À JOUR AVEC SUCCÈS"
                        } else {
                            "TRADUCTION INSTALLÉE AVEC SUCCÈS"
                        };
                        
                        ui.vertical_centered(|ui| {
                            ui.heading(egui::RichText::new(title).color(egui::Color32::from_rgb(255, 204, 0)).strong().size(17.0));
                            ui.add_space(10.0);
                            
                            ui.label("Votre jeu est prêt et entièrement configuré en français !");
                            ui.add_space(14.0);

                            if let Some(game_dir) = &state.final_game_dir {
                                ui.label(egui::RichText::new("Emplacement du jeu :").size(11.0).color(egui::Color32::from_rgb(160, 160, 170)));
                                ui.add_space(2.0);
                                ui.label(egui::RichText::new(game_dir.to_string_lossy().to_string()).color(egui::Color32::WHITE).size(11.0));
                                ui.add_space(18.0);
                            }

                            if let (Some(project), Some(game_dir)) = (state.selected_project, &state.final_game_dir) {
                                if undertale_btn(ui, "Lancer le jeu maintenant", true, egui::vec2(280.0, 42.0)).clicked() {
                                    launch_game(project, game_dir, state.final_is_unx);
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                                ui.add_space(14.0);
                            }
                            
                            ui.horizontal(|ui| {
                                let available = ui.available_width();
                                let margin = ((available - 320.0) / 2.0).max(0.0);
                                ui.add_space(margin);
                                
                                if undertale_btn(ui, "Accueil", false, egui::vec2(150.0, 36.0)).clicked() {
                                    state.current_step = Step::MainSelection;
                                    state.selected_project = None;
                                    state.detected_path = None;
                                    state.manual_path = None;
                                    state.progress = 0.0;
                                    state.auto_install_uty = false;
                                    state.is_update_mode = false;
                                }
                                ui.add_space(16.0);
                                if undertale_btn(ui, "Quitter", false, egui::vec2(150.0, 36.0)).clicked() {
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                            });
                        });
                    }
                    
                    Step::Error => {
                        ui.vertical_centered(|ui| {
                            ui.heading(egui::RichText::new("UNE ERREUR EST SURVENUE").color(egui::Color32::from_rgb(255, 60, 60)).strong().size(17.0));
                            ui.add_space(12.0);
                            
                            ui.label(&state.error_message);
                            ui.add_space(14.0);

                            if state.error_message.contains("repack") || state.error_message.contains("zip") || state.error_message.contains("Connexion") || state.error_message.contains("403") || state.error_message.contains("404") {
                                ui.label("Vous pouvez sélectionner manuellement l'archive contenant la traduction :");
                                if undertale_btn(ui, "Sélectionner l'archive locale", false, egui::vec2(260.0, 36.0)).clicked() {
                                    if let Some(file) = rfd::FileDialog::new()
                                        .add_filter("Archive Zip", &["zip"])
                                        .pick_file() 
                                    {
                                        state.manual_repack_path = Some(file);
                                        state.current_step = Step::Patching;
                                        start_patching_process(Arc::clone(&self.state));
                                    }
                                }
                                ui.add_space(14.0);
                            }
                            
                            ui.horizontal(|ui| {
                                let available = ui.available_width();
                                let margin = ((available - 320.0) / 2.0).max(0.0);
                                ui.add_space(margin);
                                
                                if undertale_btn(ui, "Réessayer", false, egui::vec2(150.0, 36.0)).clicked() {
                                    state.current_step = Step::DetectGame;
                                    state.progress = 0.0;
                                    state.error_message.clear();
                                }
                                ui.add_space(16.0);
                                if undertale_btn(ui, "Accueil", false, egui::vec2(150.0, 36.0)).clicked() {
                                    state.current_step = Step::MainSelection;
                                    state.selected_project = None;
                                    state.detected_path = None;
                                    state.manual_path = None;
                                    state.progress = 0.0;
                                    state.error_message.clear();
                                    state.auto_install_uty = false;
                                    state.is_update_mode = false;
                                }
                            });
                        });
                    }
                }
            });
        });
    }
}

fn draw_step_header(ui: &mut egui::Ui, current: Step) {
    let steps = [
        ("1. Jeu", matches!(current, Step::MainSelection)),
        ("2. Options", matches!(current, Step::ChooseInstallMethod | Step::DetectGame | Step::InstallRepack)),
        ("3. Installation", matches!(current, Step::Patching)),
        ("4. Terminé", matches!(current, Step::Success | Step::Error)),
    ];
    
    ui.horizontal(|ui| {
        for (i, (name, is_active)) in steps.iter().enumerate() {
            if *is_active {
                ui.label(egui::RichText::new(*name).size(11.0).strong().color(egui::Color32::from_rgb(255, 204, 0)));
            } else {
                ui.label(egui::RichText::new(*name).size(10.0).color(egui::Color32::from_rgb(130, 125, 145)));
            }
            if i < steps.len() - 1 {
                ui.label(egui::RichText::new(">").size(9.0).color(egui::Color32::from_rgb(80, 70, 100)));
            }
        }
    });
}

// Détection automatique du répertoire du jeu
fn start_game_detection(state: &mut AppState) {
    let game_project = state.selected_project.unwrap();
    let home = std::env::var("HOME").unwrap_or_default();
    
    // Chemins Steam standards (Linux + Windows)
    let steam_paths = vec![
        format!("{}/.local/share/Steam/steamapps/common", home),
        format!("{}/.steam/steam/steamapps/common", home),
        format!("{}/.steam/steamapps/common", home),
        format!("{}/.steam/debian-installation/steamapps/common", home),
        format!("{}/snap/steam/common/.local/share/Steam/steamapps/common", home),
        format!("{}/.var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/common", home),
        format!("{}/Bureau/bazar", home),
        format!("{}/Bureau", home),
        format!("{}/Games", home),
        "C:\\Program Files (x86)\\Steam\\steamapps\\common".to_string(),
        "C:\\Program Files\\Steam\\steamapps\\common".to_string(),
        "D:\\Steam\\steamapps\\common".to_string(),
        "E:\\Steam\\steamapps\\common".to_string(),
    ];

    match game_project {
        GameProject::UndertaleYellow => {
            let uty_local_paths = vec![
                format!("{}/UndertaleYellowFR", home),
                format!("{}/Documents/UndertaleYellow", home),
                format!("{}/Games/UndertaleYellow", home),
                format!("{}/Downloads/Undertale Yellow v1_1PatchFr213", home),
                format!("{}/Téléchargements/Undertale Yellow v1_1PatchFr213", home),
                format!("{}/Bureau/Undertale Yellow", home),
                format!("{}/Games/Undertale Yellow", home),
                "C:\\Games\\Undertale Yellow".to_string(),
                "C:\\Program Files\\Undertale Yellow".to_string(),
            ];
            for path in &uty_local_paths {
                let pb = PathBuf::from(path);
                if pb.exists() && (pb.join("data.win").exists() || pb.join("assets").join("game.unx").exists() || pb.join("Undertale Yellow.exe").exists()) {
                    state.detected_path = Some(pb);
                    return;
                }
            }
            state.detected_path = None;
        }
        GameProject::RedAndYellow => {
            let folder_names = vec![
                "Undertale", "undertale", "UNDERTALE",
                "undertale red and yellow", "Undertale Red and Yellow",
                "UndertaleFR", "Undertale_FR"
            ];
            for base in &steam_paths {
                for folder in &folder_names {
                    let path = PathBuf::from(base).join(folder);
                    let has_win = path.join("data.win").exists() || path.join("UNDERTALE.exe").exists();
                    let has_unx = path.join("assets").join("game.unx").exists() || path.join("runner").exists();
                    if path.exists() && (has_win || has_unx) {
                        state.detected_path = Some(path);
                        return;
                    }
                }
            }
            state.detected_path = None;
        }
    }
}

// Recherche d'un repack zip local
fn find_local_repack(project: GameProject, manual_repack: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(path) = manual_repack {
        if path.exists() {
            return Some(path);
        }
    }

    let home = std::env::var("HOME").unwrap_or_default();
    let possible_dirs = vec![
        PathBuf::from("."),
        PathBuf::from(&home).join("Downloads"),
        PathBuf::from(&home).join("Téléchargements"),
        PathBuf::from(&home).join("Bureau"),
        PathBuf::from(&home).join("Desktop"),
    ];

    let possible_filenames: Vec<&str> = match project {
        GameProject::UndertaleYellow => vec![
            "Undertale Yellow v1_1PatchFr.zip",
            "Undertale Yellow v1_1PatchFr213.zip",
            "undertale-yellow-repack.zip",
            "repack.zip",
            "undertale-yellow.zip",
            "UndertaleYellow.zip",
        ],
        GameProject::RedAndYellow => vec![
            "Undertale.Red.and.Yellow.v2.1.4.FR.Linux.zip",
            "Undertale.Red.and.Yellow.v2.1.4.FR.Windows.zip",
            "Undertale_Red_and_Yellow_FR.zip",
            "undertale-red-yellow.zip",
            "ry-assets.zip",
        ],
    };

    for dir in &possible_dirs {
        for filename in &possible_filenames {
            let path = dir.join(filename);
            if path.exists() {
                return Some(path);
            }
        }
    }
    None
}

// Récupère l'URL de téléchargement du repack depuis le versions.json GitHub
fn get_github_repack_url(project: GameProject) -> Result<String, String> {
    if let Ok(response) = minreq::get(VERSIONS_URL)
        .with_header("User-Agent", "zenith-patcher/1.0")
        .with_timeout(15)
        .send()
    {
        if response.status_code == 200 {
            if let Ok(body) = response.as_str() {
                let key = match project {
                    GameProject::UndertaleYellow => "uty-fr",
                    GameProject::RedAndYellow => "ry-fr",
                };

                if let Some(pos) = body.find(key) {
                    let after_key = &body[pos..];
                    let field = if cfg!(windows) && after_key.contains("\"repack_windows_url\":") {
                        "\"repack_windows_url\":"
                    } else {
                        "\"repack_url\":"
                    };

                    if let Some(rpos) = after_key.find(field) {
                        let rest = &after_key[rpos + field.len()..];
                        let rest = rest.trim_start();
                        if rest.starts_with('"') {
                            let inner = &rest[1..];
                            if let Some(end) = inner.find('"') {
                                return Ok(inner[..end].to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Liens directs fiables vers les releases GitHub
    match project {
        GameProject::UndertaleYellow => {
            Ok("https://github.com/redyellowpatchfr-a11y/patcher/releases/download/uty-fr-v0.5.0/Undertale.Yellow.v1_1PatchFr.zip".to_string())
        }
        GameProject::RedAndYellow => {
            if cfg!(windows) {
                Ok("https://github.com/redyellowpatchfr-a11y/patcher/releases/download/ry-fr-v2.1.4/Undertale.Red.and.Yellow.v2.1.4.FR.Windows.zip".to_string())
            } else {
                Ok("https://github.com/redyellowpatchfr-a11y/patcher/releases/download/ry-fr-v2.1.4/Undertale.Red.and.Yellow.v2.1.4.FR.Linux.zip".to_string())
            }
        }
    }
}


// Processus asynchrone de téléchargement et de patching
fn start_patching_process(state_mutex: Arc<Mutex<AppState>>) {
    thread::spawn(move || {
        let (project, game_path, auto_install, manual_repack, create_shortcut) = {
            let mut state = state_mutex.lock().unwrap();
            state.is_patching = true;
            state.progress = 0.1;
            state.status_message = "Recherche des ressources...".to_string();
            
            let project = state.selected_project.unwrap();
            let auto_install = state.auto_install_uty;
            let manual_repack = state.manual_repack_path.clone();
            
            let game_path = if auto_install {
                state.install_dir.clone()
            } else {
                state.manual_path.clone().or_else(|| state.detected_path.clone()).unwrap()
            };
            let create_shortcut = state.create_shortcut;
            
            (project, game_path, auto_install, manual_repack, create_shortcut)
        };

        let temp_dir = tempfile::tempdir().unwrap();
        
        if auto_install {
            {
                let mut state = state_mutex.lock().unwrap();
                state.status_message = "Recherche d'un pack d'installation...".to_string();
            }

            if let Some(local_zip) = find_local_repack(project, manual_repack) {
                let mut state = state_mutex.lock().unwrap();
                state.status_message = format!("Pack détecté : {}", local_zip.file_name().unwrap().to_string_lossy());
                state.progress = 0.5;
                thread::sleep(Duration::from_millis(800));

                state.status_message = "Extraction du pack...".to_string();
                state.progress = 0.8;
                
                fs::create_dir_all(&game_path).unwrap();
                if let Err(e) = extract_zip(&local_zip, &game_path) {
                    state.current_step = Step::Error;
                    state.error_message = format!("Erreur lors de l'extraction :\n{}", e);
                    state.is_patching = false;
                    return;
                }
                
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let runner = game_path.join("runner");
                    let run_sh = game_path.join("run.sh");
                    if let Ok(m) = fs::metadata(&runner) {
                        let mut p = m.permissions();
                        p.set_mode(0o755);
                        let _ = fs::set_permissions(&runner, p);
                    }
                    if let Ok(m) = fs::metadata(&run_sh) {
                        let mut p = m.permissions();
                        p.set_mode(0o755);
                        let _ = fs::set_permissions(&run_sh, p);
                    }
                }
                
                let is_unx = game_path.join("assets/game.unx").exists() || game_path.join("runner").exists();
                
                if create_shortcut {
                    let _ = try_create_shortcut(project, &game_path, is_unx);
                }
                
                state.final_game_dir = Some(game_path.clone());
                state.final_is_unx = is_unx;
                state.current_step = Step::Success;
                state.progress = 1.0;
                state.is_patching = false;
                return;
            }

            {
                let mut state = state_mutex.lock().unwrap();
                state.status_message = "Recherche des fichiers d'installation...".to_string();
            }

            let repack_url = match get_github_repack_url(project) {
                Ok(url) => url,
                Err(e) => {
                    let mut state = state_mutex.lock().unwrap();
                    state.current_step = Step::Error;
                    state.error_message = format!(
                        "{}\n\nVérifiez votre connexion internet ou importez l'archive ZIP localement.",
                        e
                    );
                    state.is_patching = false;
                    return;
                }
            };

            let repack_zip = temp_dir.path().join("repack.zip");

            {
                let mut state = state_mutex.lock().unwrap();
                state.status_message = "Téléchargement du pack d'installation...".to_string();
            }

            match download_file(&repack_url, &repack_zip, &state_mutex, 0.1, 0.75) {
                Ok(_) => {
                    let mut state = state_mutex.lock().unwrap();
                    state.status_message = "Extraction des fichiers du jeu...".to_string();
                    state.progress = 0.8;

                    fs::create_dir_all(&game_path).unwrap();
                    if let Err(e) = extract_zip(&repack_zip, &game_path) {
                        state.current_step = Step::Error;
                        state.error_message = format!("Erreur d'extraction :\n{}", e);
                        state.is_patching = false;
                        return;
                    }

                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let runner = game_path.join("runner");
                        let run_sh = game_path.join("run.sh");
                        if let Ok(m) = fs::metadata(&runner) {
                            let mut p = m.permissions();
                            p.set_mode(0o755);
                            let _ = fs::set_permissions(&runner, p);
                        }
                        if let Ok(m) = fs::metadata(&run_sh) {
                            let mut p = m.permissions();
                            p.set_mode(0o755);
                            let _ = fs::set_permissions(&run_sh, p);
                        }
                    }

                    // Appliquer ensuite le patch FR sur le data.win extrait si présent
                    state.status_message = "Finalisation de l'installation...".to_string();
                    state.progress = 0.9;
                    drop(state); // libérer le verrou avant l'opération longue

                    let mut final_path = game_path.clone();
                    if game_path.join("Undertale Yellow v1_1PatchFr213").exists() {
                        final_path = game_path.join("Undertale Yellow v1_1PatchFr213");
                    }

                    let is_unx = final_path.join("assets/game.unx").exists() || final_path.join("runner").exists();

                    if create_shortcut {
                        let _ = try_create_shortcut(project, &final_path, is_unx);
                    }

                    let mut state = state_mutex.lock().unwrap();
                    state.final_game_dir = Some(final_path);
                    state.final_is_unx = is_unx;
                    state.current_step = Step::Success;
                    state.progress = 1.0;
                    state.is_patching = false;
                }
                Err(e) => {
                    let mut state = state_mutex.lock().unwrap();
                    state.current_step = Step::Error;
                    state.error_message = format!("Échec du téléchargement depuis GitHub :\n{}", e);
                    state.is_patching = false;
                }
            }
            return;
        }

        // Localiser le fichier de données en premier
        let (original_file, is_unx) = {
            let unx = game_path.join("assets").join("game.unx");
            let win = game_path.join("data.win");
            if unx.exists() {
                (unx, true)
            } else if win.exists() {
                (win, false)
            } else {
                let mut state = state_mutex.lock().unwrap();
                state.current_step = Step::Error;
                state.error_message = "Fichier data.win ou game.unx introuvable dans le dossier d'origine.".to_string();
                state.is_patching = false;
                return;
            }
        };

        // --- Logique du Patcher Manuel avec Fichier Local ---
        let local_patch_paths = match project {
            GameProject::UndertaleYellow => vec![
                PathBuf::from("./patches/uty-fr-v0.5.0.xdelta"),
                PathBuf::from("./uty-fr-v0.5.0.xdelta"),
                PathBuf::from("uty-fr-v0.5.0.xdelta"),
            ],
            GameProject::RedAndYellow => {
                if is_unx {
                    vec![
                        PathBuf::from("./patches/ry-fr-linux-v2.1.4.xdelta"),
                        PathBuf::from("./ry-fr-linux-v2.1.4.xdelta"),
                        PathBuf::from("ry-fr-linux-v2.1.4.xdelta"),
                    ]
                } else {
                    vec![
                        PathBuf::from("./patches/ry-fr-v2.1.4.xdelta"),
                        PathBuf::from("./ry-fr-v2.1.4.xdelta"),
                        PathBuf::from("ry-fr-v2.1.4.xdelta"),
                        PathBuf::from("./patches/ry-fr-v2.2.0.xdelta"),
                    ]
                }
            }
        };

        let mut patch_path = temp_dir.path().join("patch.xdelta");
        let mut is_local_patch = false;

        for path in local_patch_paths {
            if path.exists() {
                patch_path = path;
                is_local_patch = true;
                break;
            }
        }

        if !is_local_patch {
            let patch_url = match project {
                GameProject::UndertaleYellow => "https://github.com/redyellowpatchfr-a11y/patcher/releases/download/uty-fr-v0.5.0/uty-fr-v0.5.0.xdelta".to_string(),
                GameProject::RedAndYellow => {
                    if is_unx {
                        "https://github.com/redyellowpatchfr-a11y/patcher/releases/download/ry-fr-v2.2.0/ry-fr-linux-v2.1.4.xdelta".to_string()
                    } else {
                        "https://github.com/redyellowpatchfr-a11y/patcher/releases/download/ry-fr-v2.2.0/ry-fr-v2.2.0.xdelta".to_string()
                    }
                }
            };

            {
                let mut state = state_mutex.lock().unwrap();
                state.status_message = "Téléchargement du patch de traduction...".to_string();
                state.progress = 0.2;
            }

            match download_file(&patch_url, &patch_path, &state_mutex, 0.2, 0.7) {
                Ok(_) => {}
                Err(e) => {
                    let mut state = state_mutex.lock().unwrap();
                    state.current_step = Step::Error;
                    state.error_message = format!(
                        "Échec de connexion ({}).\nImpossible de télécharger le patch xdelta depuis GitHub.\n\nOption de repli : Placez le fichier de patch dans le dossier 'patches' pour l'appliquer hors-ligne.", 
                        e
                    );
                    state.is_patching = false;
                    return;
                }
            }
        } else {
            let mut state = state_mutex.lock().unwrap();
            state.status_message = "Patch xdelta local détecté et chargé.".to_string();
            state.progress = 0.4;
            thread::sleep(Duration::from_millis(600));
        }

        // Extraction xdelta3
        let xdelta_bin_path = temp_dir.path().join(if cfg!(windows) { "xdelta3.exe" } else { "xdelta3" });
        let xdelta_bytes = if cfg!(windows) { XDELTA_WIN } else { XDELTA_LINUX };

        if let Err(e) = fs::write(&xdelta_bin_path, xdelta_bytes) {
            let mut state = state_mutex.lock().unwrap();
            state.current_step = Step::Error;
            state.error_message = format!("Échec de l'extraction de xdelta3 :\n{}", e);
            state.is_patching = false;
            return;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(&xdelta_bin_path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                let _ = fs::set_permissions(&xdelta_bin_path, perms);
            }
        }

        // Sauvegarde
        let backup_file = original_file.with_extension(if is_unx { "unx.backup" } else { "win.backup" });
        if !backup_file.exists() {
            if let Err(e) = fs::copy(&original_file, &backup_file) {
                let mut state = state_mutex.lock().unwrap();
                state.current_step = Step::Error;
                state.error_message = format!("Échec de la sauvegarde de sécurité :\n{}", e);
                state.is_patching = false;
                return;
            }
        }

        let source_file = if backup_file.exists() {
            &backup_file
        } else {
            &original_file
        };

        let temp_patched_file = temp_dir.path().join("patched.win");
        let mut cmd = Command::new(&xdelta_bin_path);
        cmd.args(&[
            "-d",
            "-f",
            "-s",
            source_file.to_str().unwrap(),
            patch_path.to_str().unwrap(),
            temp_patched_file.to_str().unwrap()
        ]);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // Empêche le clignotement de la console cmd
        }
        let output = cmd.output();

        match output {
            Ok(out) if out.status.success() => {
                if let Err(e) = fs::copy(&temp_patched_file, &original_file) {
                    let mut state = state_mutex.lock().unwrap();
                    state.current_step = Step::Error;
                    state.error_message = format!("Échec du remplacement du fichier :\n{}", e);
                    state.is_patching = false;
                    return;
                }
                
                if project == GameProject::RedAndYellow && is_unx {
                    let runner_path = game_path.join("runner");
                    if !runner_path.exists() {
                        {
                            let mut state = state_mutex.lock().unwrap();
                            state.status_message = "Finalisation du lanceur Linux...".to_string();
                        }
                        let runner_url = "https://github.com/redyellowpatchfr-a11y/patcher/releases/download/ry-fr-v2.2.0/runner";
                        let _ = download_file(runner_url, &runner_path, &state_mutex, 0.92, 0.98);
                    }
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        if let Ok(metadata) = fs::metadata(&runner_path) {
                            let mut perms = metadata.permissions();
                            perms.set_mode(0o755);
                            let _ = fs::set_permissions(&runner_path, perms);
                        }
                    }
                }

                if project == GameProject::RedAndYellow && !is_unx {
                    {
                        let mut state = state_mutex.lock().unwrap();
                        state.status_message = "Téléchargement des musiques du mod (124 Mo)...".to_string();
                        state.progress = 0.85;
                    }
                    let assets_url = "https://github.com/redyellowpatchfr-a11y/patcher/releases/download/ry-fr-v2.2.0/ry-assets.zip";
                    let assets_zip = temp_dir.path().join("ry-assets.zip");
                    match download_file(assets_url, &assets_zip, &state_mutex, 0.85, 0.96) {
                        Ok(_) => {
                            {
                                let mut state = state_mutex.lock().unwrap();
                                state.status_message = "Extraction des musiques...".to_string();
                                state.progress = 0.97;
                            }
                            if let Err(e) = extract_zip(&assets_zip, &game_path) {
                                let mut state = state_mutex.lock().unwrap();
                                state.current_step = Step::Error;
                                state.error_message = format!("Échec de l'extraction des musiques du mod :\n{}", e);
                                state.is_patching = false;
                                return;
                            }
                        }
                        Err(e) => {
                            let mut state = state_mutex.lock().unwrap();
                            state.current_step = Step::Error;
                            state.error_message = format!("Échec du téléchargement des musiques du mod :\n{}", e);
                            state.is_patching = false;
                            return;
                        }
                    }
                }

                if create_shortcut {
                    let _ = try_create_shortcut(project, &game_path, is_unx);
                }

                let mut state = state_mutex.lock().unwrap();
                state.final_game_dir = Some(game_path.clone());
                state.final_is_unx = is_unx;
                state.current_step = Step::Success;
                state.progress = 1.0;
                state.is_patching = false;
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                let mut state = state_mutex.lock().unwrap();
                state.current_step = Step::Error;
                state.error_message = format!("Erreur xdelta3 :\n{}\nAssurez-vous que les fichiers du jeu original sont intacts.", stderr);
                state.is_patching = false;
            }
            Err(e) => {
                let mut state = state_mutex.lock().unwrap();
                state.current_step = Step::Error;
                state.error_message = format!("Erreur de lancement de xdelta3 :\n{}", e);
                state.is_patching = false;
            }
        }
    });
}

fn extract_zip(zip_path: &Path, dest_path: &Path) -> Result<(), String> {
    let status = if cfg!(target_os = "windows") {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            let mut cmd = Command::new("powershell");
            cmd.creation_flags(0x08000000); // Empêche l'ouverture du terminal cmd
            cmd.args(&[
                "-Command",
                &format!("Expand-Archive -Path '{}' -DestinationPath '{}' -Force", 
                    zip_path.to_str().unwrap(), 
                    dest_path.to_str().unwrap()
                )
            ]);
            cmd.status()
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok(std::process::Command::new("true").status().unwrap())
        }
    } else {
        Command::new("unzip")
            .args(&[
                "-o",
                zip_path.to_str().unwrap(),
                "-d",
                dest_path.to_str().unwrap()
            ])
            .status()
    };

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => Err("La commande d'extraction a retourné une erreur".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

// Fonction de téléchargement HTTP générique avec barre de progression fluide
fn download_file(url: &str, dest: &Path, state_mutex: &Arc<Mutex<AppState>>, start_pct: f32, end_pct: f32) -> Result<(), String> {
    let response = minreq::get(url)
        .with_header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .with_timeout(300)
        .send_lazy()
        .map_err(|e| e.to_string())?;
    
    if response.status_code != 200 {
        return Err(format!("Erreur HTTP {}", response.status_code));
    }
    
    // Récupération de la taille totale depuis les headers
    let total_size = response
        .headers
        .get("content-length")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    
    let mut file = File::create(dest).map_err(|e| e.to_string())?;
    
    let mut response_reader = response;
    let mut buffer = [0u8; 65536]; // Morceaux de 64 Ko
    let mut written = 0;
    
    loop {
        let n = response_reader.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
        written += n;
        
        let mut state = state_mutex.lock().unwrap();
        if total_size > 0 {
            state.progress = start_pct + (written as f32 / total_size as f32) * (end_pct - start_pct);
            state.download_speed = format!("{:.1} Mo / {:.1} Mo", written as f32 / 1024.0 / 1024.0, total_size as f32 / 1024.0 / 1024.0);
        } else {
            state.progress = start_pct + 0.1; // fallback if content-length is missing
            state.download_speed = format!("{:.1} Mo téléchargés", written as f32 / 1024.0 / 1024.0);
        }
        // sleep minimal pour laisser l'UI fluide
        thread::sleep(Duration::from_micros(10));
    }
    
    Ok(())
}

fn try_create_shortcut(project: GameProject, game_dir: &Path, is_unx: bool) -> Result<(), String> {
    #[cfg(unix)]
    {
        let home = std::env::var("HOME").unwrap_or_default();
        if home.is_empty() {
            return Err("Variable HOME vide".to_string());
        }
        let mut desktop_path = PathBuf::from(&home).join("Desktop");
        if !desktop_path.exists() {
            desktop_path = PathBuf::from(&home).join("Bureau");
        }
        if !desktop_path.exists() {
            return Err("Dossier Bureau ou Desktop introuvable".to_string());
        }
        
        let shortcut_path = match project {
            GameProject::UndertaleYellow => desktop_path.join("undertale-yellow-fr.desktop"),
            GameProject::RedAndYellow => desktop_path.join("undertale-red-yellow-fr.desktop"),
        };
        
        let name = match project {
            GameProject::UndertaleYellow => "Undertale Yellow FR",
            GameProject::RedAndYellow => "Undertale Red & Yellow FR",
        };
        
        let exec_cmd = match project {
            GameProject::UndertaleYellow => {
                format!("wine \"{}\"", game_dir.join("Undertale Yellow.exe").to_string_lossy())
            }
            GameProject::RedAndYellow => {
                if is_unx {
                    format!("\"{}\"", game_dir.join("runner").to_string_lossy())
                } else {
                    format!("wine \"{}\"", game_dir.join("UNDERTALE.exe").to_string_lossy())
                }
            }
        };
        
        let icon_path = game_dir.join("coeur_icon.png");
        let _ = fs::write(&icon_path, APP_ICON_BYTES);
        
        let shortcut_content = format!(
            "[Desktop Entry]\n\
            Name={}\n\
            Exec={}\n\
            Path={}\n\
            Icon={}\n\
            Terminal=false\n\
            Type=Application\n\
            Categories=Game;\n",
            name,
            exec_cmd,
            game_dir.to_string_lossy(),
            icon_path.to_string_lossy()
        );
        
        fs::write(&shortcut_path, shortcut_content).map_err(|e| e.to_string())?;
        
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(&shortcut_path) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&shortcut_path, perms);
        }
    }
    Ok(())
}

fn launch_game(project: GameProject, game_dir: &Path, is_unx: bool) {
    let _ = match project {
        GameProject::UndertaleYellow => {
            if cfg!(windows) {
                Command::new(game_dir.join("Undertale Yellow.exe"))
                    .current_dir(game_dir)
                    .spawn()
            } else {
                Command::new("wine")
                    .arg(game_dir.join("Undertale Yellow.exe"))
                    .current_dir(game_dir)
                    .spawn()
            }
        }
        GameProject::RedAndYellow => {
            if cfg!(windows) {
                if game_dir.join("UNDERTALE.exe").exists() {
                    Command::new(game_dir.join("UNDERTALE.exe"))
                        .current_dir(game_dir)
                        .spawn()
                } else {
                    Command::new(game_dir.join("runner.exe"))
                        .current_dir(game_dir)
                        .spawn()
                }
            } else {
                // Sur Linux : priorité aux fichiers natifs (runner / run.sh)
                let runner = game_dir.join("runner");
                let run_sh = game_dir.join("run.sh");
                
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(m) = fs::metadata(&runner) {
                        let mut p = m.permissions();
                        p.set_mode(0o755);
                        let _ = fs::set_permissions(&runner, p);
                    }
                    if let Ok(m) = fs::metadata(&run_sh) {
                        let mut p = m.permissions();
                        p.set_mode(0o755);
                        let _ = fs::set_permissions(&run_sh, p);
                    }
                }
                
                if is_unx && Command::new("xdg-open").arg("steam://run/391540").spawn().is_ok() {
                    Ok(std::process::Command::new("true").spawn().unwrap())
                } else if run_sh.exists() {
                    Command::new("sh")
                        .arg(&run_sh)
                        .current_dir(game_dir)
                        .spawn()
                } else if runner.exists() {
                    Command::new(&runner)
                        .current_dir(game_dir)
                        .spawn()
                } else if game_dir.join("UNDERTALE.exe").exists() {
                    Command::new("wine")
                        .arg(game_dir.join("UNDERTALE.exe"))
                        .current_dir(game_dir)
                        .spawn()
                } else {
                    Ok(std::process::Command::new("true").spawn().unwrap())
                }
            }
        }
    };
}

fn draw_custom_progress_bar(ui: &mut egui::Ui, progress: f32) {
    let width = 360.0;
    let height = 22.0;
    
    ui.horizontal(|ui| {
        let available_width = ui.available_width();
        let spacing = (available_width - width) / 2.0;
        if spacing > 0.0 {
            ui.add_space(spacing);
        }
        
        let (rect, _response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        
        // Dessiner la bordure blanche
        ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(2.0_f32, egui::Color32::WHITE));
        
        // Rectangle interne
        let inner_rect = rect.shrink(3.0);
        
        // Fond rouge sombre
        ui.painter().rect_filled(inner_rect, 0.0, egui::Color32::from_rgb(180, 0, 0));
        
        // Remplissage jaune de progression
        let progress = progress.clamp(0.0, 1.0);
        if progress > 0.0 {
            let mut progress_rect = inner_rect;
            progress_rect.set_width(inner_rect.width() * progress);
            ui.painter().rect_filled(progress_rect, 0.0, egui::Color32::from_rgb(255, 204, 0));
        }
    });
}

#[cfg(target_os = "linux")]
fn ensure_linux_desktop_entry() {
    if let Ok(home) = std::env::var("HOME") {
        let app_dir = PathBuf::from(&home).join(".local/share/applications");
        let icon_dir = PathBuf::from(&home).join(".local/share/icons/hicolor/128x128/apps");
        let pixmaps_dir = PathBuf::from(&home).join(".local/share/pixmaps");
        
        let _ = fs::create_dir_all(&app_dir);
        let _ = fs::create_dir_all(&icon_dir);
        let _ = fs::create_dir_all(&pixmaps_dir);
        
        let icon_path = icon_dir.join("zenith-patcher.png");
        let _ = fs::write(&icon_path, APP_ICON_BYTES);
        let _ = fs::write(pixmaps_dir.join("zenith-patcher.png"), APP_ICON_BYTES);
        
        if let Ok(exe_path) = std::env::current_exe() {
            let desktop_path = app_dir.join("zenith-patcher.desktop");
            let desktop_content = format!(
                "[Desktop Entry]\n\
                Type=Application\n\
                Name=Zenith Patcher\n\
                GenericName=Patcher de Traduction FR\n\
                Comment=Patcher de traduction pour Undertale Yellow et Red and Yellow\n\
                Exec=\"{}\"\n\
                Icon={}\n\
                Terminal=false\n\
                StartupWMClass=zenith-patcher\n\
                Categories=Utility;Game;\n",
                exe_path.to_string_lossy(),
                icon_path.to_string_lossy()
            );
            let _ = fs::write(desktop_path, desktop_content);
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn ensure_linux_desktop_entry() {}

fn run_app(renderer: eframe::Renderer, icon: Option<egui::IconData>) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([760.0, 500.0])
            .with_resizable(true)
            .with_maximize_button(true)
            .with_decorations(true)
            .with_icon(icon.unwrap_or_default())
            .with_title("Zenith Patcher"),
        renderer,
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
            supported_backends: eframe::wgpu::Backends::all(),
            power_preference: eframe::wgpu::PowerPreference::None,
            device_descriptor: std::sync::Arc::new(|_adapter| {
                eframe::wgpu::DeviceDescriptor {
                    label: Some("zenith_patcher_wgpu"),
                    required_features: eframe::wgpu::Features::empty(),
                    required_limits: eframe::wgpu::Limits::downlevel_defaults(),
                }
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    
    eframe::run_native(
        "zenith-patcher",
        options,
        Box::new(|cc| Ok(Box::new(PatcherApp::new(cc)))),
    )
}

fn main() -> eframe::Result<()> {
    #[cfg(target_os = "windows")]
    {
        // Enregistrer les panics dans un fichier log
        std::panic::set_hook(Box::new(|info| {
            let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
                *s
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                s.as_str()
            } else {
                "Unknown panic"
            };
            let location = if let Some(loc) = info.location() {
                format!("at {}:{}", loc.file(), loc.line())
            } else {
                "unknown location".to_string()
            };
            let error_msg = format!("PANIC: {} {}\n", msg, location);
            if let Ok(mut exe_dir) = std::env::current_exe() {
                exe_dir.pop();
                let _ = std::fs::write(exe_dir.join("zenith_patcher_panic.txt"), error_msg);
            } else {
                let _ = std::fs::write("zenith_patcher_panic.txt", error_msg);
            }
        }));
    }

    // S'assurer de la présence du lanceur desktop et de l'icône système sous Linux
    ensure_linux_desktop_entry();

    // Icône de l'application (coeur rouge d'Undertale) redimensionnée carré 64x64
    let icon = load_app_icon(APP_ICON_BYTES);

    #[cfg(target_os = "windows")]
    let final_result = run_app(eframe::Renderer::Wgpu, icon);

    #[cfg(not(target_os = "windows"))]
    let final_result = run_app(eframe::Renderer::Glow, icon);

    if let Err(ref e) = final_result {
        if let Ok(mut exe_dir) = std::env::current_exe() {
            exe_dir.pop();
            let _ = std::fs::write(exe_dir.join("zenith_patcher_error.txt"), format!("EFRAME ERROR: {:?}", e));
        } else {
            let _ = std::fs::write("zenith_patcher_error.txt", format!("EFRAME ERROR: {:?}", e));
        }
    }

    final_result
}


