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




// use rand::Rng; // 乱数用

// struct Character {
//     name: String,
//     hp: i32,
//     atk: i32,
//     is_live: bool,
// }

// impl Character {
//     // コンストラクタ相当
//     fn new(name: &str) -> Self {
//         let mut rng = rand::thread_rng();

//         let hp = rng.gen_range(50..=100);
//         let atk = rng.gen_range(20..=40);

//         let me = Self {
//             name: name.to_string(),
//             hp,
//             atk,
//             is_live: true,
//         };

//         // println!("{}: hp={}, atk={}", me.name, me.hp, me.atk);

//         me
//     }

//     fn damage(&mut self, amount: i32) {
//         // ダメージを受ける
//         self.hp -= amount;

//         // HPが0以下になったら死亡判定
//         if self.hp <= 0 {
//             self.is_live = false;
//         }
//     }

//     fn attack(&self, target: &mut Character) {
//         // すでに死んでいるなら何もしない
//         if !self.is_live {
//             return;
//         }

//         println!(
//             "{} が {} に {} ダメージ与えた！",
//             self.name, target.name, self.atk
//         );

//         target.damage(self.atk);
//     }
// }

// fn main() {
//     let mut rng = rand::thread_rng();
//     let count = rng.gen_range(1000..=10000);

//     let mut chars: Vec<Character> = (0..count)
//         .map(|_| {
//             let name = random_name(5);
//             Character::new(&name)
//         })
//         .collect();
//     println!("{} 体のキャラクターが生成されました！", chars.len());

//     // ここからバトルループを作っていく
//     let mut rng = rand::thread_rng();

//     for turn in 0.. {
//         println!("--- {} ターン目 ---", turn + 1);
//         {
//             let i = rng.gen_range(0..chars.len());
//             let mut j = rng.gen_range(0..chars.len());
//             while j == i {
//                 j = rng.gen_range(0..chars.len());
//             }

//             let (attacker, defender) = two_mut(&mut chars, i, j);
//             attacker.attack(defender);
//         }

//         let dead_names: Vec<String> = chars
//             .iter()
//             .filter(|c| c.hp <= 0)
//             .map(|c| c.name.clone())
//             .collect();

//         // 死んだキャラを Vec から削除
//         chars.retain(|c| c.hp > 0);
//         for name in &dead_names {
//             println!("{} が倒れた！", name);
//         }

//         // 生き残りが1人以下なら終了
//         if chars.len() <= 1 {
//             if let Some(winner) = chars.first() {
//                 println!("最後の生き残りは {} です！", winner.name);
//             } else {
//                 println!("全滅しました…");
//             }
//             break;
//         }
//     }
// }

// fn two_mut<T>(slice: &mut [T], i: usize, j: usize) -> (&mut T, &mut T) {
//     assert!(i != j);

//     if i < j {
//         // [0 .. j) と [j .. end) に分割
//         let (first, rest) = slice.split_at_mut(j);
//         let attacker = &mut first[i]; // i 番目
//         let defender = &mut rest[0]; // j 番目
//         (attacker, defender)
//     } else {
//         // i > j の場合：同じことを j / i でやる
//         let (first, rest) = slice.split_at_mut(i);
//         let attacker = &mut rest[0]; // i 番目
//         let defender = &mut first[j]; // j 番目
//         (attacker, defender)
//     }
// }

// fn random_name(len: usize) -> String {
//     use rand::Rng;
//     let mut rng = rand::thread_rng();

//     (0..len)
//         .map(|_| {
//             let c = rng.gen_range(b'A'..=b'Z'); // A〜Z
//             c as char
//         })
//         .collect()
// }
