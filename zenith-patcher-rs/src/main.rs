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

        Self { state, tex_uty, tex_ry, tex_bg, tex_discord, tex_heart }
    }
}

fn setup_retro_style(ctx: &egui::Context) {
    ctx.set_pixels_per_point(1.35);

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
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(2.0, egui::Color32::WHITE);
    
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_black_alpha(220);
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(2.0, egui::Color32::WHITE);
    
    // Sélection Jaune sur survol
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_black_alpha(240);
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 204, 0));
    
    // Sélection Rouge sur clic
    style.visuals.widgets.active.bg_fill = egui::Color32::from_black_alpha(255);
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 51, 51));
    
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(255, 204, 0);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::BLACK);

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

impl eframe::App for PatcherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut state = self.state.lock().unwrap();
        
        if state.is_patching {
            ctx.request_repaint_after(Duration::from_millis(50));
        }

        // --- 1. Barre de titre personnalisée (Frameless Window) ---
        let title_bar_frame = egui::Frame::none()
            .fill(egui::Color32::from_rgb(10, 5, 15))
            .inner_margin(egui::Margin::symmetric(15.0, 10.0));
            
        egui::TopBottomPanel::top("title_bar")
            .frame(title_bar_frame)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Cœur rouge d'Undertale
                    if let Some(heart_tex) = &self.tex_heart {
                        ui.image((heart_tex.id(), egui::vec2(14.0, 14.0)));
                    } else {
                        ui.label(egui::RichText::new("❤").color(egui::Color32::from_rgb(255, 51, 51)).size(14.0).strong());
                    }
                    ui.label(egui::RichText::new("ZÉNITH PATCHER").color(egui::Color32::from_rgb(255, 204, 0)).strong().size(12.0));
                    
                    let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                    
                    // Zone de drag de la fenêtre (alloue tout l'espace disponible moins 90px pour les boutons)
                    let drag_width = (ui.available_width() - 90.0).max(10.0);
                    let drag_space = egui::vec2(drag_width, ui.available_height());
                    let (_rect, response) = ui.allocate_at_least(drag_space, egui::Sense::drag());
                    if response.dragged() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }

                    // Boutons de contrôle (Fermer, Maximiser/Restaurer, Réduire) à droite
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_close = ui.add(egui::Button::new(
                            egui::RichText::new("✕").size(11.0).color(egui::Color32::WHITE)
                        ).frame(false));
                        if btn_close.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }

                        ui.add_space(8.0);

                        let btn_max = ui.add(egui::Button::new(
                            egui::RichText::new(if is_maximized { "🗗" } else { "🗖" }).size(11.0).color(egui::Color32::WHITE)
                        ).frame(false));
                        if btn_max.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                        }

                        ui.add_space(8.0);

                        let btn_min = ui.add(egui::Button::new(
                            egui::RichText::new("—").size(11.0).color(egui::Color32::WHITE)
                        ).frame(false));
                        if btn_min.clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                    });
                });
            });

        // --- 2. Panel Central ---
        let panel_frame = egui::Frame::none().fill(egui::Color32::from_rgb(20, 12, 28));
        egui::CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
            if let Some(bg_tex) = &self.tex_bg {
                let rect = ui.max_rect();
                ui.painter().image(
                    bg_tex.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::from_white_alpha(30) // Transparence d'arrière-plan
                );
            }

            egui::Frame::none()
                .inner_margin(egui::Margin::symmetric(30.0, 10.0))
                .show(ui, |ui| {
                    ui.add_space(5.0);
                
                // Titre principal
                ui.vertical_centered(|ui| {
                    ui.heading(egui::RichText::new("ZENITH PATCHER").size(24.0).strong().color(egui::Color32::WHITE));
                    ui.label(egui::RichText::new("Traduction française officielle du projet Zénith").size(11.0).color(egui::Color32::from_rgb(180, 180, 180)));
                });
                ui.add_space(15.0);

                match state.current_step {
                    Step::MainSelection => {
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("Sélectionnez le jeu à traduire :").size(15.0).strong().color(egui::Color32::from_rgb(255, 204, 0)));
                            ui.add_space(15.0);
                            
                            ui.columns(2, |columns| {
                                // Colonne 1 : Undertale Yellow
                                let card_frame = egui::Frame::none()
                                    .fill(egui::Color32::from_rgb(32, 20, 42))
                                    .stroke(egui::Stroke::new(1.5, egui::Color32::from_rgb(70, 50, 90)))
                                    .rounding(6.0)
                                    .inner_margin(12.0);

                                card_frame.show(&mut columns[0], |ui| {
                                    ui.vertical_centered(|ui| {
                                        if let Some(tex) = &self.tex_uty {
                                            let img_click = ui.add(egui::Image::new(
                                                egui::load::SizedTexture::new(tex.id(), [140.0, 186.0])
                                            ).sense(egui::Sense::click()));
                                            
                                            if img_click.clicked() {
                                                state.selected_project = Some(GameProject::UndertaleYellow);
                                                state.current_step = Step::ChooseInstallMethod;
                                            }
                                        }
                                        ui.add_space(8.0);
                                        let btn_text = ui.add_sized([180.0, 36.0], egui::Button::new(
                                            egui::RichText::new("Undertale Yellow").size(13.0).strong().color(egui::Color32::from_rgb(255, 204, 0))
                                        ));
                                        if btn_text.clicked() {
                                            state.selected_project = Some(GameProject::UndertaleYellow);
                                            state.current_step = Step::ChooseInstallMethod;
                                        }
                                        ui.add_space(4.0);
                                        ui.label(egui::RichText::new("Traduction v0.5.0").size(10.0).color(egui::Color32::from_rgb(180, 180, 180)));
                                    });
                                });
 
                                // Colonne 2 : Red & Yellow
                                card_frame.show(&mut columns[1], |ui| {
                                    ui.vertical_centered(|ui| {
                                        if let Some(tex) = &self.tex_ry {
                                            let img_click = ui.add(egui::Image::new(
                                                egui::load::SizedTexture::new(tex.id(), [140.0, 186.0])
                                            ).sense(egui::Sense::click()));
                                            
                                            if img_click.clicked() {
                                                state.selected_project = Some(GameProject::RedAndYellow);
                                                state.current_step = Step::ChooseInstallMethod;
                                            }
                                        }
                                        ui.add_space(8.0);
                                        let btn_text = ui.add_sized([180.0, 36.0], egui::Button::new(
                                            egui::RichText::new("Red & Yellow").size(13.0).strong().color(egui::Color32::from_rgb(255, 51, 51))
                                        ));
                                        if btn_text.clicked() {
                                            state.selected_project = Some(GameProject::RedAndYellow);
                                            state.current_step = Step::ChooseInstallMethod;
                                        }
                                        ui.add_space(4.0);
                                        ui.label(egui::RichText::new("Traduction v2.2.0").size(10.0).color(egui::Color32::from_rgb(180, 180, 180)));
                                    });
                                });
                            });
                        });
                    }

                    Step::ChooseInstallMethod => {
                        let project_name = match state.selected_project {
                            Some(GameProject::UndertaleYellow) => "Undertale Yellow",
                            Some(GameProject::RedAndYellow) => "Undertale Red & Yellow",
                            None => "",
                        };

                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new(format!("Jeu sélectionné : {}", project_name)).size(16.0).strong().color(egui::Color32::WHITE));
                            ui.add_space(15.0);

                            ui.label(egui::RichText::new("Choisissez votre mode d'installation :").size(13.0).color(egui::Color32::from_rgb(180, 180, 180)));
                            ui.add_space(15.0);

                            // Mode 1 : Patcher existant
                            let btn_a = ui.add_sized([420.0, 52.0], egui::Button::new(
                                egui::RichText::new("Traduire un jeu existant").size(15.0).strong().color(egui::Color32::WHITE)
                            ));
                            ui.add_space(3.0);
                            ui.label(egui::RichText::new("Détecte ou sélectionne le dossier de votre jeu déjà installé pour y appliquer le patch.").size(10.0).color(egui::Color32::GRAY));
                            if btn_a.clicked() {
                                state.auto_install_uty = false;
                                state.current_step = Step::DetectGame;
                                start_game_detection(&mut state);
                            }

                            ui.add_space(15.0);

                            // Mode 2 : Installation complète (Yellow seulement)
                            if state.selected_project == Some(GameProject::UndertaleYellow) {
                                let btn_b = ui.add_sized([420.0, 52.0], egui::Button::new(
                                    egui::RichText::new("Télécharger et installer le jeu complet").size(15.0).strong().color(egui::Color32::from_rgb(255, 204, 0))
                                ));
                                ui.add_space(3.0);
                                ui.label(egui::RichText::new("Télécharge le jeu complet configuré avec la traduction française (230 Mo).").size(10.0).color(egui::Color32::GRAY));
                                if btn_b.clicked() {
                                    state.auto_install_uty = true;
                                    state.current_step = Step::InstallRepack;
                                }
                            } else {
                                ui.label(egui::RichText::new("(Le téléchargement autonome du jeu complet n'est pas disponible pour Red & Yellow)").size(10.0).color(egui::Color32::DARK_GRAY));
                            }

                            ui.add_space(20.0);
                            if ui.add_sized([120.0, 32.0], egui::Button::new("Retour")).clicked() {
                                state.current_step = Step::MainSelection;
                                state.selected_project = None;
                            }
                        });
                    }
                    
                    Step::DetectGame => {
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("APPLICATION DE LA TRADUCTION").size(16.0).strong().color(egui::Color32::from_rgb(255, 204, 0)));
                            ui.add_space(15.0);

                            if let Some(path) = state.manual_path.clone().or_else(|| state.detected_path.clone()) {
                                let group_frame = egui::Frame::none()
                                    .fill(egui::Color32::from_rgb(32, 20, 42))
                                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 50, 90)))
                                    .rounding(4.0)
                                    .inner_margin(12.0);

                                group_frame.show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Dossier détecté :").strong().size(13.0).color(egui::Color32::WHITE));
                                    });
                                    ui.add_space(2.0);
                                    ui.label(egui::RichText::new(path.to_string_lossy().to_string()).color(egui::Color32::from_rgb(0, 180, 255)).size(11.0));
                                    ui.add_space(8.0);
                                    ui.checkbox(&mut state.create_shortcut, "Créer un raccourci de jeu sur le Bureau");
                                });
                                ui.add_space(15.0);

                                ui.horizontal(|ui| {
                                    ui.add_space(60.0);
                                    if ui.add_sized([180.0, 36.0], egui::Button::new(
                                        egui::RichText::new("Lancer la traduction").size(13.0).strong().color(egui::Color32::from_rgb(255, 204, 0))
                                    )).clicked() {
                                        state.current_step = Step::Patching;
                                        start_patching_process(Arc::clone(&self.state));
                                    }
                                    ui.add_space(10.0);
                                    if ui.add_sized([180.0, 36.0], egui::Button::new(
                                        egui::RichText::new("Changer de dossier").size(13.0).color(egui::Color32::WHITE)
                                    )).clicked() {
                                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                            state.manual_path = Some(folder);
                                        }
                                    }
                                    ui.add_space(60.0);
                                });
                            } else {
                                match state.selected_project {
                                    Some(GameProject::UndertaleYellow) => {
                                        ui.label(egui::RichText::new("Le jeu Undertale Yellow n'a pas été détecté automatiquement.").size(13.0).color(egui::Color32::from_rgb(255, 180, 0)));
                                        ui.add_space(6.0);
                                        ui.label("Ce jeu étant distribué via itch.io, veuillez indiquer");
                                        ui.label("son dossier d'installation, ou choisissez l'installation complète.");
                                        ui.add_space(12.0);
                                        ui.horizontal(|ui| {
                                            ui.add_space(40.0);
                                            if ui.add_sized([220.0, 36.0], egui::Button::new(
                                                egui::RichText::new("Sélectionner le dossier").size(13.0).color(egui::Color32::WHITE)
                                            )).clicked() {
                                                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                                    state.manual_path = Some(folder);
                                                }
                                            }
                                            ui.add_space(8.0);
                                            if ui.add_sized([220.0, 36.0], egui::Button::new(
                                                egui::RichText::new("Installer le jeu complet").size(13.0).strong().color(egui::Color32::from_rgb(255, 204, 0))
                                            )).clicked() {
                                                state.auto_install_uty = true;
                                                state.current_step = Step::InstallRepack;
                                            }
                                        });
                                    }
                                    _ => {
                                        ui.label(egui::RichText::new("Recherche automatique échouée dans les répertoires Steam.").color(egui::Color32::from_rgb(255, 51, 51)));
                                        ui.label("Veuillez sélectionner manuellement le dossier contenant le jeu d'origine.");
                                        ui.add_space(15.0);
                                        if ui.add_sized([240.0, 36.0], egui::Button::new(
                                            egui::RichText::new("📁 Sélectionner le dossier du jeu").size(13.0).color(egui::Color32::WHITE)
                                        )).clicked() {
                                            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                                state.manual_path = Some(folder);
                                            }
                                        }
                                    }
                                }
                            }

                            ui.add_space(20.0);
                            if ui.add_sized([120.0, 32.0], egui::Button::new("Retour")).clicked() {
                                state.current_step = Step::ChooseInstallMethod;
                                state.manual_path = None;
                                state.detected_path = None;
                            }
                        });
                    }

                    Step::InstallRepack => {
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("TÉLÉCHARGEMENT DU JEU COMPLET").size(16.0).strong().color(egui::Color32::from_rgb(255, 204, 0)));
                            ui.add_space(12.0);

                            ui.label("Le programme va télécharger et installer le jeu complet configuré en français.");
                            ui.label(egui::RichText::new("(Taille du téléchargement : environ 230 Mo)").size(10.0).color(egui::Color32::GRAY));
                            ui.add_space(12.0);

                            let group_frame = egui::Frame::none()
                                .fill(egui::Color32::from_rgb(32, 20, 42))
                                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 50, 90)))
                                .rounding(4.0)
                                .inner_margin(12.0);

                            group_frame.show(ui, |ui| {
                                ui.label(egui::RichText::new("Dossier d'installation :").strong().size(13.0).color(egui::Color32::WHITE));
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(state.install_dir.to_string_lossy().to_string()).color(egui::Color32::LIGHT_BLUE).size(11.0));
                                    if ui.button("Modifier...").clicked() {
                                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                            state.install_dir = folder.join("UndertaleYellowFR");
                                        }
                                    }
                                });
                                ui.add_space(6.0);
                                ui.checkbox(&mut state.create_shortcut, "Créer un raccourci sur le Bureau");
                            });
                            ui.add_space(15.0);

                            ui.horizontal(|ui| {
                                ui.add_space(60.0);
                                if ui.add_sized([180.0, 36.0], egui::Button::new(
                                    egui::RichText::new("Lancer le téléchargement").size(13.0).strong().color(egui::Color32::from_rgb(255, 204, 0))
                                )).clicked() {
                                    state.current_step = Step::Patching;
                                    start_patching_process(Arc::clone(&self.state));
                                }
                                ui.add_space(10.0);
                                if ui.add_sized([180.0, 36.0], egui::Button::new(
                                    egui::RichText::new("Importer un ZIP local").size(13.0).color(egui::Color32::WHITE)
                                )).clicked() {
                                    if let Some(file) = rfd::FileDialog::new()
                                        .add_filter("Archive ZIP", &["zip"])
                                        .pick_file() 
                                    {
                                        state.manual_repack_path = Some(file);
                                        state.current_step = Step::Patching;
                                        start_patching_process(Arc::clone(&self.state));
                                    }
                                }
                                ui.add_space(60.0);
                            });

                            ui.add_space(20.0);
                            if ui.add_sized([120.0, 32.0], egui::Button::new("Retour")).clicked() {
                                state.current_step = Step::ChooseInstallMethod;
                            }
                        });
                    }
                    
                    Step::Patching => {
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("Mise en place de la traduction en cours...").size(16.0).strong().color(egui::Color32::WHITE));
                            ui.add_space(20.0);
                            
                            ui.label(egui::RichText::new(&state.status_message).size(13.0).color(egui::Color32::from_rgb(200, 200, 200)));
                            ui.add_space(15.0);
                            
                            draw_custom_progress_bar(ui, state.progress);
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new(format!("{:.0}%", state.progress * 100.0)).size(15.0).strong().color(egui::Color32::from_rgb(255, 204, 0)));
                            
                            if !state.download_speed.is_empty() {
                                ui.add_space(8.0);
                                ui.label(egui::RichText::new(&state.download_speed).size(12.0).color(egui::Color32::GRAY));
                            }
                        });
                    }
                    
                    Step::Success => {
                        ui.vertical_centered(|ui| {
                            ui.heading(egui::RichText::new("TRADUCTION APPLIQUÉE AVEC SUCCÈS").color(egui::Color32::from_rgb(0, 220, 100)).strong().size(18.0));
                            ui.add_space(15.0);
                            
                            ui.label("Félicitations ! Votre jeu est désormais entièrement traduit en français.");
                            ui.label(egui::RichText::new("Une sauvegarde de sécurité du fichier original a été créée.").size(10.0).color(egui::Color32::GRAY));
                            ui.add_space(15.0);

                            if let Some(game_dir) = &state.final_game_dir {
                                let group_frame = egui::Frame::none()
                                    .fill(egui::Color32::from_rgb(32, 20, 42))
                                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 50, 90)))
                                    .rounding(4.0)
                                    .inner_margin(12.0);

                                group_frame.show(ui, |ui| {
                                    ui.label(egui::RichText::new("Dossier du jeu :").strong().size(12.0).color(egui::Color32::WHITE));
                                    ui.label(egui::RichText::new(game_dir.to_string_lossy().to_string()).color(egui::Color32::from_rgb(0, 180, 255)).size(11.0));
                                });
                                ui.add_space(15.0);
                            }

                            if let (Some(project), Some(game_dir)) = (state.selected_project, &state.final_game_dir) {
                                let btn_launch = ui.add_sized([300.0, 44.0], egui::Button::new(
                                    egui::RichText::new("Lancer le jeu maintenant").size(14.0).strong().color(egui::Color32::from_rgb(255, 204, 0))
                                ));
                                if btn_launch.clicked() {
                                    launch_game(project, game_dir, state.final_is_unx);
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                                ui.add_space(15.0);
                            }
                            
                            ui.horizontal(|ui| {
                                ui.add_space(60.0);
                                if ui.add_sized([180.0, 32.0], egui::Button::new("Accueil")).clicked() {
                                    state.current_step = Step::MainSelection;
                                    state.selected_project = None;
                                    state.detected_path = None;
                                    state.manual_path = None;
                                    state.progress = 0.0;
                                    state.auto_install_uty = false;
                                }
                                ui.add_space(10.0);
                                if ui.add_sized([180.0, 32.0], egui::Button::new("Quitter")).clicked() {
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                                ui.add_space(60.0);
                            });
                        });
                    }
                    
                    Step::Error => {
                        ui.vertical_centered(|ui| {
                            ui.heading(egui::RichText::new("UNE ERREUR EST SURVENUE").color(egui::Color32::from_rgb(255, 51, 51)).strong().size(18.0));
                            ui.add_space(15.0);
                            
                            ui.label(&state.error_message);
                            ui.add_space(15.0);

                            if state.error_message.contains("repack") || state.error_message.contains("zip") || state.error_message.contains("Connexion") || state.error_message.contains("403") {
                                ui.label("Vous pouvez sélectionner manuellement l'archive de jeu contenant la traduction :");
                                if ui.add_sized([240.0, 36.0], egui::Button::new("Sélectionner l'archive locale")).clicked() {
                                    if let Some(file) = rfd::FileDialog::new()
                                        .add_filter("Archive Zip", &["zip"])
                                        .pick_file() 
                                    {
                                        state.manual_repack_path = Some(file);
                                        state.current_step = Step::Patching;
                                        start_patching_process(Arc::clone(&self.state));
                                    }
                                }
                                ui.add_space(10.0);
                            }
                            
                            ui.horizontal(|ui| {
                                ui.add_space(60.0);
                                if ui.add_sized([180.0, 32.0], egui::Button::new("Réessayer")).clicked() {
                                    state.current_step = Step::DetectGame;
                                    state.progress = 0.0;
                                    state.error_message.clear();
                                }
                                ui.add_space(10.0);
                                if ui.add_sized([180.0, 32.0], egui::Button::new("Accueil")).clicked() {
                                    state.current_step = Step::MainSelection;
                                    state.selected_project = None;
                                    state.detected_path = None;
                                    state.manual_path = None;
                                    state.progress = 0.0;
                                    state.error_message.clear();
                                    state.auto_install_uty = false;
                                }
                                ui.add_space(60.0);
                            });
                        });
                    }
                }
                
                // --- Barre de support Discord & Version (Alignement soigné) ---
                ui.add_space(15.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Zénith Patcher v1.0.0").size(10.0).color(egui::Color32::from_rgb(120, 120, 120)));
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(10.0);
                        
                        let text = egui::RichText::new("Aide & Discord").size(11.0).color(egui::Color32::from_rgb(180, 180, 180));
                        let response = ui.add(egui::Button::new(text).frame(false));
                        if response.clicked() {
                            let _ = webbrowser::open(DISCORD_URL);
                        }
                        
                        if let Some(discord_tex) = &self.tex_discord {
                            ui.image((discord_tex.id(), egui::vec2(14.0, 14.0)));
                        }
                    });
                });
            });
        });
    }
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
        "C:\\Program Files (x86)\\Steam\\steamapps\\common".to_string(),
        "C:\\Program Files\\Steam\\steamapps\\common".to_string(),
    ];

    match game_project {
        GameProject::UndertaleYellow => {
            // UTY n'est pas sur Steam chercher dans des emplacements locaux
            let uty_local_paths = vec![
                // Dossiers itch.io typiques
                format!("{}/UndertaleYellowFR", home),
                format!("{}/Documents/UndertaleYellow", home),
                format!("{}/Games/UndertaleYellow", home),
                format!("{}/Downloads/Undertale Yellow v1_1PatchFr213", home),
                // Chemins Windows avec Wine/Proton
                format!("{}/Games/Undertale Yellow", home),
            ];
            for path in &uty_local_paths {
                let pb = PathBuf::from(path);
                if pb.exists() && (pb.join("data.win").exists() || pb.join("assets").join("game.unx").exists()) {
                    state.detected_path = Some(pb);
                    return;
                }
            }
            // Pas trouvé — UTY n'est pas dans Steam, l'utilisateur doit utiliser l'option repack
            state.detected_path = None;
        }
        GameProject::RedAndYellow => {
            // R&Y s'installe depuis Undertale (Steam) — chercher vanilla Undertale
            let folder_names = vec!["Undertale", "undertale", "UNDERTALE"];
            for base in &steam_paths {
                for folder in &folder_names {
                    let path = PathBuf::from(base).join(folder);
                    let has_win = path.join("data.win").exists();
                    let has_unx = path.join("assets").join("game.unx").exists();
                    if path.exists() && (has_win || has_unx) {
                        state.detected_path = Some(path);
                        return;
                    }
                }
            }
        }
    }
}

