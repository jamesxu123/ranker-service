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
    let c1 = Item {
        name: "Project 1".to_owned(),
        location: "a1".to_owned(),
        description: "cool project".to_owned(),
        score: Box::new(Glicko2::new()),
    };

    let c2 = Item {
        name: "Project 2".to_owned(),
        location: "a1".to_owned(),
        description: "cool project".to_owned(),
        score: Box::new(Glicko2::new()),
    };

    let mut arr = vec![c1, c2];
    let scheduler_state = Arc::from(SchedulerState::new());
    scheduler_state.add_items(&mut arr);
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
async fn test() {
    let c1 = Item {
        name: "Project 1".to_owned(),
        location: "a2".to_owned(),
        description: "cool project 1".to_owned(),
        score: Box::new(Glicko2::new()),
    };

    let c2 = Item {
        name: "Project 2".to_owned(),
        location: "a2".to_owned(),
        description: "cool project 2".to_owned(),
        score: Box::new(Glicko2::new()),
    };

    let c3 = Item {
        name: "Project 3".to_owned(),
        location: "a3".to_owned(),
        description: "cool project 3".to_owned(),
        score: Box::new(Glicko2::new()),
    };

    let mut arr = vec![c1, c2, c3];

    let scheduler_state = Arc::from(SchedulerState::new());
    scheduler_state.add_items(&mut arr);
    let ss = Arc::clone(&scheduler_state);

    let handle = tokio::spawn(async move {
        let result = ss.seed_start(10);
        assert!(result);
    });
    handle.await.unwrap();
    let matches = scheduler_state.get_matches();
    println!("Result {:#?}", matches.read().unwrap());

    let j1 = Judge::new("J1".to_owned());
    let mut jv = vec![j1];
    scheduler_state.add_judges(&mut jv);

    let v = scheduler_state.get_judges();

    let next_match = scheduler_state
        .give_judge_next_match(v.get(0).unwrap())
        .unwrap();
    println!("{:#?}", next_match);
}
