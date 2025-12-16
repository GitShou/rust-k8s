use axum::{routing::post, extract::State, Json, Router};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, oneshot};
use tokio::time::{sleep_until, Instant};
use std::collections::HashMap;

// ===== リクエスト / レスポンス =====

#[derive(Deserialize)]
struct JoinRequest {
    name: String,
}

#[derive(Serialize)]
struct JoinResponse {
    name: String,
    rank: usize,
    final_hp: i32,
    is_winner: bool,
}

#[derive(Serialize, Clone)]
struct BattleResult {
    name: String,
    rank: usize,
    final_hp: i32,
    is_winner: bool,
}

// ===== マッチング用の構造体 =====

#[derive(Clone)]
struct Character {
    name: String,
    hp: i32,
    atk: i32,
    is_alive: bool,
    is_client: bool,
}

struct PlayerEntry {
    character: Character,
    tx: oneshot::Sender<BattleResult>, // このプレイヤーへの結果送信口
}

struct Lobby {
    players: Vec<PlayerEntry>,
    _deadline: Instant, // 使ってないなら消してもOK
}

struct SharedState {
    lobby: Option<Lobby>, // 今マッチング中のロビー（1つだけ）
}

type Shared = Arc<Mutex<SharedState>>;

fn two_mut<T>(slice: &mut [T], i: usize, j: usize) -> (&mut T, &mut T) {
    assert!(i != j);

    if i < j {
        let (first, rest) = slice.split_at_mut(j);
        let a = &mut first[i];
        let b = &mut rest[0];
        (a, b)
    } else {
        let (first, rest) = slice.split_at_mut(i);
        let a = &mut rest[0];
        let b = &mut first[j];
        (a, b)
    }
}

/// 100人バトルを行い、各キャラクターの BattleResult を返す
fn run_battle(mut chars: Vec<Character>) -> Vec<BattleResult> {
    let mut rng = rand::thread_rng();
    let mut death_order: Vec<usize> = Vec::new();

    loop {
        let alive_indices: Vec<usize> = chars
            .iter()
            .enumerate()
            .filter(|(_, c)| c.is_alive)
            .map(|(i, _)| i)
            .collect();

        if alive_indices.len() <= 1 {
            break;
        }

        let attacker_idx = alive_indices[rng.gen_range(0..alive_indices.len())];
        let mut defender_idx = attacker_idx;
        while defender_idx == attacker_idx {
            defender_idx = alive_indices[rng.gen_range(0..alive_indices.len())];
        }

        let (attacker, defender) = two_mut(&mut chars, attacker_idx, defender_idx);

        if attacker.is_alive && defender.is_alive {
            defender.hp -= attacker.atk;

            if defender.hp <= 0 && defender.is_alive {
                defender.is_alive = false;
                death_order.push(defender_idx);
            }
        }
    }

    // 最後まで生き残ったキャラ（優勝者）も death_order に入れる
    for (i, c) in chars.iter().enumerate() {
        if c.is_alive {
            death_order.push(i);
        }
    }

    // death_order は「死んだ順 + 最後に生存者」
    // ランクは逆順にして 1位から振る
    let total = death_order.len();
    let mut results = Vec::with_capacity(total);

    for (rank_from_zero, &idx) in death_order.iter().rev().enumerate() {
        let c = &chars[idx];
        let rank = rank_from_zero + 1;
        results.push(BattleResult {
            name: c.name.clone(),
            final_hp: c.hp,
            rank,
            is_winner: rank == 1,
        });
    }

    results
}

// ===== /join ハンドラ =====

async fn join_handler(
    State(shared): State<Shared>,
    Json(req): Json<JoinRequest>,
) -> Json<JoinResponse> {
    let (tx, rx) = oneshot::channel::<BattleResult>();

    {
        let mut state = shared.lock().await;

        let character = Character {
            name: req.name.clone(),
            hp: 100,
            atk: 20,
            is_alive: true,
            is_client: true,
        };

        match &mut state.lobby {
            Some(lobby) => {
                lobby.players.push(PlayerEntry { character, tx });
            }
            None => {
                // ロビーが無い -> 1人目の参加者
                let deadline = Instant::now() + Duration::from_secs(10);

                let mut lobby = Lobby {
                    players: Vec::new(),
                    _deadline: deadline,
                };
                lobby.players.push(PlayerEntry { character, tx });

                state.lobby = Some(lobby);

                let shared_clone = shared.clone();
                tokio::spawn(async move {
                    sleep_until(deadline).await;
                    finalize_match(shared_clone).await;
                });
            }
        }
    }

    let result = rx.await.expect("match finalize task dropped");

    Json(JoinResponse {
        name: result.name,
        rank: result.rank,
        final_hp: result.final_hp,
        is_winner: result.is_winner,
    })
}

// ===== マッチ確定処理 =====

async fn finalize_match(shared: Shared) {
    let lobby = {
        let mut state = shared.lock().await;
        state.lobby.take()
    };

    let Some(lobby) = lobby else {
        return;
    };

    let mut rng = rand::thread_rng();

    let mut all_chars: Vec<Character> = lobby
        .players
        .iter()
        .map(|p| p.character.clone())
        .collect();

    while all_chars.len() < 100 {
        let id = all_chars.len();
        all_chars.push(Character {
            name: format!("NPC_{}", id),
            hp: rng.gen_range(80..120),
            atk: rng.gen_range(5..20),
            is_alive: true,
            is_client: false,
        });
    }

    let results = run_battle(all_chars);

    let mut map: HashMap<String, BattleResult> =
        results.into_iter().map(|r| (r.name.clone(), r)).collect();

    for player in lobby.players {
        if let Some(result) = map.remove(&player.character.name) {
            let _ = player.tx.send(result);
        } else {
            let _ = player.tx.send(BattleResult {
                name: player.character.name.clone(),
                rank: 999,
                final_hp: -1,
                is_winner: false,
            });
        }
    }
}

// ===== main =====

#[tokio::main]
async fn main() {
    let shared = Arc::new(Mutex::new(SharedState { lobby: None }));

    let app = Router::new()
        .route("/join", post(join_handler))
        .with_state(shared);

    let addr: SocketAddr = "0.0.0.0:3000".parse().unwrap();
    println!("Server listening on {}", addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}