// Recherche d'un repack zip local
fn find_local_repack(manual_repack: Option<PathBuf>) -> Option<PathBuf> {
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
        PathBuf::from("/mnt/c/Users/cronos/Downloads"),
    ];

    // Noms de fichiers possibles pour le repack UTY
    let possible_filenames = vec![
        // Nom exact du repack officiel
        "Undertale Yellow v1_1PatchFr.zip",
        "Undertale Yellow v1_1PatchFr213.zip",
        // Noms alternatifs courants
        "undertale-yellow-repack.zip",
        "repack.zip",
        "undertale-yellow-v1.1PatchFr210.zip",
        "undertale_yellow_repack.zip",
        "undertale-yellow.zip",
        "UndertaleYellow.zip",
    ];

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

// Récupère l'URL de téléchargement du repack UTY depuis le versions.json GitHub
fn get_github_repack_url() -> Result<String, String> {
    let response = minreq::get(VERSIONS_URL)
        .with_header("User-Agent", "zenith-patcher/1.0")
        .with_timeout(15)
        .send()
        .map_err(|e| format!("Impossible de joindre GitHub : {}", e))?;

    if response.status_code != 200 {
        return Err(format!("GitHub a répondu {} lors de la récupération du versions.json", response.status_code));
    }

    let body = response.as_str().map_err(|e| e.to_string())?;

    // Extraction simple de repack_url depuis le JSON
    if let Some(pos) = body.find("\"repack_url\":") {
        let rest = &body[pos + "\"repack_url\":".len()..];
        let rest = rest.trim_start();
        if rest.starts_with('"') {
            let inner = &rest[1..];
            if let Some(end) = inner.find('"') {
                return Ok(inner[..end].to_string());
            }
        }
    }

    Err("Champ repack_url introuvable dans le versions.json GitHub".to_string())
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

            if let Some(local_zip) = find_local_repack(manual_repack) {
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
                
                if create_shortcut {
                    let _ = try_create_shortcut(project, &game_path, false);
                }
                
                state.final_game_dir = Some(game_path.clone());
                state.final_is_unx = false;
                state.current_step = Step::Success;
                state.progress = 1.0;
                state.is_patching = false;
                return;
            }

            {
                let mut state = state_mutex.lock().unwrap();
                state.status_message = "Récupération du lien de téléchargement depuis GitHub...".to_string();
            }

            let repack_url = match get_github_repack_url() {
                Ok(url) => url,
                Err(e) => {
                    let mut state = state_mutex.lock().unwrap();
                    state.current_step = Step::Error;
                    state.error_message = format!(
                        "{}\n\nVérifiez votre connexion internet ou importez le zip localement.",
                        e
                    );
                    state.is_patching = false;
                    return;
                }
            };

            let repack_zip = temp_dir.path().join("repack.zip");

            {
                let mut state = state_mutex.lock().unwrap();
                state.status_message = "Téléchargement du repack UTY depuis GitHub...".to_string();
            }

            match download_file(&repack_url, &repack_zip, &state_mutex, 0.1, 0.75) {
                Ok(_) => {
                    let mut state = state_mutex.lock().unwrap();
                    state.status_message = "Extraction des fichiers du repack...".to_string();
                    state.progress = 0.8;

                    fs::create_dir_all(&game_path).unwrap();
                    if let Err(e) = extract_zip(&repack_zip, &game_path) {
                        state.current_step = Step::Error;
                        state.error_message = format!("Erreur d'extraction :\n{}", e);
                        state.is_patching = false;
                        return;
                    }

                    // Appliquer ensuite le patch FR sur le data.win extrait
                    state.status_message = "Application du patch de traduction FR...".to_string();
                    state.progress = 0.9;
                    drop(state); // libérer le verrou avant l'opération longue

                    // Trouver le data.win dans le dossier extrait
                    let _extracted_data = game_path
                        .read_dir()
                        .ok()
                        .and_then(|mut d| d.find_map(|e| {
                            let e = e.ok()?;
                            let sub = e.path().join("data.win");
                            if sub.exists() { Some(sub) } else { None }
                        }))
                        .unwrap_or_else(|| game_path.join("data.win"));

                    let mut final_path = game_path.clone();
                    if game_path.join("Undertale Yellow v1_1PatchFr213").exists() {
                        final_path = game_path.join("Undertale Yellow v1_1PatchFr213");
                    }

                    if create_shortcut {
                        let _ = try_create_shortcut(project, &final_path, false);
                    }

                    let mut state = state_mutex.lock().unwrap();
                    state.final_game_dir = Some(final_path);
                    state.final_is_unx = false;
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
                        PathBuf::from("./patches/ry-fr-v2.2.0.xdelta"),
                        PathBuf::from("./ry-fr-v2.2.0.xdelta"),
                        PathBuf::from("ry-fr-v2.2.0.xdelta"),
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
                    {
                        let mut state = state_mutex.lock().unwrap();
                        state.status_message = "Téléchargement du moteur de jeu compatible...".to_string();
                    }
                    let runner_url = "https://github.com/redyellowpatchfr-a11y/patcher/releases/download/ry-fr-v2.2.0/runner";
                    let runner_path = game_path.join("runner");
                    match download_file(runner_url, &runner_path, &state_mutex, 0.92, 0.98) {
                        Ok(_) => {
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
                        Err(e) => {
                            let mut state = state_mutex.lock().unwrap();
                            state.current_step = Step::Error;
                            state.error_message = format!("Échec du téléchargement du moteur compatible :\n{}", e);
                            state.is_patching = false;
                            return;
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
            if is_unx {
                let mut launched_via_steam = false;
                if let Ok(_child) = Command::new("xdg-open")
                    .arg("steam://run/391540")
                    .spawn()
                {
                    launched_via_steam = true;
                }
                if !launched_via_steam {
                    let run_sh = game_dir.join("run.sh");
                    if run_sh.exists() {
                        Command::new("sh")
                            .arg(&run_sh)
                            .current_dir(game_dir)
                            .spawn()
                    } else {
                        Command::new(game_dir.join("runner"))
                            .current_dir(game_dir)
                            .spawn()
                    }
                } else {
                    Ok(std::process::Command::new("true").spawn().unwrap()) // dummy Ok matching spawn's result type
                }
            } else {
                if cfg!(windows) {
                    Command::new(game_dir.join("UNDERTALE.exe"))
                        .current_dir(game_dir)
                        .spawn()
                } else {
                    Command::new("wine")
                        .arg(game_dir.join("UNDERTALE.exe"))
                        .current_dir(game_dir)
                        .spawn()
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
        ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(2.0, egui::Color32::WHITE));
        
        // Rectangle interne
        let inner_rect = rect.shrink(3.0);
        
        // Fond rouge sombre (style HP d'Undertale)
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
        let app_dir = PathBuf::from(home).join(".local/share/applications");
        let icon_dir = PathBuf::from(&app_dir).parent().unwrap().join("icons/hicolor/64x64/apps");
        
        let _ = fs::create_dir_all(&app_dir);
        let _ = fs::create_dir_all(&icon_dir);
        
        let icon_path = icon_dir.join("zenith-patcher.png");
        let _ = fs::write(&icon_path, APP_ICON_BYTES);
        
        if let Ok(exe_path) = std::env::current_exe() {
            let desktop_path = app_dir.join("zenith-patcher.desktop");
            let desktop_content = format!(
                "[Desktop Entry]\n\
                Type=Application\n\
                Name=Zenith Patcher\n\
                Comment=Patcher de traduction pour Undertale Yellow et Red and Yellow\n\
                Exec={}\n\
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

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([700.0, 440.0]) // Ratio d'aspect panoramique optimal
            .with_resizable(true)
            .with_maximize_button(true)
            .with_decorations(false) // Supprime le cadre Windows d'origine (blanc)
            .with_icon(icon.unwrap_or_default())
            .with_title("Zenith Patcher"), // Titre standard pour GNOME / barre des tâches
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
            supported_backends: eframe::wgpu::Backends::all(),
            ..Default::default()
        },
        ..Default::default()
    };
    
    let result = eframe::run_native(
        "zenith-patcher", // app_id pour correspondre au WM_CLASS de GNOME
        options,
        Box::new(|cc| Ok(Box::new(PatcherApp::new(cc)))),
    );

    if let Err(ref e) = result {
        if let Ok(mut exe_dir) = std::env::current_exe() {
            exe_dir.pop();
            let _ = std::fs::write(exe_dir.join("zenith_patcher_error.txt"), format!("EFRAME ERROR: {:?}", e));
        } else {
            let _ = std::fs::write("zenith_patcher_error.txt", format!("EFRAME ERROR: {:?}", e));
        }
    }

    result
}


