use std::sync::{RwLock, Arc};

use crate::glicko2::algo::Glicko2;

struct Item {
    name: String,
    location: String,
    description: String,
    ranking: Glicko2
}

struct MatchPair {
    match_pair_id: String,
    i1: Item,
    i2: Item,
    visited: i32
}

struct Judges {
    id: String,
    name: String
}

#[derive(Copy, Clone)]
enum States {
    NONE,
    INIT,
    CONTINUOUS,
    END
}

struct SchedulerState {
    current_state: Arc<RwLock<States>>,
    judges: Arc<RwLock<Vec<Judges>>>
}

impl SchedulerState {
    fn get_state(self) -> States {
        let state = self.current_state.read().unwrap();
        *state
    }

    fn seed_start(self) {

    }
}