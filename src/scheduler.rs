use rand::seq::SliceRandom;
use std::{
    sync::{Arc, RwLock, RwLockReadGuard},
    vec, collections::HashMap, rc::Rc,
};

use crate::glicko2::algo::Glicko2;

#[derive(Clone, Debug)]
pub struct Item {
    name: String,
    location: String,
    description: String,
}

#[derive(Debug, Clone)]
pub struct MatchPair {
    match_pair_id: String,
    i1: Item,
    i2: Item,
    visited: i32,
}

pub struct Judge {
    id: String,
    name: String,
}

#[derive(Copy, Clone, PartialEq)]
pub enum States {
    NONE,
    INIT,
    CONTINUOUS,
    END,
}

pub struct SchedulerState {
    current_state: Arc<RwLock<States>>,
    judges: Arc<RwLock<Vec<Judge>>>,
    items: Arc<RwLock<Vec<Item>>>,
    matches: Arc<RwLock<HashMap<String, Rc<MatchPair>>>>
}

fn create_initial_matches(competitors: &[Item], n: usize) -> Vec<MatchPair> {
    let mut matches: Vec<MatchPair> = vec![];
    let rng = &mut rand::thread_rng();
    for _ in 0..n {
        let choices: Vec<Item> = competitors.choose_multiple(rng, 2).cloned().collect();
        matches.push(MatchPair {
            match_pair_id: uuid::Uuid::new_v4().to_string(),
            i1: choices[0].clone(),
            i2: choices[1].clone(),
            visited: 0,
        });
    }
    matches
}

impl SchedulerState {
    pub fn new() -> SchedulerState {
        let current_state = Arc::from(RwLock::from(States::NONE));
        let judges = Arc::from(RwLock::from(vec![]));
        let items = Arc::from(RwLock::from(vec![]));
        let matches = Arc::from(RwLock::from(HashMap::new()));
        SchedulerState { current_state, judges, items, matches }
    }

    pub fn get_state(&self) -> States {
        let state = self.current_state.read().unwrap();
        *state
    }

    pub fn seed_start(&mut self, n: usize) -> bool {
        if self.get_state() != States::NONE {
            return false;
        }

        let items = self.items.read().unwrap();
        let mut matches = self.matches.write().unwrap();
        let starter_matches = create_initial_matches(&items, n);
        for m in starter_matches.iter() {
            matches.insert(m.match_pair_id.clone(), m.clone().into());
        }
        let mut old_state = self.current_state.write().unwrap();
        *old_state = States::INIT;

        return true;
    }

    pub fn add_items(&mut self, new_items: &mut Vec<Item>) {
        let mut items = self.items.write().unwrap();
        items.append(new_items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_start() {
        let c1 = Item {
            name: "Project 1".to_owned(),
            location: "a1".to_owned(),
            description: "cool project".to_owned(),
        };

        let c2 = Item {
            name: "Project 2".to_owned(),
            location: "a1".to_owned(),
            description: "cool project".to_owned(),
        };

        let c3 = Item {
            name: "Project 3".to_owned(),
            location: "a1".to_owned(),
            description: "cool project".to_owned(),
        };

        let mut arr = vec![c1, c2, c3];

        let mut scheduler_state = SchedulerState::new();
        scheduler_state.add_items(&mut arr);
        let result = scheduler_state.seed_start(10);
        let matches = scheduler_state.matches.read().unwrap();

        assert_eq!(matches.len(), 10);
        assert_eq!(result, true);

    }

    #[test]
    fn test_add_items() {
        let c1 = Item {
            name: "Project 1".to_owned(),
            location: "a1".to_owned(),
            description: "cool project".to_owned(),
        };

        let c2 = Item {
            name: "Project 2".to_owned(),
            location: "a1".to_owned(),
            description: "cool project".to_owned(),
        };

        let c3 = Item {
            name: "Project 3".to_owned(),
            location: "a1".to_owned(),
            description: "cool project".to_owned(),
        };

        let mut arr = vec![c1, c2, c3];

        let mut scheduler_state = SchedulerState::new();
        scheduler_state.add_items(&mut arr);
        let items = scheduler_state.items.read().unwrap();

        assert_eq!(items.len(), 3);

    }

    #[test]
    fn test_create_initial_matches() {
        let c1 = Item {
            name: "Project 1".to_owned(),
            location: "a1".to_owned(),
            description: "cool project".to_owned(),
        };

        let c2 = Item {
            name: "Project 2".to_owned(),
            location: "a1".to_owned(),
            description: "cool project".to_owned(),
        };

        let c3 = Item {
            name: "Project 3".to_owned(),
            location: "a1".to_owned(),
            description: "cool project".to_owned(),
        };

        let arr = vec![c1, c2, c3];

        let matches = create_initial_matches(&arr, 5);

        assert_eq!(matches.len(), 5);
    }
}
