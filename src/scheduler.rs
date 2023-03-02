use priority_queue::DoublePriorityQueue;
use rand::seq::SliceRandom;
use std::{
    collections::HashMap,
    error::Error,
    fmt,
    sync::{Arc, RwLock},
    vec,
};

#[derive(Debug)]
pub struct SchedulerError {
    details: String,
}

impl SchedulerError {
    fn new(msg: &str) -> SchedulerError {
        SchedulerError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for SchedulerError {
    fn description(&self) -> &str {
        &self.details
    }
}

#[derive(Clone, Debug)]
pub struct Item {
    pub name: String,
    pub location: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct MatchPair {
    match_pair_id: String,
    i1: Item,
    i2: Item,
    visited: i32,
    winner: MatchWinner,
    judge_id: Option<String>,
}

pub struct Judge {
    id: String,
    name: String,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MatchWinner {
    A,
    B,
    NA,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum States {
    None,
    Init,
    Continuous,
    End,
}

pub struct SchedulerState {
    current_state: Arc<RwLock<States>>,
    judges: Arc<RwLock<Vec<Judge>>>,
    items: Arc<RwLock<Vec<Item>>>,
    matches: Arc<RwLock<HashMap<String, Arc<RwLock<MatchPair>>>>>,
    mq: Arc<RwLock<DoublePriorityQueue<String, i32>>>,
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
            winner: MatchWinner::NA,
            judge_id: None,
        });
    }
    matches
}

impl Judge {
    pub fn new(name: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
        }
    }
}

impl SchedulerState {
    pub fn new() -> SchedulerState {
        let current_state = Arc::from(RwLock::from(States::None));
        let judges = Arc::from(RwLock::from(vec![]));
        let items = Arc::from(RwLock::from(vec![]));
        let matches = Arc::from(RwLock::from(HashMap::new()));
        let mq = Arc::from(RwLock::from(DoublePriorityQueue::new()));
        SchedulerState {
            current_state,
            judges,
            items,
            matches,
            mq,
        }
    }

    pub fn get_state(&self) -> States {
        let guard = self.current_state.clone();
        let state = guard.read().unwrap();
        *state
    }

    pub fn get_judges(&self) -> Arc<RwLock<Vec<Judge>>> {
        
        self.judges.clone()
    }

    pub fn get_matches(&self) -> Arc<RwLock<HashMap<String, Arc<RwLock<MatchPair>>>>> {
        
        self.matches.clone()
    }

    pub fn seed_start(&self, n: usize) -> bool {
        if self.get_state() != States::None {
            return false;
        }

        let binding = self.items.clone();
        let items = binding.read().unwrap();

        let matches_binding = self.matches.clone();
        let mut matches = matches_binding.write().unwrap();

        let pq_binding = self.mq.clone();
        let mut pq = pq_binding.write().unwrap();

        let mut starter_matches = create_initial_matches(&items, n);
        for m in starter_matches.drain(..) {
            pq.push(m.match_pair_id.clone(), m.visited);
            matches.insert(m.match_pair_id.clone(), Arc::from(RwLock::from(m)));
        }

        let state_binding = self.current_state.clone();
        let mut old_state = state_binding.write().unwrap();
        *old_state = States::Init;

        true
    }

    pub fn add_items(&self, new_items: &mut Vec<Item>) {
        let mut items = self.items.write().unwrap();
        items.append(new_items);
    }

    pub fn add_judges(&self, new_judges: &mut Vec<Judge>) {
        let mut judges = self.judges.write().unwrap();
        judges.append(new_judges);
    }

    fn find_next_match(&self) -> Result<Arc<RwLock<MatchPair>>, SchedulerError> {
        let state = self.get_state();
        match state {
            States::None => Err(SchedulerError::new(
                "Cannot get next match while in NONE state",
            )),
            States::Init => {
                let q = self.mq.write().unwrap();
                let hm = self.get_matches();
                let matches = hm.write().unwrap();
                if let Some(best) = q.peek_min() {
                    let key = best.0;
                    if let Some(val) = matches.get(key) {
                        Ok(val.clone())
                    } else {
                        Err(SchedulerError::new("Match key not found"))
                    }
                } else {
                    Err(SchedulerError::new("Could not peek queue"))
                }
            }
            States::Continuous => todo!(),
            States::End => Err(SchedulerError::new(
                "Cannot get next match while in END state",
            )),
        }
    }

    pub fn give_judge_next_match(
        &self,
        judge: &Judge,
    ) -> Result<Arc<RwLock<MatchPair>>, SchedulerError> {
        let nm = self.find_next_match();
        match nm {
            Ok(m_lock) => {
                let mut m = m_lock.write().unwrap();
                let m_id = &m.match_pair_id;
                let mut q = self.mq.write().unwrap();
                q.change_priority_by(m_id, |i| *i += 1);
                m.judge_id = Some(judge.id.clone());
                m.visited += 1;
                Ok(m_lock.clone())
            }
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    #[test]
    fn test_give_judge_next_match() {
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

        let mut arr = vec![c1, c2];
        let scheduler_state = Arc::from(SchedulerState::new());
        scheduler_state.add_items(&mut arr);
        scheduler_state.seed_start(1);

        let j1 = Judge::new("J1".to_owned());
        let mut jv = vec![j1];
        scheduler_state.add_judges(&mut jv);

        let binding = scheduler_state.get_judges();
        let v = binding.read().unwrap();

        let actual_j = v.get(0).unwrap();
        let next_match = scheduler_state.give_judge_next_match(actual_j).unwrap();
        let mp = next_match.read().unwrap();

        let id1 = mp.judge_id.clone();
        assert_eq!(id1, Some(actual_j.id.clone()))
    }

    #[test]
    fn test_seed_start_thread() {
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

        let scheduler_state = Arc::from(SchedulerState::new());
        scheduler_state.add_items(&mut arr);
        let ss = Arc::clone(&scheduler_state);

        let handle = thread::spawn(move || {
            let result = ss.seed_start(10);
            assert!(result);
        });
        handle.join().unwrap();
        assert_eq!(scheduler_state.get_state(), States::Init);
    }

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

        let scheduler_state = SchedulerState::new();
        scheduler_state.add_items(&mut arr);
        let result = scheduler_state.seed_start(10);
        let matches = scheduler_state.matches.read().unwrap();

        assert_eq!(matches.len(), 10);
        assert!(result);
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

        let scheduler_state = SchedulerState::new();
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

    #[test]
    fn test_add_judges() {
        let j1 = Judge::new("J1".to_owned());
        let j2 = Judge::new("J2".to_owned());
        let j3 = Judge::new("J3".to_owned());

        let scheduler_state = SchedulerState::new();
        let mut jv = vec![j1, j2, j3];
        scheduler_state.add_judges(&mut jv);

        let binding = scheduler_state.get_judges();
        let judges = binding.read().unwrap();
        assert_eq!(judges.len(), 3);
    }
}
