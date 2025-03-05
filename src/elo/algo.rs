pub enum Winner {
    P1,
    P2,
}

pub const K: f64 = 30.0;

pub const INITIAL_ELO: f64 = 1000.0;

fn calc_new_rating(old_rating: f64, expected: f64, k: f64, win: bool) -> f64 {
    if win {
        old_rating + k * (1.0 - expected)
    } else {
        old_rating + k * (0.0 - expected)
    }
}

pub fn calculate(r1: f64, r2: f64, k: f64, winner: Winner) -> (f64, f64) {
    let p1 = 1.0 / (1.0 + ((r1 - r2) / 400.0).powi(10));
    let p2 = 1.0 / (1.0 + ((r2 - r1) / 400.0).powi(10));

    return match winner {
        Winner::P1 => (
            calc_new_rating(r1, p1, k, true),
            calc_new_rating(r2, p2, k, false),
        ),
        Winner::P2 => (
            calc_new_rating(r1, p1, k, false),
            calc_new_rating(r2, p2, k, true),
        ),
    };
}
