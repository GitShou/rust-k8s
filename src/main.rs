use rand::Rng; // 乱数用

struct Character {
    name: String,
    hp: i32,
    atk: i32,
    is_live: bool,
}

impl Character {
    // コンストラクタ相当
    fn new(name: &str) -> Self {
        let mut rng = rand::thread_rng();

        let hp = rng.gen_range(50..=100);
        let atk = rng.gen_range(20..=40);

        let me = Self {
            name: name.to_string(),
            hp,
            atk,
            is_live: true,
        };

        // println!("{}: hp={}, atk={}", me.name, me.hp, me.atk);

        me
    }

    fn damage(&mut self, amount: i32) {
        // ダメージを受ける
        self.hp -= amount;

        // HPが0以下になったら死亡判定
        if self.hp <= 0 {
            self.is_live = false;
        }
    }

    fn attack(&self, target: &mut Character) {
        // すでに死んでいるなら何もしない
        if !self.is_live {
            return;
        }

        println!(
            "{} が {} に {} ダメージ与えた！",
            self.name, target.name, self.atk
        );

        target.damage(self.atk);
    }
}

fn main() {
    let mut rng = rand::thread_rng();
    let count = rng.gen_range(1000..=10000);

    let mut chars: Vec<Character> = (0..count)
        .map(|_| {
            let name = random_name(5);
            Character::new(&name)
        })
        .collect();
    println!("{} 体のキャラクターが生成されました！", chars.len());

    // ここからバトルループを作っていく
    let mut rng = rand::thread_rng();

    for turn in 0.. {
        println!("--- {} ターン目 ---", turn + 1);
        {
            let i = rng.gen_range(0..chars.len());
            let mut j = rng.gen_range(0..chars.len());
            while j == i {
                j = rng.gen_range(0..chars.len());
            }

            let (attacker, defender) = two_mut(&mut chars, i, j);
            attacker.attack(defender);
        }

        let dead_names: Vec<String> = chars
            .iter()
            .filter(|c| c.hp <= 0)
            .map(|c| c.name.clone())
            .collect();

        // 死んだキャラを Vec から削除
        chars.retain(|c| c.hp > 0);
        for name in &dead_names {
            println!("{} が倒れた！", name);
        }

        // 生き残りが1人以下なら終了
        if chars.len() <= 1 {
            if let Some(winner) = chars.first() {
                println!("最後の生き残りは {} です！", winner.name);
            } else {
                println!("全滅しました…");
            }
            break;
        }
    }
}

fn two_mut<T>(slice: &mut [T], i: usize, j: usize) -> (&mut T, &mut T) {
    assert!(i != j);

    if i < j {
        // [0 .. j) と [j .. end) に分割
        let (first, rest) = slice.split_at_mut(j);
        let attacker = &mut first[i]; // i 番目
        let defender = &mut rest[0]; // j 番目
        (attacker, defender)
    } else {
        // i > j の場合：同じことを j / i でやる
        let (first, rest) = slice.split_at_mut(i);
        let attacker = &mut rest[0]; // i 番目
        let defender = &mut first[j]; // j 番目
        (attacker, defender)
    }
}

fn random_name(len: usize) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    (0..len)
        .map(|_| {
            let c = rng.gen_range(b'A'..=b'Z'); // A〜Z
            c as char
        })
        .collect()
}
