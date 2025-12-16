// 1. use 宣言
use axum::{routing::post, Json, Router};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

// 2. リクエスト / レスポンス型（3.で書いた部分）
#[derive(Deserialize)]
struct ClientCharacterInput {
    name: String,
}

#[derive(Deserialize)]
struct BattleRequest {
    characters: Vec<ClientCharacterInput>,
}

#[derive(Serialize)]
struct ClientCharacterResult {
    name: String,
    rank: usize,
    final_hp: i32,
    is_winner: bool,
}

#[derive(Serialize)]
struct BattleResult {
    total_chars: usize,
    client_results: Vec<ClientCharacterResult>,
}

// 3. サーバ側で使うキャラクター構造体とバトルロジック（4.の部分）
struct Character {
    name: String,
    hp: i32,
    atk: i32,
    is_alive: bool,
    is_client: bool,
}

fn random_character(name_prefix: &str, index: usize) -> Character {
    let mut rng = rand::thread_rng();
    let hp = rng.gen_range(50..=100);
    let atk = rng.gen_range(20..=40);

    Character {
        name: format!("{}{}", name_prefix, index),
        hp,
        atk,
        is_alive: true,
        is_client: false,
    }
}

// 2つのインデックスから同時に &mut を取り出す
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

fn run_battle(mut chars: Vec<Character>) -> (Vec<Character>, Vec<usize>) {
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

    for (i, c) in chars.iter().enumerate() {
        if c.is_alive {
            death_order.push(i);
        }
    }

    (chars, death_order)
}

// 4. ハンドラ + main（5.の部分）
async fn battle_handler(Json(req): Json<BattleRequest>) -> Json<BattleResult> {
    let mut rng = rand::thread_rng();

    let mut chars: Vec<Character> = req
        .characters
        .into_iter()
        .map(|c| {
            let hp = rng.gen_range(50..=100);
            let atk = rng.gen_range(20..=40);
            Character {
                name: c.name,
                hp,
                atk,
                is_alive: true,
                is_client: true,
            }
        })
        .collect();

    let client_count = chars.len();
    let max_chars = 100;

    if chars.len() < max_chars {
        let need = max_chars - chars.len();
        for i in 0..need {
            chars.push(random_character("NPC_", i));
        }
    }

    let total_chars = chars.len();
    let (final_chars, death_order) = run_battle(chars);

    let mut index_to_rank = vec![0usize; total_chars];
    for (pos, idx) in death_order.iter().enumerate() {
        let rank = total_chars - pos;
        index_to_rank[*idx] = rank;
    }

    let mut client_results = Vec::new();
    for (i, c) in final_chars.iter().enumerate().take(client_count) {
        let rank = index_to_rank[i];
        client_results.push(ClientCharacterResult {
            name: c.name.clone(),
            rank,
            final_hp: c.hp,
            is_winner: c.is_alive && rank == 1,
        });
    }

    Json(BattleResult {
        total_chars,
        client_results,
    })
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/battle", post(battle_handler));

    let addr: SocketAddr = "0.0.0.0:3000".parse().unwrap();
    println!("Server listening on {}", addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}