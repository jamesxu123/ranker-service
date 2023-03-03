use dashmap::DashMap;
use priority_queue::DoublePriorityQueue;
use rand::seq::SliceRandom;
use std::{
    collections::HashMap,
    error::Error,
    fmt,
    sync::{Arc, RwLock},
    vec,
};

use crate::glicko2::algo::Glicko2;

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
    pub id: String,
    pub name: String,
    pub location: String,
    pub description: String,
    pub score: Box<Glicko2>,
}

#[derive(Debug, Clone)]
pub struct MatchPair {
    pub match_pair_id: String,
    pub i1: String,
    pub i2: String,
    visit_count: i32,
    winner: Option<MatchWinner>,
    judge_id: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Judge {
    id: String,
    name: String,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MatchWinner {
    A,
    B,
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
    items: Arc<DashMap<String, Box<Item>>>,
    matches: Arc<RwLock<HashMap<String, Arc<RwLock<MatchPair>>>>>,
    mq: Arc<RwLock<DoublePriorityQueue<String, i32>>>,
}

fn create_initial_matches(competitors: &[Box<Item>], n: usize) -> Vec<MatchPair> {
    let mut matches: Vec<MatchPair> = vec![];
    let rng = &mut rand::thread_rng();
    for _ in 0..n {
        let mut cc: Vec<&Box<Item>> = Vec::from_iter(competitors.iter());
        cc.shuffle(rng);
        if cc.len() % 2 != 0 {
            cc.push(cc.get(0).unwrap());
        }
        for i in 0..cc.len() / 2 {
            let c1 = *cc.get(i).unwrap();
            let c2 = *cc.get(cc.len() - 1 - i).unwrap();
            matches.push(MatchPair {
                match_pair_id: uuid::Uuid::new_v4().to_string(),
                i1: c1.id.clone(),
                i2: c2.id.clone(),
                visit_count: 0,
                winner: None,
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

    pub fn log_match_action(&self) {
        println!("match logged");
    }
}

impl SchedulerState {
    pub fn new() -> SchedulerState {
        let current_state = Arc::from(RwLock::from(States::NoState));
        let judges = Arc::from(RwLock::from(vec![]));
        let items = Arc::from(DashMap::new());
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

    pub fn judge_match(&self, judge: &Judge, match_id: &str, winner: MatchWinner) -> bool {
        let m = self.matches.write();
        let matches = m.unwrap();

        if let Some(match_pair_guard) = matches.get(match_id) {
            if let Ok(mut match_pair) = match_pair_guard.write() {
                match_pair.winner = Some(winner);

                judge.log_match_action();
                
                // TODO: this unwrap is _probably_ a bad idea
                let mut s1 = self.items.get_mut(&match_pair.i1).unwrap();
                let mut s2 = self.items.get_mut(&match_pair.i2).unwrap();
                // TODO: does not actually update backing items array...I don't have a good solution here
                return match winner {
                    MatchWinner::A => {
                        let s1_im = s1.score.clone();
                        let s2_im = s2.score.clone();

                        s1.score.process_matches(&vec![&s2_im], &vec![1f64]);
                        s2.score.process_matches(&vec![&s1_im], &vec![0f64]);
                        true
                    }
                    MatchWinner::B => {
                        let s1_im = s1.score.clone();
                        let s2_im = s2.score.clone();

                        s1.score.process_matches(&vec![&s2_im], &vec![0f64]);
                        s2.score.process_matches(&vec![&s1_im], &vec![1f64]);
                        true
                    }
                };
            }
        }
        false
    }

    pub fn get_state(&self) -> States {
        let guard = self.current_state.clone();
        let state = guard.read().unwrap();
        *state
    }

    pub fn get_judges(&self) -> Vec<Judge> {
        let iter = self.judges.read().unwrap();
        let mut v: Vec<Judge> = vec![];
        for i in iter.iter() {
            v.push(i.clone());
        }
        v
    }

    pub fn get_items(&self) -> Vec<Box<Item>> {
        let iter = &self.items;
        let mut v: Vec<Box<Item>> = vec![];
        for i in iter.iter() {
            v.push(i.value().clone());
        }
        v
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

        let matches_binding = self.matches.clone();
        let mut matches = matches_binding.write().unwrap();

        let pq_binding = self.mq.clone();
        let mut pq = pq_binding.write().unwrap();

        let item_vec = self.get_items();

        let mut starter_matches = create_initial_matches(&item_vec, n);
        for m in starter_matches.drain(..) {
            pq.push(m.match_pair_id.clone(), m.visit_count);
            matches.insert(m.match_pair_id.clone(), Arc::from(RwLock::from(m)));
        }

        let state_binding = self.current_state.clone();
        let mut old_state = state_binding.write().unwrap();
        *old_state = States::Init;

        true
    }

    pub fn add_items(&self, new_items: &mut Vec<Box<Item>>) {
        let items = &self.items;
        for item in new_items.into_iter() {
            items.insert(item.id.clone(), item.clone());
        }
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
        let items = self.get_items();

        let choices: Vec<Box<Item>> = items.choose_multiple(rng, 2).cloned().collect();
        // TODO: keep a sorted array of closest scores and then select best match

        let id = uuid::Uuid::new_v4().to_string();
        let m = MatchPair {
            match_pair_id: id.clone(),
            i1: choices[0].id.clone(),
            i2: choices[1].id.clone(),
            visit_count: 0,
            winner: None,
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
                m.visit_count += 1;
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
        let c1 = Box::new(Item {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Project 1".to_owned(),
            location: "a1".to_owned(),
            description: "cool project".to_owned(),
            score: Box::new(Glicko2::new()),
        });

        let cc1 = c1.clone();

        let c2 = Box::new(Item {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Project 2".to_owned(),
            location: "a1".to_owned(),
            description: "cool project".to_owned(),
            score: Box::new(Glicko2::new()),
        });

        let cc2 = c2.clone();

        let mut arr = vec![c1, c2];
        let scheduler_state = Arc::from(SchedulerState::new());
        scheduler_state.add_items(&mut arr);

        let next_match = scheduler_state.get_continuous_stage().unwrap();
        let read = next_match.read().unwrap();

        if *read.i1 != cc1.id {
            assert_eq!(*read.i1, cc2.id);
            assert_eq!(*read.i2, cc1.id);
        } else {
            assert_eq!(*read.i1, cc1.id);
            assert_eq!(*read.i2, cc2.id);
        }
    }

    #[test]
    fn test_give_judge_next_match() {
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

        let mut arr = vec![c1, c2];
        let scheduler_state = Arc::from(SchedulerState::new());
        scheduler_state.add_items(&mut arr);
        scheduler_state.seed_start(1);

        let j1 = Judge::new("J1".to_owned());
        let mut jv = vec![j1];
        scheduler_state.add_judges(&mut jv);

        let v = scheduler_state.get_judges();

        let actual_j = v.get(0).unwrap();
        let next_match = scheduler_state.give_judge_next_match(actual_j).unwrap();
        let mp = next_match.read().unwrap();

        let id1 = mp.judge_id.clone();
        assert_eq!(id1, Some(actual_j.id.clone()))
    }

    #[test]
    fn test_judge_match() {
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

        println!("{:#?}", items);
    }

    #[test]
    fn test_seed_start_thread() {
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

        let c3 = Box::new(Item {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Project 3".to_owned(),
            location: "a1".to_owned(),
            description: "cool project".to_owned(),
            score: Box::new(Glicko2::new()),
        });

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

        let c3 = Box::new(Item {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Project 3".to_owned(),
            location: "a1".to_owned(),
            description: "cool project".to_owned(),
            score: Box::new(Glicko2::new()),
        });

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

        let c3 = Box::new(Item {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Project 3".to_owned(),
            location: "a1".to_owned(),
            description: "cool project".to_owned(),
            score: Box::new(Glicko2::new()),
        });

        let mut arr = vec![c1, c2, c3];

        let scheduler_state = SchedulerState::new();
        scheduler_state.add_items(&mut arr);
        let items = scheduler_state.items;

        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_create_initial_matches() {
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

        let c3 = Box::new(Item {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Project 3".to_owned(),
            location: "a1".to_owned(),
            description: "cool project".to_owned(),
            score: Box::new(Glicko2::new()),
        });

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

        let judges = scheduler_state.get_judges();
        assert_eq!(judges.len(), 3);
    }
}
