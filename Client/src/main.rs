use eframe::egui;
use serde::{Deserialize, Serialize};
use std::sync::mpsc;

#[derive(Debug, Serialize)]
struct JoinRequest {
    name: String,
    hp: i32,
    atk: i32,
}

#[derive(Debug, Deserialize, Clone)]
struct JoinResponse {
    name: String,
    rank: i32,
    final_hp: i32,
    is_winner: bool,
}

#[derive(Debug, Clone)]
enum ClientEvent {
    Started,
    Completed(JoinResponse),
    Failed(String),
}

struct AppState {
    server_url: String,
    player_name: String,

    hp: i32,
    atk: i32,

    status: String,
    waiting: bool,
    last_result: Option<JoinResponse>,

    rx: mpsc::Receiver<ClientEvent>,
    tx: mpsc::Sender<ClientEvent>,
}

impl Default for AppState {
    fn default() -> Self {
        use rand::Rng;

        let (tx, rx) = mpsc::channel();
        let mut rng = rand::thread_rng();

        let hp = rng.gen_range(80..120);
        let atk = rng.gen_range(5..20);

        Self {
            server_url: "http://127.0.0.1:3000".to_string(),
            player_name: "Shogo_A".to_string(),

            hp,
            atk,

            status: "Idle".to_string(),
            waiting: false,
            last_result: None,
            rx,
            tx,
        }
    }
}

impl AppState {
    fn join(&mut self) {
        if self.waiting {
            return;
        }

        let server_url = self.server_url.trim_end_matches('/').to_string();
        let name = self.player_name.trim().to_string();
        if name.is_empty() {
            self.status = "Name is empty".to_string();
            return;
        }

        self.waiting = true;
        self.last_result = None;
        self.status = "Waiting... (POST /join)".to_string();
        
        let hp = self.hp;
        let atk = self.atk;
        let tx = self.tx.clone();

        std::thread::spawn(move || {
            let _ = tx.send(ClientEvent::Started);

            // ここはGUIスレッドを止めないために別スレッドで block してOK
            let client = reqwest::blocking::Client::new();
            let url = format!("{}/join", server_url);

            let req = JoinRequest {
                name,
                hp: hp,
                atk: atk,
            };

            let resp = client.post(url).json(&req).send();

            match resp {
                Ok(r) => {
                    if !r.status().is_success() {
                        let status = r.status();
                        let body = r.text().unwrap_or_default();
                        let _ = tx.send(ClientEvent::Failed(format!("HTTP {}: {}", status, body)));
                        return;
                    }

                    match r.json::<JoinResponse>() {
                        Ok(data) => {
                            let _ = tx.send(ClientEvent::Completed(data));
                        }
                        Err(e) => {
                            let _ =
                                tx.send(ClientEvent::Failed(format!("JSON parse error: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(ClientEvent::Failed(format!("Request error: {}", e)));
                }
            }
        });
    }

    fn pump_events(&mut self) {
        // まとめて捌く（描画ごとに詰まりにくい）
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                ClientEvent::Started => {
                    // 表示更新だけ
                    self.status = "Waiting... (server is matching / battling)".to_string();
                }
                ClientEvent::Completed(res) => {
                    self.waiting = false;
                    self.status = "Done".to_string();
                    self.last_result = Some(res);
                }
                ClientEvent::Failed(msg) => {
                    self.waiting = false;
                    self.status = format!("Error: {}", msg);
                }
            }
        }
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.pump_events();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Battle Client");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("Server URL:");
                ui.text_edit_singleline(&mut self.server_url);
            });

            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.player_name);
            });
            
            ui.separator();
            ui.label("Character Status:");
            ui.monospace(format!("HP  : {}", self.hp));
            ui.monospace(format!("ATK : {}", self.atk));

            ui.add_space(8.0);

            let join_btn = ui.add_enabled(!self.waiting, egui::Button::new("Join"));
            if join_btn.clicked() {
                self.join();
            }

            ui.add_space(12.0);
            ui.label(format!("Status: {}", self.status));

            ui.add_space(12.0);
            ui.separator();
            ui.label("Result:");

            if let Some(r) = &self.last_result {
                ui.monospace(format!("name      : {}", r.name));
                ui.monospace(format!("rank      : {}", r.rank));
                ui.monospace(format!("final_hp  : {}", r.final_hp));
                ui.monospace(format!("is_winner : {}", r.is_winner));
            } else {
                ui.monospace("(no result)");
            }

            ui.add_space(8.0);
            ui.small("Note: /join はレスポンスが返るまで待機します（10秒待機 + バトル時間）。");
        });

        // 待機中はそれなりに再描画（CPUを焼かない程度）
        if self.waiting {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([520.0, 350.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Battle Client",
        options,
        Box::new(|_cc| Ok(Box::new(AppState::default()))),
    )
}
