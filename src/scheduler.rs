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

#[derive(Clone, Debug, PartialEq)]
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
    NoState,
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
        let mut cc: Vec<&Item> = Vec::from_iter(competitors.iter());
        cc.shuffle(rng);
        if cc.len() % 2 != 0 {
            cc.push(cc.get(0).unwrap());
        }
        for i in 0..cc.len() / 2 {
            let c1 = *cc.get(i).unwrap();
            let c2 = *cc.get(cc.len() - 1 - i).unwrap();
            matches.push(MatchPair {
                match_pair_id: uuid::Uuid::new_v4().to_string(),
                i1: c1.clone(),
                i2: c2.clone(),
                visited: 0,
                winner: MatchWinner::NA,
                judge_id: None,
            });
        }
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
        let current_state = Arc::from(RwLock::from(States::NoState));
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

    fn state_machine_internal_transition(&self) -> States {
        let guard = self.current_state.clone();
        let mut state = guard.write().unwrap();

        match *state {
            States::NoState => States::NoState,
            States::Init => {
                let q = self.mq.read().unwrap();
                let peek = q.peek_min();
                if let Some(p) = peek {
                    if *p.1 < 1 {
                        *state = States::Init;
                        return States::Init;
                    }
                    *state = States::Continuous;
                    States::Continuous
                } else {
                    *state = States::Init;
                    States::Init
                }
            }
            States::Continuous => States::Continuous,
            States::End => States::End,
        }
    }

    pub fn get_matches(&self) -> Arc<RwLock<HashMap<String, Arc<RwLock<MatchPair>>>>> {
        self.matches.clone()
    }

    pub fn seed_start(&self, n: usize) -> bool {
        if self.get_state() != States::NoState {
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
            States::NoState => Err(SchedulerError::new(
                "Cannot get next match while in NONE state",
            )),
            States::Init => self.get_from_queue(0).unwrap(),
            States::Continuous => self.get_continuous_stage(),
            States::End => Err(SchedulerError::new(
                "Cannot get next match while in END state",
            )),
        }
    }

    fn get_from_queue(
        &self,
        min_prio: i32,
    ) -> Option<Result<Arc<RwLock<MatchPair>>, SchedulerError>> {
        let q = self.mq.write().unwrap();
        let hm = self.get_matches();
        let matches = hm.write().unwrap();
        if let Some(best) = q.peek_min() {
            let key = best.0;
            let prio = *best.1;
            if min_prio < prio {
                return None;
            }
            if let Some(val) = matches.get(key) {
                Some(Ok(val.clone()))
            } else {
                Some(Err(SchedulerError::new("Match key not found")))
            }
        } else {
            Some(Err(SchedulerError::new("Could not peek queue")))
        }
    }

    fn get_continuous_stage(&self) -> Result<Arc<RwLock<MatchPair>>, SchedulerError> {
        if let Some(Ok(queue_item)) = self.get_from_queue(1) {
            return Ok(queue_item);
        }

        let rng = &mut rand::thread_rng();
        let items_guard = self.items.read();
        let items = items_guard.unwrap();

        let choices: Vec<Item> = items.choose_multiple(rng, 2).cloned().collect();
        // TODO: keep a sorted array of closest scores and then select best match

        let id = uuid::Uuid::new_v4().to_string();
        let m = MatchPair {
            match_pair_id: id.clone(),
            i1: choices[0].clone(),
            i2: choices[1].clone(),
            visited: 0,
            winner: MatchWinner::NA,
            judge_id: None,
        };

        let as_arc = Arc::from(RwLock::from(m));

        let hm_binding = self.get_matches();
        let mut hm = hm_binding.write().unwrap();
        hm.insert(id, as_arc.clone());
        Ok(as_arc)
    }

    pub fn give_judge_next_match(
        &self,
        judge: &Judge,
    ) -> Result<Arc<RwLock<MatchPair>>, SchedulerError> {
        self.state_machine_internal_transition();
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
    fn test_get_continuous_stage() {
        let c1 = Item {
            name: "Project 1".to_owned(),
            location: "a1".to_owned(),
            description: "cool project".to_owned(),
        };

        let cc1 = c1.clone();

        let c2 = Item {
            name: "Project 2".to_owned(),
            location: "a1".to_owned(),
            description: "cool project".to_owned(),
        };

        let cc2 = c2.clone();

        let mut arr = vec![c1, c2];
        let scheduler_state = Arc::from(SchedulerState::new());
        scheduler_state.add_items(&mut arr);

        let next_match = scheduler_state.get_continuous_stage().unwrap();
        let read = next_match.read().unwrap();

        if read.i1 != cc1 {
            assert_eq!(read.i1, cc2);
            assert_eq!(read.i2, cc1);
        } else {
            assert_eq!(read.i1, cc1);
            assert_eq!(read.i2, cc2);
        }
    }

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
        let result = scheduler_state.seed_start(3);
        let matches = scheduler_state.matches.read().unwrap();

        assert_eq!(matches.len(), 6);
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

        let matches = create_initial_matches(&arr, 3);

        assert_eq!(matches.len(), 6);
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
