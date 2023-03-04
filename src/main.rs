use scheduler::{Item, MatchWinner, SchedulerState};
use std::sync::Arc;

use crate::{glicko2::algo::Glicko2, scheduler::Judge};

pub mod glicko2;
mod scheduler;

#[tokio::main]
async fn main() {
    println!("This is very WIP.");
    // test().await
    test2()
}

fn test2() {
    let c1 = Box::new(Item {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Project 1".to_owned(),
        location: "a1".to_owned(),
        description: "cool project".to_owned(),
        score: Box::new(Glicko2::new()),
    });

    let c2 = Box::new(Item {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Project 2".to_owned(),
        location: "a1".to_owned(),
        description: "cool project".to_owned(),
        score: Box::new(Glicko2::new()),
    });

    let arr = vec![c1, c2];
    let scheduler_state = Arc::from(SchedulerState::new());
    scheduler_state.add_items(arr);
    scheduler_state.seed_start(1);

    let j1 = Judge::new("J1".to_owned());
    let j = j1.clone();
    let mut jv = vec![j1];
    scheduler_state.add_judges(&mut jv);

    let match_id = {
        let next_match = scheduler_state.give_judge_next_match(&j).unwrap();
        let x = next_match.read().unwrap().match_pair_id.clone();
        x
    };
    scheduler_state.judge_match(&j, &match_id, MatchWinner::A);

    let mut items = scheduler_state.get_items();
    items.sort_by(|a, b| a.score.mu.total_cmp(&b.score.mu));

    println!("Items: {:#?}", items);
}
